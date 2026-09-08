//! The migration runner: scan the projection for objects the plan still
//! owes, compute their upcast targets from log-derived values, and submit
//! each as an ordinary Action through the ONE writer — which stamps the
//! head revision itself. Deterministic over (plan, projection): no clocks,
//! no minted randomness. Pending is a VALUE predicate (a computed target
//! differing from the current value), so crash-and-rerun converges, a
//! rerun at fixpoint submits nothing, and keys are drift-sensitive by the
//! scanned last-ordinal. A pass that makes no progress stops — refusals
//! and conflicts repeat deterministically and looping on them is spin.

use data_ontology_kernel::{
    ActionInvocationRequest, ActionPolicyDecision, ActionTypeId, AutonomyTier,
    EntityTypeDefinition, EntityTypeId, ObjectEntity, PropertyValue,
};
use foundry_edits::{EditSet, OntologyEdit, WireProperty, WireValue};
use foundry_records_draft::{RecordsLog, RecordsLogError};
use std::collections::BTreeSet;

use crate::boundary;
use crate::state::ProjectionState;
use crate::writer::{ActionSubmission, ApplyOutcome, WriteError, submit};

use super::value::Fnv1a64;
use super::{DefaultValue, MigrationPlan, PlanError, UpcastTransform, ValueConversion, declared};

/// Who runs the migration: the principal and the policy decision that
/// authorizes the whole run. Per-object identity lives in the request and
/// its drift-sensitive idempotency key, never here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationAuthority {
    pub principal_id: String,          // data_class: INTERNAL_ONLY
    pub decision_id: String,           // data_class: INTERNAL_ONLY
    pub allowed_surfaces: Vec<String>, // data_class: INTERNAL_ONLY
    pub autonomy_tier: AutonomyTier,   // data_class: INTERNAL_ONLY
}

/// One object the plan still owes, with its computed target values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingUpcast {
    pub object_ref: String, // data_class: INTERNAL_ONLY
    /// The per-tenant ordinal of the object's last applied envelope at
    /// scan time — the drift-sensitive component of the idempotency key.
    pub last_ordinal: u64, // data_class: INTERNAL_ONLY
    pub targets: Vec<WireProperty>, // data_class: PROPERTY_VALUE_PRIVACY_CLASS
}

/// What one run did and where the population stands after it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MigrationStatus {
    /// Objects of the plan's entity type in this tenant's projection.
    pub total: u64, // data_class: INTERNAL_ONLY
    /// Upcast entries applied by this run.
    pub upcast: u64, // data_class: INTERNAL_ONLY
    /// Objects still owed after the final rescan.
    pub pending: u64, // data_class: INTERNAL_ONLY
    /// Submissions a writer gate refused (each is on the denial trail).
    pub refused: u64, // data_class: INTERNAL_ONLY
    /// Appends the log refused as divergent idempotency-key reuse. An
    /// ATTEMPT count: there is no receipt to deduplicate against, so a
    /// re-attempt of the same object in a later pass counts again.
    pub conflicted: u64, // data_class: INTERNAL_ONLY
    /// Appends the log could not accept at all — an adapter fault, not a
    /// verdict about the submission. Held apart from `conflicted` because
    /// this crate's own law says so: a storage fault reported as a key
    /// conflict is "a cause that did not occur, blame in the wrong place,
    /// and advice against the retry that would work".
    pub unavailable: u64, // data_class: INTERNAL_ONLY
    /// DISTINCT poisoned ordinals this run observed — entries, not attempts.
    pub poisoned: u64, // data_class: INTERNAL_ONLY
    /// `pending == 0` over a full rescan — the plan's value fixpoint.
    pub fixpoint: bool, // data_class: INTERNAL_ONLY
}

/// `mig_<plan digest>_<object digest>:<last ordinal>` — fixed-width
/// digests keep the key inside the envelope cap for any object_ref.
pub fn upcast_idempotency_key(plan: &MigrationPlan, object_ref: &str, last_ordinal: u64) -> String {
    let mut digest = Fnv1a64::new();
    digest.write(object_ref.as_bytes());
    format!(
        "mig_{}_{:016x}:{}",
        plan.digest16(),
        digest.finish(),
        last_ordinal
    )
}

/// Every object of the plan's entity type whose computed targets differ
/// from its current values. A pure view: unknown types scan to empty.
pub fn pending_objects(state: &ProjectionState, plan: &MigrationPlan) -> Vec<PendingUpcast> {
    let Ok(type_id) = EntityTypeId::new(plan.entity_type.clone()) else {
        return Vec::new();
    };
    let Some(head) = state.engine.entity_type(&state.tenant_id, &type_id) else {
        return Vec::new();
    };
    let mut pending = Vec::new();
    for (object_ref, binding) in &state.bindings {
        if binding.entity_type != plan.entity_type {
            continue;
        }
        let Some(entity) = state.objects.get(&state.tenant_id, object_ref) else {
            continue;
        };
        let targets: Vec<WireProperty> = plan
            .transforms
            .iter()
            .filter_map(|transform| computed_target(transform, entity, head))
            .collect();
        if !targets.is_empty() {
            pending.push(PendingUpcast {
                object_ref: object_ref.clone(),
                last_ordinal: binding.last_ordinal,
                targets,
            });
        }
    }
    pending
}

/// Validate, then scan-and-submit passes until the value fixpoint or a
/// pass without progress. An invalid plan touches nothing.
pub fn run_to_fixpoint(
    plan: &MigrationPlan,
    authority: &MigrationAuthority,
    log: &mut dyn RecordsLog,
    denial_log: &mut dyn RecordsLog,
    projection: &mut ProjectionState,
) -> Result<MigrationStatus, PlanError> {
    plan.validate(&projection.registry_input)?;
    let action_id =
        ActionTypeId::new(plan.action_type.clone()).map_err(|_| PlanError::InvalidActionType)?;
    let mut seen_poison: BTreeSet<u64> = BTreeSet::new();
    let mut status = MigrationStatus {
        total: 0,
        upcast: 0,
        pending: 0,
        refused: 0,
        conflicted: 0,
        unavailable: 0,
        poisoned: 0,
        fixpoint: false,
    };
    loop {
        let pending = pending_objects(projection, plan);
        if pending.is_empty() {
            break;
        }
        let mut progressed = false;
        for owed in pending {
            let key = upcast_idempotency_key(plan, &owed.object_ref, owed.last_ordinal);
            // Non-empty by the pending predicate; fail-closed anyway.
            let Ok(edit) = OntologyEdit::upsert_properties(owed.targets) else {
                status.refused += 1;
                continue;
            };
            let Ok(edits) = EditSet::new(vec![edit]) else {
                status.refused += 1;
                continue;
            };
            let submission = ActionSubmission {
                request: ActionInvocationRequest {
                    tenant_id: plan.tenant_id.clone(),
                    principal_id: authority.principal_id.clone(),
                    action_id: action_id.clone(),
                    entity_id: owed.object_ref.clone(),
                    idempotency_key: key,
                    requested_at_epoch_seconds: plan.declared_at_epoch_seconds,
                },
                decision: ActionPolicyDecision {
                    decision_id: authority.decision_id.clone(),
                    tenant_id: plan.tenant_id.clone(),
                    principal_id: authority.principal_id.clone(),
                    allowed_surfaces: authority.allowed_surfaces.clone(),
                    autonomy_tier: authority.autonomy_tier,
                },
                parameters: Vec::new(),
                edits,
            };
            match submit(submission, log, denial_log, projection) {
                Ok(ApplyOutcome::Applied { .. }) => {
                    status.upcast += 1;
                    progressed = true;
                }
                Ok(ApplyOutcome::Poisoned { receipt, .. }) => {
                    // NOT progress. A poison stands in the log and advances
                    // the fold, but it never binds the object: `apply_sealed`
                    // leaves `bindings` untouched, so `plan_owes` still owes
                    // it, the same drift-sensitive key is re-derived, the
                    // byte-identical append deduplicates onto the same
                    // poisoned ordinal, and the next pass is identical to
                    // this one. Counting it as progress made this loop
                    // unbounded — a fixed point of its own body that it
                    // refused to recognise, holding the tenant lock forever.
                    // The module's own law says a pass that makes no progress
                    // stops; a poison is exactly that class.
                    // DISTINCT ORDINALS THIS RUN OBSERVED, not receipts it
                    // appended. A later pass re-submits the same object under
                    // the same drift-sensitive key and deduplicates onto the
                    // ordinal already poisoned; counting that again reports
                    // two poisoned entries where one exists. But gating on
                    // `!deduplicated` under-counts the other way: a byte-
                    // identical retry from an EARLIER run also deduplicates,
                    // so a second run would report every field zero while a
                    // poisoned entry still blocks the object — the bare count
                    // with no reason in it that this module refuses to emit.
                    if seen_poison.insert(receipt.ordinal) {
                        status.poisoned += 1;
                    }
                }
                Err(WriteError::Refused(_)) => {
                    status.refused += 1;
                }
                Err(WriteError::Log(RecordsLogError::IdempotencyConflict { .. })) => {
                    status.conflicted += 1;
                }
                Err(WriteError::Log(RecordsLogError::Storage { .. })) => {
                    status.unavailable += 1;
                }
            }
        }
        if !progressed {
            break;
        }
    }
    status.pending = pending_objects(projection, plan).len() as u64;
    status.fixpoint = status.pending == 0;
    status.total = projection
        .bindings
        .values()
        .filter(|binding| binding.entity_type == plan.entity_type)
        .count() as u64;
    Ok(status)
}

/// The plan's computed value for one target on one object, or `None` when
/// the transform owes nothing: source absent, default already present, or
/// the current value already equal (compared in kernel space through the
/// one boundary seam).
pub(super) fn computed_target(
    transform: &UpcastTransform,
    entity: &ObjectEntity,
    head: &EntityTypeDefinition,
) -> Option<WireProperty> {
    let to = transform.to_name();
    let target = declared(head, to)?;
    let current = entity.properties.get(to);
    let computed = match transform {
        UpcastTransform::CopyAs { from, .. } => {
            boundary::wire_value(&entity.properties.get(from.as_str())?.value.value).ok()?
        }
        UpcastTransform::ConvertAs {
            from, conversion, ..
        } => {
            let source = entity.properties.get(from.as_str())?;
            // Kind mismatches cannot survive validate() plus write-time
            // value conformance; owe nothing rather than guess.
            match (conversion, &source.value.value) {
                (ValueConversion::IntegerToString, PropertyValue::Integer(number)) => {
                    WireValue::String(number.to_string())
                }
                (ValueConversion::BooleanToInteger, PropertyValue::Boolean(flag)) => {
                    WireValue::Integer(i64::from(*flag))
                }
                _ => return None,
            }
        }
        UpcastTransform::DefaultTo { value, .. } => {
            if current.is_some() {
                // A default fills absence; it never overwrites.
                return None;
            }
            value.to_wire()
        }
    };
    if let Some(existing) = current
        && boundary::value(&computed).ok().as_ref() == Some(&existing.value.value)
    {
        return None;
    }
    WireProperty::new(
        to,
        boundary::wire_tier(target.tier),
        boundary::wire_label(&target.data_class)?,
        computed,
    )
    .ok()
}
