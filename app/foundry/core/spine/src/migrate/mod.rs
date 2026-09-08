//! Migration plans: the typed, total declaration of one D80 upcast — which
//! entity type, from which retained revision to the registry head, through
//! which transforms. Every conversion is total (no parses); targets are
//! optional non-key properties of the head revision; sources must exist at
//! the from-revision, not merely at head. The plan digest is FNV-1a-64 over
//! canonical bytes — fixed width over unbounded inputs, so the runner's
//! per-object idempotency key can never overflow the envelope cap.

mod attest;
mod dependencies;
mod runner;
mod value;

use std::collections::BTreeSet;

use data_ontology_kernel::{
    EntityTypeDefinition, EntityTypePropertyDefinition, OntologyEngine, ScalarType,
    ValueTypeDeclaration,
};

pub(crate) use attest::plan_owes;
pub use attest::{MigrationAttestation, migration_attestation};
pub use runner::{
    MigrationAuthority, MigrationStatus, PendingUpcast, pending_objects, run_to_fixpoint,
    upcast_idempotency_key,
};
use value::Fnv1a64;
pub use value::{DefaultValue, ValueConversion};

/// One per-property upcast step.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UpcastTransform {
    /// Copy `from` verbatim into `to`; declarations must be identical.
    CopyAs { from: String, to: String }, // data_class: INTERNAL_ONLY
    /// Totally convert `from` into `to`.
    ConvertAs {
        from: String,                // data_class: INTERNAL_ONLY
        to: String,                  // data_class: INTERNAL_ONLY
        conversion: ValueConversion, // data_class: INTERNAL_ONLY
    },
    /// Write a constant into `to` where the object carries no value.
    DefaultTo {
        to: String,          // data_class: INTERNAL_ONLY
        value: DefaultValue, // data_class: PROPERTY_VALUE_PRIVACY_CLASS
    },
}

impl UpcastTransform {
    fn to_name(&self) -> &str {
        match self {
            Self::CopyAs { to, .. } | Self::ConvertAs { to, .. } | Self::DefaultTo { to, .. } => to,
        }
    }
}

/// The declared shape of one migration. Validation is total over the plan
/// and the registry; execution belongs to the runner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationPlan {
    pub tenant_id: String,   // data_class: INTERNAL_ONLY
    pub entity_type: String, // data_class: INTERNAL_ONLY
    pub from_revision: u32,  // data_class: INTERNAL_ONLY
    pub to_revision: u32,    // data_class: INTERNAL_ONLY
    /// The pre-registered per-plan action type the runner submits under.
    pub action_type: String, // data_class: INTERNAL_ONLY
    /// The audit event type every upcast of this plan lands under.
    pub audit_event_type: String, // data_class: INTERNAL_ONLY
    /// When the plan was declared — the writer's occurred-at source.
    pub declared_at_epoch_seconds: u64, // data_class: INTERNAL_ONLY
    pub transforms: Vec<UpcastTransform>, // data_class: INTERNAL_ONLY
}

/// Typed refusals of plan validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlanError {
    /// `entity_type` is not an `ety_`-prefixed id.
    InvalidEntityType,
    /// `action_type` is not an `aty_`-prefixed id.
    InvalidActionType,
    /// No definition registered under `(tenant_id, entity_type)`.
    UnknownEntityType,
    /// Registered, but against a different entity type: the writer stamps
    /// the revision from the action's own type, so such a plan mis-stamps
    /// every upcast it writes.
    ActionNotBoundToEntityType,
    /// No definition registered under `(tenant_id, action_type)`. Every
    /// upcast it would submit is refused one at a time by the writer's own
    /// gate — a bare count with no reason in it.
    UnknownActionType,
    /// The registry head is not the plan's `to_revision` — evolve first.
    RegistryHeadMismatch { head: u32 },
    /// `from_revision` must be strictly below `to_revision`.
    RevisionsNotAscending,
    /// The named revision was never accepted for this entity type.
    UnretainedRevision { revision: u32 },
    /// A transform reads a property the from-revision does not declare.
    SourceAbsent { name: String },
    /// A transform writes a property the head revision does not declare.
    TargetAbsent { name: String },
    /// Targets must be optional; required-at-head stays a kernel lane.
    TargetRequired { name: String },
    /// Neither side of a transform may name the primary-key property.
    PrimaryKeyTouched { name: String },
    /// Two transforms write the same target.
    DuplicateTarget { name: String },
    /// The transform is not total over the declared types.
    TypeIncompatible { target: String },
    /// Cross-property dependencies must be acyclic to reach a fixpoint.
    CyclicTransforms,
}

impl MigrationPlan {
    /// Refuse everything the runner must never be handed. Per-transform
    /// check order is pinned: duplicate target, primary key, target
    /// declared, target optional, source primary key, source declared,
    /// type compatibility.
    pub fn validate(&self, registry: &OntologyEngine) -> Result<(), PlanError> {
        if self.from_revision >= self.to_revision {
            return Err(PlanError::RevisionsNotAscending);
        }
        let type_id = data_ontology_kernel::EntityTypeId::new(self.entity_type.clone())
            .map_err(|_| PlanError::InvalidEntityType)?;
        let head = registry
            .entity_type(&self.tenant_id, &type_id)
            .ok_or(PlanError::UnknownEntityType)?;
        if head.revision != self.to_revision {
            return Err(PlanError::RegistryHeadMismatch {
                head: head.revision,
            });
        }
        // After the entity type resolves, so a plan wrong about both hears
        // the more fundamental refusal. EXISTENCE, not shape: checking the id
        // parses and stopping let an unregistered action through both
        // surfaces — `attest` answered it with pending objects, and `run`
        // refused every object one at a time as a count with no reason in it.
        let action_id = data_ontology_kernel::ActionTypeId::new(self.action_type.clone())
            .map_err(|_| PlanError::InvalidActionType)?;
        let action = registry
            .action_type(&self.tenant_id, &action_id)
            .ok_or(PlanError::UnknownActionType)?;
        // BOUND to this entity type, not merely present: the writer stamps
        // `schema_revision` from the ACTION's type, so an action bound
        // elsewhere writes envelopes at another type's head — accepted rather
        // than poisoned, and silently wrong on every later refold.
        if action.entity_type != type_id {
            return Err(PlanError::ActionNotBoundToEntityType);
        }
        let from_definition = registry
            .entity_type_at_revision(&self.tenant_id, &type_id, self.from_revision)
            .ok_or(PlanError::UnretainedRevision {
                revision: self.from_revision,
            })?;
        let primary_key = head.primary_key_property.as_deref();
        let mut targets = BTreeSet::new();
        for transform in &self.transforms {
            let to = transform.to_name();
            if !targets.insert(to.to_string()) {
                return Err(PlanError::DuplicateTarget { name: to.into() });
            }
            if Some(to) == primary_key {
                return Err(PlanError::PrimaryKeyTouched { name: to.into() });
            }
            let target = declared(head, to).ok_or(PlanError::TargetAbsent { name: to.into() })?;
            if target.required {
                return Err(PlanError::TargetRequired { name: to.into() });
            }
            check_transform(transform, from_definition, primary_key, target)?;
        }
        dependencies::check(&self.transforms)
    }

    /// FNV-1a-64 over the plan's canonical bytes, as 16 hex characters.
    pub fn digest16(&self) -> String {
        let mut digest = Fnv1a64::new();
        for field in [
            self.tenant_id.as_str(),
            self.entity_type.as_str(),
            self.action_type.as_str(),
            self.audit_event_type.as_str(),
        ] {
            digest.write(field.as_bytes());
            digest.write(&[0]);
        }
        digest.write(&self.from_revision.to_be_bytes());
        digest.write(&self.to_revision.to_be_bytes());
        digest.write(&self.declared_at_epoch_seconds.to_be_bytes());
        for transform in &self.transforms {
            match transform {
                UpcastTransform::CopyAs { from, to } => {
                    digest.write(&[1]);
                    digest.write(from.as_bytes());
                    digest.write(&[0]);
                    digest.write(to.as_bytes());
                    digest.write(&[0]);
                }
                UpcastTransform::ConvertAs {
                    from,
                    to,
                    conversion,
                } => {
                    let tag = match conversion {
                        ValueConversion::IntegerToString => 2,
                        ValueConversion::BooleanToInteger => 3,
                    };
                    digest.write(&[tag]);
                    digest.write(from.as_bytes());
                    digest.write(&[0]);
                    digest.write(to.as_bytes());
                    digest.write(&[0]);
                }
                UpcastTransform::DefaultTo { to, value } => {
                    digest.write(&[4]);
                    digest.write(to.as_bytes());
                    digest.write(&[0]);
                    value.digest_into(&mut digest);
                }
            }
        }
        format!("{:016x}", digest.finish())
    }
}

pub(super) fn declared<'a>(
    definition: &'a EntityTypeDefinition,
    name: &str,
) -> Option<&'a EntityTypePropertyDefinition> {
    definition.properties.iter().find(|p| p.name == name)
}

fn scalar_of(declaration: &ValueTypeDeclaration) -> Option<ScalarType> {
    match declaration {
        ValueTypeDeclaration::Scalar(scalar) => Some(*scalar),
        ValueTypeDeclaration::Array { .. } | ValueTypeDeclaration::Struct(_) => None,
    }
}

fn source_checked<'a>(
    from_definition: &'a EntityTypeDefinition,
    primary_key: Option<&str>,
    from: &str,
) -> Result<&'a EntityTypePropertyDefinition, PlanError> {
    if Some(from) == primary_key {
        return Err(PlanError::PrimaryKeyTouched { name: from.into() });
    }
    declared(from_definition, from).ok_or(PlanError::SourceAbsent { name: from.into() })
}

fn check_transform(
    transform: &UpcastTransform,
    from_definition: &EntityTypeDefinition,
    primary_key: Option<&str>,
    target: &EntityTypePropertyDefinition,
) -> Result<(), PlanError> {
    let incompatible = || PlanError::TypeIncompatible {
        target: transform.to_name().into(),
    };
    match transform {
        UpcastTransform::CopyAs { from, .. } => {
            // Identical declarations only; typed -> untyped is a retype.
            let source = source_checked(from_definition, primary_key, from)?;
            if source.tier != target.tier || source.value_type != target.value_type {
                return Err(incompatible());
            }
        }
        UpcastTransform::ConvertAs {
            from, conversion, ..
        } => {
            let source = source_checked(from_definition, primary_key, from)?;
            let source_scalar = source.value_type.as_ref().and_then(scalar_of);
            let target_scalar = target.value_type.as_ref().and_then(scalar_of);
            let total = match conversion {
                // An untyped target keeps the legacy String contract.
                ValueConversion::IntegerToString => {
                    source_scalar == Some(ScalarType::Integer)
                        && (target_scalar == Some(ScalarType::String)
                            || target.value_type.is_none())
                }
                ValueConversion::BooleanToInteger => {
                    source_scalar == Some(ScalarType::Boolean)
                        && target_scalar == Some(ScalarType::Integer)
                }
            };
            if !total {
                return Err(incompatible());
            }
        }
        UpcastTransform::DefaultTo { value, .. } => {
            let satisfied = match &target.value_type {
                // Untyped declarations carry the legacy String contract.
                None => matches!(value, DefaultValue::String(_)),
                Some(declaration) => scalar_of(declaration) == Some(value.scalar_type()),
            };
            if !satisfied {
                return Err(incompatible());
            }
        }
    }
    Ok(())
}
