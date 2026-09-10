//! The durable projection as a graph source.

use foundry_projection_draft::{ProjectedLink, ProjectionStore};

use crate::contract::{KnowledgeGraphNode, KnowledgeGraphQueryError};
use crate::link::KnowledgeGraphLinkInstance;
use crate::traversal::GraphSource;

pub struct StoreGraphSource<'a> {
    store: &'a dyn ProjectionStore,
}

impl<'a> StoreGraphSource<'a> {
    pub fn new(store: &'a dyn ProjectionStore) -> Self {
        Self { store }
    }
}

/// Milliseconds since the epoch, as the traversal's seconds. Truncating
/// division is deliberate: an edge observed at 1.9s is not fresher than
/// a floor of 2s, and rounding up would admit an edge the floor
/// excludes.
fn as_epoch_seconds(observed_at_epoch_ms: u64) -> u64 {
    observed_at_epoch_ms / 1_000
}

fn source_failure(error: impl core::fmt::Debug) -> KnowledgeGraphQueryError {
    KnowledgeGraphQueryError::Source {
        detail: format!("{error:?}"),
    }
}

fn as_link_instance(
    tenant_id: &str,
    edge: &ProjectedLink,
) -> Result<KnowledgeGraphLinkInstance, KnowledgeGraphQueryError> {
    KnowledgeGraphLinkInstance::new(
        tenant_id,
        &edge.from_object_ref,
        &edge.to_object_ref,
        &edge.link_type,
        as_epoch_seconds(edge.observed_at_epoch_ms),
    )
}

impl GraphSource for StoreGraphSource<'_> {
    fn node(
        &self,
        tenant_id: &str,
        entity_id: &str,
    ) -> Result<Option<KnowledgeGraphNode>, KnowledgeGraphQueryError> {
        Ok(self
            .store
            .get(tenant_id, entity_id)
            .map_err(source_failure)?
            .map(|object| KnowledgeGraphNode {
                entity_id: entity_id.to_string(),
                entity_type_id: object.entity.entity_type.value.clone(),
            }))
    }

    fn outbound(
        &self,
        tenant_id: &str,
        entity_id: &str,
    ) -> Result<Vec<KnowledgeGraphLinkInstance>, KnowledgeGraphQueryError> {
        self.store
            .links_from(tenant_id, entity_id)
            .map_err(source_failure)?
            .iter()
            .map(|edge| as_link_instance(tenant_id, edge))
            .collect()
    }

    fn inbound(
        &self,
        tenant_id: &str,
        entity_id: &str,
    ) -> Result<Vec<KnowledgeGraphLinkInstance>, KnowledgeGraphQueryError> {
        self.store
            .links_to(tenant_id, entity_id)
            .map_err(source_failure)?
            .iter()
            .map(|edge| as_link_instance(tenant_id, edge))
            .collect()
    }
}

pub fn query_graph_slice_from_store(
    store: &dyn ProjectionStore,
    request: crate::request::KnowledgeGraphQueryRequest,
) -> Result<crate::contract::KnowledgeGraphQueryResponse, KnowledgeGraphQueryError> {
    crate::traversal::walk(&StoreGraphSource::new(store), request)
}
