//! Link-instance registration: idempotency and cardinality enforcement
//! against the declared [`LinkCardinality`](crate::LinkCardinality).

use crate::definitions::{LinkCardinality, LinkTypeId};
use crate::error::OntologyEngineError;

use super::{LinkEndpointKey, LinkInstanceKey, OntologyEngine, ontology_scoped_key};

/// Outcome returned by [`OntologyEngine::register_link_instance`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinkInstanceOutcome {
    /// The instance was freshly inserted into the registry.
    Registered,
    /// The identical `(link_type_id, from_entity_id, to_entity_id)` tuple
    /// already existed; no state change occurred.
    AlreadyExists,
}

impl OntologyEngine {
    /// Register a directed link instance from `from_entity_id` to `to_entity_id`
    /// under `link_type_id` for `tenant_id`.
    pub fn register_link_instance(
        &mut self,
        tenant_id: &str,
        link_type_id: &LinkTypeId,
        from_entity_id: &str,
        to_entity_id: &str,
    ) -> Result<LinkInstanceOutcome, OntologyEngineError> {
        let cardinality = self.cardinality_for(tenant_id, link_type_id)?;
        let instance = LinkInstanceKey::new(tenant_id, link_type_id, from_entity_id, to_entity_id);
        if self.link_instances.contains_key(&instance) {
            return Ok(LinkInstanceOutcome::AlreadyExists);
        }
        let outbound = LinkEndpointKey::new(tenant_id, link_type_id, from_entity_id);
        let inbound = LinkEndpointKey::new(tenant_id, link_type_id, to_entity_id);
        self.check_cardinality(cardinality, &outbound, &inbound)?;
        self.index_link(instance, outbound, inbound);
        Ok(LinkInstanceOutcome::Registered)
    }

    fn cardinality_for(
        &self,
        tenant_id: &str,
        link_type_id: &LinkTypeId,
    ) -> Result<LinkCardinality, OntologyEngineError> {
        self.link_types
            .get(&ontology_scoped_key(tenant_id, &link_type_id.value))
            .map(|definition| definition.cardinality)
            .ok_or(OntologyEngineError::UnknownLinkType)
    }

    fn check_cardinality(
        &self,
        cardinality: LinkCardinality,
        outbound: &LinkEndpointKey,
        inbound: &LinkEndpointKey,
    ) -> Result<(), OntologyEngineError> {
        let occupied = match cardinality {
            LinkCardinality::OneToOne => {
                self.link_outbound.contains_key(outbound) || self.link_inbound.contains_key(inbound)
            }
            LinkCardinality::OneToMany => self.link_inbound.contains_key(inbound),
            LinkCardinality::ManyToMany => false,
        };
        if occupied {
            return Err(OntologyEngineError::CardinalityViolation { cardinality });
        }
        Ok(())
    }

    fn index_link(
        &mut self,
        instance: LinkInstanceKey,
        outbound: LinkEndpointKey,
        inbound: LinkEndpointKey,
    ) {
        self.link_instances.insert(instance, ());
        self.link_outbound.insert(outbound, ());
        self.link_inbound.insert(inbound, ());
    }
}
