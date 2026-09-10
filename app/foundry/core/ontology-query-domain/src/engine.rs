//! The traversal engine and its private link indexes.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use data_ontology_kernel::{ObjectGraph, OntologyEngine};

use crate::contract::*;
use crate::link::KnowledgeGraphLinkInstance;
use crate::request::KnowledgeGraphQueryRequest;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct KnowledgeGraphQueryEngine {
    links: BTreeMap<KnowledgeGraphLinkKey, KnowledgeGraphLinkInstance>, // data_class: INTERNAL_ONLY
    inbound: BTreeMap<KnowledgeGraphLinkInboundKey, KnowledgeGraphLinkInstance>, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct KnowledgeGraphLinkKey {
    tenant_id: String,
    from_entity_id: String,
    edge_type_id: String,
    to_entity_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct KnowledgeGraphLinkInboundKey {
    tenant_id: String,
    to_entity_id: String,
    edge_type_id: String,
    from_entity_id: String,
}

impl KnowledgeGraphQueryEngine {
    pub fn upsert_link(
        &mut self,
        registry: &OntologyEngine,
        graph: &ObjectGraph,
        link: KnowledgeGraphLinkInstance,
    ) -> Result<KnowledgeGraphLinkUpsertOutcome, KnowledgeGraphQueryError> {
        crate::link_law::check_registered(registry, &link)?;
        validate_link_endpoints(graph, &link)?;
        let key = link.key();
        let inbound_key = link.inbound_key();
        let outcome = if self.links.insert(key, link.clone()).is_some() {
            KnowledgeGraphLinkUpsertOutcome::Updated
        } else {
            KnowledgeGraphLinkUpsertOutcome::Inserted
        };
        self.inbound.insert(inbound_key, link);
        Ok(outcome)
    }

    /// Traverse the in-memory index. The walk itself lives in
    /// `traversal`, so this path and the store-backed one cannot drift.
    pub fn query_graph_slice(
        &self,
        graph: &ObjectGraph,
        request: KnowledgeGraphQueryRequest,
    ) -> Result<KnowledgeGraphQueryResponse, KnowledgeGraphQueryError> {
        crate::traversal::walk(
            &InMemorySource {
                engine: self,
                graph,
            },
            request,
        )
    }

    pub fn link_count(&self) -> usize {
        self.links.len()
    }

    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }

    fn outbound_links<'a>(
        &'a self,
        tenant_id: &'a str,
        from_entity_id: &'a str,
    ) -> impl Iterator<Item = &'a KnowledgeGraphLinkInstance> + 'a {
        self.links
            .range(
                KnowledgeGraphLinkKey {
                    tenant_id: tenant_id.to_string(),
                    from_entity_id: from_entity_id.to_string(),
                    edge_type_id: String::new(),
                    to_entity_id: String::new(),
                }..,
            )
            .map_while(move |(key, link)| {
                ((key.tenant_id == tenant_id) && (key.from_entity_id == from_entity_id))
                    .then_some(link)
            })
    }

    fn inbound_links<'a>(
        &'a self,
        tenant_id: &'a str,
        to_entity_id: &'a str,
    ) -> impl Iterator<Item = &'a KnowledgeGraphLinkInstance> + 'a {
        self.inbound
            .range(
                KnowledgeGraphLinkInboundKey {
                    tenant_id: tenant_id.to_string(),
                    to_entity_id: to_entity_id.to_string(),
                    edge_type_id: String::new(),
                    from_entity_id: String::new(),
                }..,
            )
            .map_while(move |(key, link)| {
                ((key.tenant_id == tenant_id) && (key.to_entity_id == to_entity_id)).then_some(link)
            })
    }
}

impl KnowledgeGraphLinkInstance {
    fn key(&self) -> KnowledgeGraphLinkKey {
        KnowledgeGraphLinkKey {
            tenant_id: self.tenant_id.clone(),
            from_entity_id: self.from_entity_id.clone(),
            edge_type_id: self.edge_type_id.clone(),
            to_entity_id: self.to_entity_id.clone(),
        }
    }

    fn inbound_key(&self) -> KnowledgeGraphLinkInboundKey {
        KnowledgeGraphLinkInboundKey {
            tenant_id: self.tenant_id.clone(),
            to_entity_id: self.to_entity_id.clone(),
            edge_type_id: self.edge_type_id.clone(),
            from_entity_id: self.from_entity_id.clone(),
        }
    }
}

struct InMemorySource<'a> {
    engine: &'a KnowledgeGraphQueryEngine,
    graph: &'a ObjectGraph,
}

impl crate::traversal::GraphSource for InMemorySource<'_> {
    fn node(
        &self,
        tenant_id: &str,
        entity_id: &str,
    ) -> Result<Option<KnowledgeGraphNode>, KnowledgeGraphQueryError> {
        // An in-memory map cannot fail; absence is Ok(None).
        Ok(self
            .graph
            .get(tenant_id, entity_id)
            .map(|entity| KnowledgeGraphNode {
                entity_id: entity_id.to_string(),
                entity_type_id: entity.entity_type.value.clone(),
            }))
    }

    fn outbound(
        &self,
        tenant_id: &str,
        entity_id: &str,
    ) -> Result<Vec<KnowledgeGraphLinkInstance>, KnowledgeGraphQueryError> {
        Ok(self
            .engine
            .outbound_links(tenant_id, entity_id)
            .cloned()
            .collect())
    }

    fn inbound(
        &self,
        tenant_id: &str,
        entity_id: &str,
    ) -> Result<Vec<KnowledgeGraphLinkInstance>, KnowledgeGraphQueryError> {
        Ok(self
            .engine
            .inbound_links(tenant_id, entity_id)
            .cloned()
            .collect())
    }
}
