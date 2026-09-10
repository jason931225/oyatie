//! The engine-level error vocabulary.

use crate::definitions::LinkCardinality;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OntologyEngineError {
    InvalidTenantId,
    InvalidTypeId,
    InvalidLinkTypeId,
    InvalidActionTypeId,
    EmptyDisplayName,
    EmptyProperties,
    EmptyPropertyName,
    DuplicateEntityType,
    DuplicateLinkType,
    DuplicateActionType,
    UnknownEntityType,
    UnknownActionType,
    EmptySurface,
    EmptyAuditEventType,
    EmptyDecisionId,
    EmptyPrincipalId,
    EmptyIdempotencyKey,
    PrincipalMismatch,
    TenantMismatch,
    AuthorizationDenied,
    AutonomyTierExceeded,
    InvalidEntityId,
    /// A `LinkTypeDefinition` or `ActionTypeDefinition` references an
    /// [`EntityTypeId`] endpoint that has not been registered for the same
    /// tenant. Register all endpoint entity types before registering link or
    /// action types that reference them.
    UnknownEntityTypeEndpoint,
    /// A `LinkTypeDefinition` binds an org-pillar endpoint to a person-pillar
    /// endpoint (or vice versa). Both endpoints must share the same
    /// [`OntologyPillar`](crate::OntologyPillar), or at least one must be
    /// pillar-agnostic (`pillar: None`).
    CrossPillarLink,
    /// The candidate revision is not strictly greater than the stored revision.
    /// [`OntologyEngine::evolve_entity_type`](crate::OntologyEngine::evolve_entity_type) requires
    /// `candidate.revision > stored.revision`.
    NonMonotonicRevision,
    /// The candidate definition removes or mutates an existing property, or
    /// introduces a new property with `required: true`. Every object
    /// projected under the prior revision lacks a newly declared property,
    /// so a required one would invalidate the existing population.
    IncompatibleSchemaEvolution,
    /// The candidate definition changes the stored [`OntologyPillar`](crate::OntologyPillar)
    /// annotation (including adding or removing it). Link types were
    /// endpoint-validated against the stored pillar at registration time
    /// ([`OntologyEngineError::CrossPillarLink`]), so the pillar is
    /// immutable under [`OntologyEngine::evolve_entity_type`](crate::OntologyEngine::evolve_entity_type).
    PillarChangedOnEvolution,
    /// `register_link_instance` was called with a [`LinkTypeId`] that has not
    /// been registered for the tenant. Register the link type before creating
    /// instances of it.
    UnknownLinkType,
    /// A display-metadata field is present but blank. Blank display
    /// strings render as nothing while looking configured; the field label
    /// names the offender.
    BlankDisplayField {
        /// Static label of the blank display field.
        field: String,
    },
    /// An evolution candidate changes a field that is frozen for its
    /// definition kind.
    FrozenFieldChangedOnEvolution {
        /// Static label of the frozen field that differed.
        field: String,
    },
    /// A property or parameter declaration carries a malformed value type
    /// ([`ValueTypeDeclaration::validate`](crate::ValueTypeDeclaration::validate)
    /// refused it at registration).
    InvalidValueType {
        /// The property or parameter name carrying the bad declaration.
        name: String,
        /// The precise structural cause.
        cause: crate::value::ValueTypeError,
    },
    /// A literal-built definition states a tier that differs from its value
    /// type's projection (or declares a value type on a tier the projection
    /// never yields — the exotic tiers stay untyped in V1).
    ValueTypeTierMismatch {
        /// The property or parameter name.
        name: String,
    },
    /// A designated primary-key or title property names no declared
    /// property of the entity type.
    DesignatedPropertyNotDeclared {
        /// The designated name that no declared property carries.
        name: String,
    },
    /// The designated primary-key property is declared `required: false`.
    /// A key property must be present on every conformant instance.
    PrimaryKeyPropertyNotRequired {
        /// The designated primary-key property name.
        name: String,
    },
    /// The candidate definition changes an already-set primary-key
    /// designation (including removing it); adopting a key where none was
    /// set remains allowed.
    PrimaryKeyChangedOnEvolution,
    /// An [`ActionParameterDefinition`](crate::ActionParameterDefinition)
    /// was constructed with a blank name.
    EmptyParameterName,
    /// An action type declares two parameters with the same name
    /// ([`OntologyEngine::register_action_type`](crate::OntologyEngine::register_action_type)).
    DuplicateParameterName {
        /// The duplicated parameter name.
        name: String,
    },
    /// A submission omits a declared parameter with `required: true`
    /// ([`OntologyEngine::check_action_parameter_conformance`](crate::OntologyEngine::check_action_parameter_conformance)).
    MissingRequiredParameter {
        /// Name of the required parameter the submission lacks.
        name: String,
    },
    /// A submission carries a parameter the action type does not declare.
    /// Conformance is fail-closed on vocabulary.
    UndeclaredParameter {
        /// Name of the parameter the action type does not declare.
        name: String,
    },
    /// A submitted parameter's [`PropertyTier`](crate::PropertyTier) differs
    /// from the declared tier.
    ParameterTierMismatch {
        /// Name of the mismatched parameter.
        name: String,
    },
    /// A submitted parameter's data class differs from the declared class.
    ParameterDataClassMismatch {
        /// Name of the mismatched parameter.
        name: String,
    },
    /// An instance names a definition property with `required: true` that is
    /// absent from the instance's property set
    /// ([`OntologyEngine::check_instance_conformance`](crate::OntologyEngine::check_instance_conformance)).
    MissingRequiredProperty {
        /// Name of the required property the instance lacks.
        name: String,
    },
    /// An instance carries a property its entity type definition does not
    /// declare. Conformance is fail-closed on vocabulary: evolve the type
    /// (additively) before writing the property.
    UndeclaredProperty {
        /// Name of the property the definition does not declare.
        name: String,
    },
    /// An instance property's [`PropertyTier`](crate::PropertyTier) differs
    /// from the tier its definition declares.
    PropertyTierMismatch {
        /// Name of the mismatched property.
        name: String,
    },
    /// An instance property's data class differs from the class its
    /// definition declares.
    PropertyDataClassMismatch {
        /// Name of the mismatched property.
        name: String,
    },
    /// An instance property's carrier does not conform to its declared
    /// value type — or, for an untyped (`None`) declaration, the carrier is
    /// not the legacy `PropertyValue::String` the bridge constructors
    /// produce. The FINAL per-property conformance step; diagnostics carry
    /// names, paths, and static type labels only, never classified values.
    PropertyValueTypeMismatch {
        /// Name of the nonconforming property.
        name: String,
        /// Dotted/indexed path from the value root, `""` at the root.
        path: String,
        /// The declared expectation at that path.
        expected: &'static str,
        /// What the carrier actually held (`"absent"` for a missing
        /// required struct field).
        found: &'static str,
    },
    /// A submitted action parameter's carrier does not conform to its
    /// declared value type — the parameter mirror of
    /// [`OntologyEngineError::PropertyValueTypeMismatch`], same `None`
    /// rule, same diagnostic shape.
    ParameterValueTypeMismatch {
        /// Name of the nonconforming parameter.
        name: String,
        /// Dotted/indexed path from the value root, `""` at the root.
        path: String,
        /// The declared expectation at that path.
        expected: &'static str,
        /// What the carrier actually held (`"absent"` for a missing
        /// required struct field).
        found: &'static str,
    },
    /// A link instance would violate the [`LinkCardinality`] declared on the
    /// [`LinkTypeDefinition`]. The `cardinality` field carries the constraint
    /// that was violated.
    CardinalityViolation {
        /// The cardinality constraint that was violated.
        cardinality: LinkCardinality,
    },
}
