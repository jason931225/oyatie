//! The registry law at link upsert: an edge enters the store only under
//! a link type REGISTERED for its tenant. Endpoint-type and cardinality
//! law stay with the kernel's own link store — duplicating them against
//! this in-memory index would let the two planes drift.

use data_ontology_kernel::{LinkTypeId, OntologyEngine};

use crate::contract::KnowledgeGraphQueryError;
use crate::link::KnowledgeGraphLinkInstance;

/// Refuse any edge whose `lty_` type is not registered for its tenant.
/// An id the kernel cannot even parse is a fortiori unregistered.
pub(crate) fn check_registered(
    registry: &OntologyEngine,
    link: &KnowledgeGraphLinkInstance,
) -> Result<(), KnowledgeGraphQueryError> {
    let id = LinkTypeId::new(link.edge_type_id.clone())
        .map_err(|_| KnowledgeGraphQueryError::UnregisteredLinkType)?;
    if registry.link_type(&link.tenant_id, &id).is_none() {
        return Err(KnowledgeGraphQueryError::UnregisteredLinkType);
    }
    Ok(())
}
