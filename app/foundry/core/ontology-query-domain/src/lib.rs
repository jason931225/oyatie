//! Ontology query-engine domain foundation: bounded, tenant-scoped,
//! deterministic traversal over validated link instances.
// ADR-0083 Tier 3: tests legitimately use `.unwrap()` / `.expect()` /
// `panic!()` to assert invariants under the `cfg(test)` exemption.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

mod contract;
mod engine;
mod link;
mod link_law;
mod request;
mod store_source;
mod traversal;

pub use contract::{
    KnowledgeGraphEdge, KnowledgeGraphLinkUpsertOutcome, KnowledgeGraphNode,
    KnowledgeGraphQueryError, KnowledgeGraphQueryResponse, MAX_QUERY_DEPTH, MAX_QUERY_RESULT_EDGES,
    MAX_QUERY_RESULT_NODES, QueryCursor, TraversalDirection,
};
pub use engine::KnowledgeGraphQueryEngine;
pub use link::KnowledgeGraphLinkInstance;
pub use request::{EdgeConsent, KnowledgeGraphQueryRequest};
pub use store_source::{StoreGraphSource, query_graph_slice_from_store};

#[cfg(test)]
mod tests;
