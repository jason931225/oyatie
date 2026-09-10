//! The walk, generic in where its graph comes from.
//!
//! The traversal law is written ONCE here. A source supplies only three
//! things: whether a node exists (and its type), its outbound edges, and
//! its inbound edges.
//!
//! That split is the point. The in-memory index and the durable
//! projection store are then two sources over one law, so every merged
//! law test can be run against both and neither can drift from the
//! other. A second copy of this walk would be a second place for the
//! consent gate and the freshness floor to disagree.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::contract::*;
use crate::link::KnowledgeGraphLinkInstance;
use crate::request::{EdgeConsent, KnowledgeGraphQueryRequest};

/// Where a walk reads its graph.
pub(crate) trait GraphSource {
    /// The node, if this tenant holds it. `Err` means the SOURCE
    /// failed — never "absent", which is `Ok(None)`.
    fn node(
        &self,
        tenant_id: &str,
        entity_id: &str,
    ) -> Result<Option<KnowledgeGraphNode>, KnowledgeGraphQueryError>;

    fn outbound(
        &self,
        tenant_id: &str,
        entity_id: &str,
    ) -> Result<Vec<KnowledgeGraphLinkInstance>, KnowledgeGraphQueryError>;

    fn inbound(
        &self,
        tenant_id: &str,
        entity_id: &str,
    ) -> Result<Vec<KnowledgeGraphLinkInstance>, KnowledgeGraphQueryError>;
}

fn passes_edge_filter(link: &KnowledgeGraphLinkInstance, edge_filter: &BTreeSet<&str>) -> bool {
    edge_filter.is_empty() || edge_filter.contains(link.edge_type_id.as_str())
}

fn is_fresh_enough(link: &KnowledgeGraphLinkInstance, floor_epoch_seconds: u64) -> bool {
    link.observed_at_epoch_seconds >= floor_epoch_seconds
}

fn consent_permits(link: &KnowledgeGraphLinkInstance, consent: &EdgeConsent) -> bool {
    consent.permits(link.edge_type_id.as_str())
}

fn neighbour_of(link: &KnowledgeGraphLinkInstance, arrived_from: &str) -> String {
    if link.from_entity_id == arrived_from {
        link.to_entity_id.clone()
    } else {
        link.from_entity_id.clone()
    }
}

fn sorted_page<T: Ord>(emissions: impl Iterator<Item = T>) -> Vec<T> {
    let mut page: Vec<T> = emissions.collect();
    page.sort();
    page
}

fn insert_node<S: GraphSource>(
    source: &S,
    tenant_id: &str,
    entity_id: &str,
    nodes: &mut BTreeMap<String, KnowledgeGraphNode>,
) -> Result<(), KnowledgeGraphQueryError> {
    let node = source.node(tenant_id, entity_id)?.ok_or_else(|| {
        KnowledgeGraphQueryError::DanglingLinkEndpoint {
            entity_id: entity_id.to_string(),
        }
    })?;
    nodes.insert(entity_id.to_string(), node);
    Ok(())
}

/// Both endpoints of an edge must be objects this tenant holds; an edge
/// into nothing is a refusal, never a silently trimmed result.
fn endpoints_exist<S: GraphSource>(
    source: &S,
    link: &KnowledgeGraphLinkInstance,
) -> Result<(), KnowledgeGraphQueryError> {
    for endpoint in [&link.from_entity_id, &link.to_entity_id] {
        if source.node(&link.tenant_id, endpoint)?.is_none() {
            return Err(KnowledgeGraphQueryError::DanglingLinkEndpoint {
                entity_id: endpoint.clone(),
            });
        }
    }
    Ok(())
}

pub(crate) fn walk<S: GraphSource>(
    source: &S,
    request: KnowledgeGraphQueryRequest,
) -> Result<KnowledgeGraphQueryResponse, KnowledgeGraphQueryError> {
    request.validate()?;
    if source
        .node(&request.tenant_id, &request.root_entity_id)?
        .is_none()
    {
        return Err(KnowledgeGraphQueryError::MissingRootEntity);
    }

    let edge_filter = request.edge_filter();
    for root in &request.additional_root_entity_ids {
        if source.node(&request.tenant_id, root)?.is_none() {
            return Err(KnowledgeGraphQueryError::MissingRootEntity);
        }
    }
    let cursor = request.resume_cursor.unwrap_or_default();
    let node_budget =
        MAX_QUERY_RESULT_NODES + usize::try_from(cursor.nodes_emitted).unwrap_or(usize::MAX);
    let edge_budget =
        MAX_QUERY_RESULT_EDGES + usize::try_from(cursor.edges_emitted).unwrap_or(usize::MAX);
    let mut queue = VecDeque::from([(request.root_entity_id.clone(), 0_u32)]);
    let mut seen_nodes = BTreeSet::from([request.root_entity_id.clone()]);
    for root in &request.additional_root_entity_ids {
        if seen_nodes.insert(root.clone()) {
            queue.push_back((root.clone(), 0));
        }
    }
    let mut nodes = BTreeMap::new();
    let mut node_order: Vec<String> = Vec::new();
    let mut edges = BTreeSet::new();
    let mut edge_order: Vec<KnowledgeGraphEdge> = Vec::new();
    let mut result_truncated = false;
    insert_node(
        source,
        &request.tenant_id,
        &request.root_entity_id,
        &mut nodes,
    )?;
    node_order.push(request.root_entity_id.clone());
    for root in &request.additional_root_entity_ids {
        if !nodes.contains_key(root.as_str()) {
            insert_node(source, &request.tenant_id, root, &mut nodes)?;
            node_order.push(root.clone());
        }
    }

    'bfs: while let Some((entity_id, depth)) = queue.pop_front() {
        if depth >= request.max_depth {
            continue;
        }

        let outbound = if matches!(
            request.direction,
            TraversalDirection::Outbound | TraversalDirection::Both
        ) {
            source.outbound(&request.tenant_id, &entity_id)?
        } else {
            Vec::new()
        };
        let inbound = if matches!(
            request.direction,
            TraversalDirection::Inbound | TraversalDirection::Both
        ) {
            source.inbound(&request.tenant_id, &entity_id)?
        } else {
            Vec::new()
        };

        for link in outbound.into_iter().chain(inbound) {
            if !passes_edge_filter(&link, &edge_filter)
                || !is_fresh_enough(&link, request.freshness_floor_epoch_seconds)
                || !consent_permits(&link, &request.edge_consent)
            {
                continue;
            }
            endpoints_exist(source, &link)?;

            let neighbor_id = neighbour_of(&link, &entity_id);

            // Node cap: an edge whose neighbour cannot be emitted would
            // leave a dangling endpoint in the response, so stop first.
            if !nodes.contains_key(neighbor_id.as_str()) && nodes.len() >= node_budget {
                result_truncated = true;
                break 'bfs;
            }
            if edges.len() >= edge_budget {
                result_truncated = true;
                break 'bfs;
            }
            if edges.insert(link.as_contract_edge()) {
                edge_order.push(link.as_contract_edge());
            }

            if !nodes.contains_key(neighbor_id.as_str()) {
                insert_node(source, &request.tenant_id, &neighbor_id, &mut nodes)?;
                node_order.push(neighbor_id.clone());
            }

            if seen_nodes.insert(neighbor_id.clone()) {
                queue.push_back((neighbor_id, depth + 1));
            }
        }
    }

    let skip_nodes = usize::try_from(cursor.nodes_emitted).unwrap_or(usize::MAX);
    let skip_edges = usize::try_from(cursor.edges_emitted).unwrap_or(usize::MAX);
    let page_nodes = sorted_page(
        node_order
            .iter()
            .skip(skip_nodes)
            .filter_map(|id| nodes.get(id.as_str()).cloned()),
    );
    let page_edges = sorted_page(edge_order.iter().skip(skip_edges).cloned());
    let next_cursor = result_truncated.then_some(QueryCursor {
        nodes_emitted: node_order.len() as u64,
        edges_emitted: edge_order.len() as u64,
    });
    Ok(KnowledgeGraphQueryResponse {
        query_id: request.query_id,
        tenant_id: request.tenant_id,
        nodes: page_nodes,
        edges: page_edges,
        observed_at_epoch_seconds: request.observed_at_epoch_seconds,
        result_truncated,
        next_cursor,
    })
}
