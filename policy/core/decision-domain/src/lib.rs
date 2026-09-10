//! The joined authorization decision scope.
//!
//! Relationship expansion materializes the complete principal hierarchy at
//! one tenant-bound store snapshot before the embedded Cedar PDP evaluates it.
//! Request identity is the only source of graph tenant and subject, and one
//! mutable work budget spans every candidate subwalk.

use std::collections::BTreeMap;

use policy_cedar_domain::rebac::{
    RebacObjectRef, RebacReadSnapshot, RebacRelation, RebacTupleStore, ResolvedRebacSnapshot,
    resolve_snapshot,
};
use policy_pdp_kernel::{EntityRecord, EntitySlice, PdpError, PdpOutcome};
use policy_rebac_domain::{ExpansionBounds, ExpansionError, ExpansionSession, ValidatedNamespace};
use shared_platform_contracts_kernel::ContractViolation;
use shared_platform_contracts_kernel::pdp::{AuthorizationRequest, EntityRef};

mod identity;

pub use identity::{IdentityMappingError, PrincipalMapping};
pub use policy_pdp_kernel::PolicyDecisionPoint;

/// One graph membership whose successful expansion becomes a Cedar parent.
///
/// Candidate-to-parent schema validation is a separate model contract; this
/// type binds decision identity and does not claim that later correction.
#[derive(Clone, Debug, PartialEq)]
pub struct MembershipCandidate {
    pub object: RebacObjectRef,
    pub relation: RebacRelation,
    pub parent: EntityRef,
}

/// Public inputs that cannot carry a second tenant or principal identity.
pub struct DecisionInputs<'a, S: RebacTupleStore> {
    store: &'a S,
    namespace: &'a ValidatedNamespace,
    identity_mapping: PrincipalMapping,
    requested_snapshot: RebacReadSnapshot,
    candidates: &'a [MembershipCandidate],
    bounds: ExpansionBounds,
}

impl<'a, S: RebacTupleStore> DecisionInputs<'a, S> {
    #[must_use]
    pub fn new(
        store: &'a S,
        namespace: &'a ValidatedNamespace,
        identity_mapping: PrincipalMapping,
        requested_snapshot: RebacReadSnapshot,
        candidates: &'a [MembershipCandidate],
        bounds: ExpansionBounds,
    ) -> Self {
        Self {
            store,
            namespace,
            identity_mapping,
            requested_snapshot,
            candidates,
            bounds,
        }
    }
}

/// Complete graph materialization plus the exact graph state it represents.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaterializedParents {
    parents: Vec<EntityRef>,
    snapshot: ResolvedRebacSnapshot,
}

impl MaterializedParents {
    #[must_use]
    pub fn parents(&self) -> &[EntityRef] {
        &self.parents
    }

    #[must_use]
    pub fn resolved_snapshot(&self) -> &ResolvedRebacSnapshot {
        &self.snapshot
    }
}

/// The Cedar result and exact relationship snapshot used to produce it.
#[derive(Debug)]
pub struct JoinedDecision {
    pub outcome: PdpOutcome,
    pub relationship_snapshot: ResolvedRebacSnapshot,
}

/// Why graph materialization was refused before Cedar consultation.
#[derive(Debug, PartialEq)]
pub enum MaterializationError {
    Identity(IdentityMappingError),
    Expansion(ExpansionError),
}

/// Why a joined decision was refused. No variant is a decided deny.
#[derive(Debug)]
pub enum DecisionError {
    Identity(IdentityMappingError),
    Expansion(ExpansionError),
    InvalidSlice(Vec<ContractViolation>),
    Pdp(PdpError),
}

impl From<MaterializationError> for DecisionError {
    fn from(error: MaterializationError) -> Self {
        match error {
            MaterializationError::Identity(error) => Self::Identity(error),
            MaterializationError::Expansion(error) => Self::Expansion(error),
        }
    }
}

/// Materialize all candidate memberships from one privately bound scope.
pub fn materialize_parents<S: RebacTupleStore>(
    inputs: &DecisionInputs<'_, S>,
    request: &AuthorizationRequest,
) -> Result<MaterializedParents, MaterializationError> {
    let identity = inputs
        .identity_mapping
        .derive(request)
        .map_err(MaterializationError::Identity)?;
    let snapshot = resolve_snapshot(
        inputs.store,
        &identity.tenant,
        inputs.requested_snapshot.clone(),
    )
    .map_err(ExpansionError::from)
    .map_err(MaterializationError::Expansion)?;
    let mut session = ExpansionSession::new(
        inputs.store,
        inputs.namespace,
        identity.subject,
        snapshot,
        inputs.bounds,
    );
    let mut parents = Vec::new();
    for candidate in inputs.candidates {
        if session
            .check(&candidate.relation, &candidate.object)
            .map_err(MaterializationError::Expansion)?
        {
            parents.push(candidate.parent.clone());
        }
    }
    Ok(MaterializedParents {
        parents,
        snapshot: session.resolved_snapshot().clone(),
    })
}

/// Expand, materialize, and evaluate one authorization request in that order.
///
/// # Errors
/// Every failure is a typed refusal and Cedar is never consulted with a
/// partial or incoherent graph.
pub fn decide<S: RebacTupleStore>(
    pdp: &dyn PolicyDecisionPoint,
    inputs: &DecisionInputs<'_, S>,
    request: &AuthorizationRequest,
    principal_attributes: BTreeMap<String, serde_json::Value>,
    context_entities: Vec<EntityRecord>,
) -> Result<JoinedDecision, DecisionError> {
    let materialized = materialize_parents(inputs, request).map_err(DecisionError::from)?;
    let principal = EntityRecord {
        uid: request.principal.clone(),
        attributes: principal_attributes,
        parents: materialized.parents,
    };
    let mut entities = Vec::with_capacity(1 + context_entities.len());
    entities.push(principal);
    entities.extend(context_entities);
    let slice = EntitySlice { entities };
    slice.validate().map_err(DecisionError::InvalidSlice)?;
    let outcome = pdp.authorize(request, &slice).map_err(DecisionError::Pdp)?;
    Ok(JoinedDecision {
        outcome,
        relationship_snapshot: materialized.snapshot,
    })
}
