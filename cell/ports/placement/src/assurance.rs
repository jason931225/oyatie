use crate::{
    BoxCellFuture, CapacityVectorV1, CellProofEnvelopeV1, CellProofVerifier, Digest32,
    PlacementLocationV1, PlacementReadAuthorityV1, ProducerId, ProofConstructionError,
    ProofVerificationError, ResilienceObjectiveV1, TenantId,
};

macro_rules! opaque_id {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }

            pub fn parse(_value: impl Into<String>) -> Result<Self, ProofConstructionError> {
                Err(ProofConstructionError::NotImplemented)
            }
        }
    };
}

opaque_id!(CertificationId);
opaque_id!(KeyCustodyAuthorityId);
opaque_id!(AssuranceCompilerVersion);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AssuranceGeneration(pub u64);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocationConstraintV1 {
    PlatformPolicyOnly,
    Only(Vec<PlacementLocationV1>),
    DenyAll,
}

/// Isolation strength is a genuine ladder: every `DedicatedPhysical` cell also
/// satisfies `DedicatedLogical`, which also satisfies `SharedCertified`.
///
/// This enum therefore KEEPS `Ord`/`PartialOrd`, and `minimum_isolation` is
/// combined across requirement sources with `max`. The asymmetry against
/// [`HardwareClassV1`] and [`EncryptionRequirementV1`], which deliberately do
/// NOT derive `Ord`, is intentional and is not an oversight: those two are sets
/// of incomparable constraints, not rungs. Declaration order and protobuf tag
/// order here express the ladder; for the other two they express nothing.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum IsolationClassV1 {
    SharedCertified,
    DedicatedLogical,
    DedicatedPhysical,
}

/// Required hardware class is NOT a ladder, so this enum deliberately does not
/// derive `Ord`/`PartialOrd`.
///
/// `ConfidentialCompute` and `Accelerator` are incomparable constraints: neither
/// subsumes the other, and a cell satisfying one does not thereby satisfy the
/// other. Deriving an ordering would invite a future implementation to combine
/// two requirement sources with `max()` and silently discard one of them.
/// Combination is same-variant-else-reject; see [`AssuranceCompiler::compile`].
/// Declaration order and protobuf tag order carry no rank.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum HardwareClassV1 {
    GeneralPurpose,
    ConfidentialCompute,
    Accelerator,
}

/// Encryption requirement is NOT a ladder, so this enum deliberately does not
/// derive `Ord`/`PartialOrd`.
///
/// `CustomerManaged` and `ExternalKeyManager` are incomparable custody models,
/// not increasing strengths: a customer-managed key in the platform key service
/// does not satisfy an external key manager requirement, and the converse also
/// fails. Deriving an ordering would invite a future implementation to combine
/// two requirement sources with `max()` and silently discard one of them.
/// Combination is same-variant-else-reject; see [`AssuranceCompiler::compile`].
/// Declaration order and protobuf tag order carry no rank.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EncryptionRequirementV1 {
    ProviderManaged,
    CustomerManaged,
    ExternalKeyManager,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ExclusiveTenantKeyRequirementV1 {
    NotRequired,
    Required,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ImmutableAuditStorageRequirementV1 {
    NotRequired,
    Required,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyCustodyRequirementV1 {
    pub permitted_authorities: Vec<KeyCustodyAuthorityId>,
    pub exclusive_tenant_key: ExclusiveTenantKeyRequirementV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditRequirementV1 {
    pub minimum_retention_seconds: u64,
    pub immutable_storage: ImmutableAuditStorageRequirementV1,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ObjectiveDurationSecondsV1(pub u64);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DedicatedWarmRecoveryReserveV1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WarmRecoveryRequirementV1 {
    pub allowed_recovery_locations: LocationConstraintV1,
    pub recovery_capacity: CapacityVectorV1,
    pub recovery_resilience: ResilienceObjectiveV1,
    pub reserve_model: DedicatedWarmRecoveryReserveV1,
    pub maximum_rpo: ObjectiveDurationSecondsV1,
    pub maximum_rto: ObjectiveDurationSecondsV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StandardRecoveryRequirementV1 {
    pub backup_storage_locations: LocationConstraintV1,
    pub restore_staging_locations: LocationConstraintV1,
    pub allowed_restore_locations: LocationConstraintV1,
    pub maximum_rpo: ObjectiveDurationSecondsV1,
    pub maximum_rto: ObjectiveDurationSecondsV1,
}

/// Recovery model is NOT a ladder. `Standard` (backup and restore) and `Warm`
/// (dedicated standing reserve) are different recovery architectures with
/// different operand sets, not increasing strengths of one architecture.
/// Combination is same-variant-else-reject; see [`AssuranceCompiler::compile`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecoveryRequirementV1 {
    Standard(StandardRecoveryRequirementV1),
    Warm(WarmRecoveryRequirementV1),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectiveAssuranceRequirementsV1 {
    pub generation: AssuranceGeneration,
    pub compiler_version: AssuranceCompilerVersion,
    pub primary_locations: LocationConstraintV1,
    pub required_certifications: Vec<CertificationId>,
    pub minimum_isolation: IsolationClassV1,
    pub required_hardware: HardwareClassV1,
    pub encryption: EncryptionRequirementV1,
    pub key_custody: KeyCustodyRequirementV1,
    pub audit: AuditRequirementV1,
    pub recovery: RecoveryRequirementV1,
    pub capability_requirements_digest: Digest32,
    pub compiled_requirements_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssuranceRequirementSetV1 {
    pub source_authority_id: String,
    pub source_repository_id: String,
    pub source_object_id: String,
    pub source_object_version: u64,
    pub ordered_requirement_root_digest: Digest32,
    pub requirement_count: u64,
    pub source_content_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssuranceRequirementV1 {
    pub primary_locations: LocationConstraintV1,
    pub required_certifications: Vec<CertificationId>,
    pub minimum_isolation: IsolationClassV1,
    pub required_hardware: HardwareClassV1,
    pub encryption: EncryptionRequirementV1,
    pub key_custody: KeyCustodyRequirementV1,
    pub audit: AuditRequirementV1,
    pub recovery: RecoveryRequirementV1,
    pub requirement_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssuranceRequirementMemberV1 {
    pub ordinal: u64,
    pub source_id: String,
    pub requirement: AssuranceRequirementV1,
    pub member_digest: Digest32,
    pub inclusion_path: Vec<Digest32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssuranceRequirementPageTokenV1(Vec<u8>);

impl AssuranceRequirementPageTokenV1 {
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn parse(_value: Vec<u8>) -> Result<Self, crate::PlacementContractError> {
        Err(crate::PlacementContractError::NotImplemented)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssuranceRequirementPageRequestV1 {
    pub set: AssuranceRequirementSetV1,
    pub page_size: u32,
    pub page_token: Option<AssuranceRequirementPageTokenV1>,
    pub maximum_inclusion_path_depth: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssuranceRequirementPageV1 {
    pub members: Vec<AssuranceRequirementMemberV1>,
    pub next_page_token: Option<AssuranceRequirementPageTokenV1>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssuranceCompilationPayloadV1 {
    pub schema_version: u32,
    pub tenant_id: TenantId,
    pub generation: AssuranceGeneration,
    pub compiler_version: AssuranceCompilerVersion,
    pub tenant_floor: AssuranceRequirementSetV1,
    pub capability_requirements: AssuranceRequirementSetV1,
    pub platform_policy_generation: u64,
    pub platform_policy_digest: Digest32,
    pub effective: EffectiveAssuranceRequirementsV1,
    pub compiled_at_unix_seconds: u64,
    pub record_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedAssuranceCompilationV1 {
    pub payload: AssuranceCompilationPayloadV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssuranceCompilationExpectationV1 {
    pub tenant_id: TenantId,
    pub generation: AssuranceGeneration,
    pub compiler_version: AssuranceCompilerVersion,
    pub tenant_floor_digest: Digest32,
    pub capability_requirements_digest: Digest32,
    pub platform_policy_generation: u64,
    pub platform_policy_digest: Digest32,
    pub expected_producer: ProducerId,
    pub expected_audience: ProducerId,
    pub now_unix_seconds: u64,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedAssuranceCompilation(SignedAssuranceCompilationV1);

impl VerifiedAssuranceCompilation {
    #[must_use]
    pub fn signed(&self) -> &SignedAssuranceCompilationV1 {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AssuranceCompilationErrorV1 {
    MissingTenantFloor,
    MissingCapabilityRequirement,
    DuplicateRequirement,
    ContradictoryPrimaryLocation,
    ContradictoryRecoveryLocation,
    ContradictoryRealmOrSovereignty,
    ContradictoryKeyCustody,
    UnsupportedIsolation,
    UnsupportedHardware,
    UnprovableCertification,
    UnprovableRecoveryObjective,
    IncomparableHardwareClass,
    IncomparableEncryptionRequirement,
    IncomparableRecoveryRequirement,
    IncomparableRecoveryResilience,
    ArithmeticOverflow,
    VerificationFailed,
    NotImplemented,
}

pub fn verify_assurance_compilation(
    _verifier: &dyn CellProofVerifier,
    _signed: SignedAssuranceCompilationV1,
    _expectation: &AssuranceCompilationExpectationV1,
) -> Result<VerifiedAssuranceCompilation, ProofVerificationError> {
    Err(ProofVerificationError::NotImplemented)
}

/// Compiles a tenant assurance floor and a capability requirement set into one
/// effective requirement record.
pub trait AssuranceCompiler: Send + Sync {
    /// Compiles a tenant assurance floor and a capability requirement set into one
    /// [`EffectiveAssuranceRequirementsV1`].
    ///
    /// # Combination law
    ///
    /// Two requirement sources are combined field by field. The result is the
    /// strictest requirement that satisfies BOTH sources; where no such requirement
    /// exists the compilation is rejected rather than resolved by ranking. This law
    /// binds every future implementation of this trait, including ones that do not
    /// exist yet.
    ///
    /// - `required_certifications`: UNION. A certification demanded by either source
    ///   is demanded by the result. Duplicates collapse; `DuplicateRequirement`
    ///   reports a repeated identity within one source.
    /// - `key_custody.permitted_authorities`: INTERSECT. Only an authority permitted
    ///   by both sources survives. An empty intersection is
    ///   `ContradictoryKeyCustody`, never an empty "unconstrained" list.
    /// - `key_custody.exclusive_tenant_key` and `audit.immutable_storage`: `Required`
    ///   from either source wins over `NotRequired`.
    /// - `audit.minimum_retention_seconds`: MAX. A floor combines upward.
    /// - `maximum_rpo` and `maximum_rto` (on the recovery requirement): MIN. A
    ///   ceiling combines downward.
    /// - `minimum_isolation`: MAX over the [`IsolationClassV1`] ladder. This is the
    ///   ONLY field combined by ordinal, and it is legitimate because the variants
    ///   are genuine rungs.
    /// - `required_hardware`: SAME-VARIANT-ELSE-REJECT. Equal variants combine to
    ///   themselves; any two distinct [`HardwareClassV1`] variants are incomparable
    ///   and yield `IncomparableHardwareClass`. `ConfidentialCompute` and
    ///   `Accelerator` must never be ranked against each other.
    /// - `encryption`: SAME-VARIANT-ELSE-REJECT, yielding
    ///   `IncomparableEncryptionRequirement`. `CustomerManaged` and
    ///   `ExternalKeyManager` must never be ranked against each other.
    /// - `recovery`: SAME-VARIANT-ELSE-REJECT on the [`RecoveryRequirementV1`]
    ///   variant, yielding `IncomparableRecoveryRequirement`. Within a matching
    ///   variant, its own location constraints intersect by the rule below and its
    ///   `maximum_rpo`/`maximum_rto` combine by MIN. Within a matching `Warm`,
    ///   `recovery_capacity` combines per capacity dimension by MAX, and
    ///   `recovery_resilience` is EQUAL-ELSE-REJECT, yielding
    ///   `IncomparableRecoveryResilience`. A [`ResilienceObjectiveV1`] is a
    ///   coverage attestation — a correlation-set root and count, an ordered
    ///   scenario root and count, and a coverage proof digest — so two unequal
    ///   objectives cannot be merged at all: synthesizing a combined coverage proof
    ///   would fabricate evidence for scenarios nobody attested. Reject rather than
    ///   rank or invent, exactly as for hardware and encryption.
    /// - Every [`LocationConstraintV1`] field (`primary_locations` and each recovery
    ///   location field): INTERSECT. The algebra has exactly three cases, and a
    ///   DECLARED DENY IS NOT A CONTRADICTION:
    ///     1. `PlatformPolicyOnly` is the IDENTITY element: combined with anything
    ///        it yields that other side unchanged.
    ///     2. `DenyAll` on either side is ABSORBING and COMPILES SUCCESSFULLY to
    ///        `DenyAll`. A source that declares `DenyAll` genuinely permits nothing
    ///        there; that is a well-formed requirement, and its consequence is a
    ///        precise refusal later at selection, not a compilation error. Reporting
    ///        a deliberate deny as a contradiction would be a misleading diagnostic.
    ///     3. `Only(a)` combined with `Only(b)` yields `Only(a INTERSECT b)` when
    ///        that intersection is non-empty, and otherwise REJECTS. Two non-empty
    ///        location sets with nothing in common cannot both be satisfied, so this
    ///        is the only case that raises `ContradictoryPrimaryLocation` (from
    ///        `primary_locations`) or `ContradictoryRecoveryLocation` (from a
    ///        recovery location field). A rejected compilation produces the error
    ///        and NO requirement record; it does not first produce a `DenyAll`.
    ///
    /// Protobuf tag order in `cell/facade/proto/cell/placement/v1/assurance.proto`
    /// carries no rank for any of these enums; it is wire identity only. Rank exists
    /// solely where this contract names a ladder.
    fn compile<'a>(
        &'a self,
        authority: &'a PlacementReadAuthorityV1,
        tenant_id: &'a TenantId,
        tenant_floor: &'a AssuranceRequirementSetV1,
        capability_requirements: &'a AssuranceRequirementSetV1,
        generation: AssuranceGeneration,
    ) -> BoxCellFuture<'a, Result<SignedAssuranceCompilationV1, AssuranceCompilationErrorV1>>;
}

pub trait AssuranceRequirementReader: Send + Sync {
    fn read_page<'a>(
        &'a self,
        authority: &'a PlacementReadAuthorityV1,
        request: &'a AssuranceRequirementPageRequestV1,
    ) -> BoxCellFuture<'a, Result<AssuranceRequirementPageV1, AssuranceCompilationErrorV1>>;
}
