use crate::{BindingDigest32, BindingProofEnvelopeV1, ServingAuthorityInstanceV1};

/// The restore basis a surviving quorum asserts.
///
/// This is the signed payload, not the evidence itself: nothing here is
/// trustworthy until it arrives inside
/// [`SignedServingAuthoritySurvivingQuorumV1`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthoritySurvivingQuorumPayloadV1 {
    pub schema_version: u32,
    pub instance: ServingAuthorityInstanceV1,
    pub committed_state_revision: u64,
    pub committed_state_digest: BindingDigest32,
    pub rejection_high_water_digest: BindingDigest32,
    pub quorum_evidence: cell_placement::ImmutableEvidenceRefV1,
}

/// Authenticated surviving-quorum attestation.
///
/// Restore is how serving authority returns after loss, and two of the three
/// [`ServingAuthorityRestoreBasisV1`] arms rest on this alone, so the state
/// revision and digests it carries MUST be bound by a signature. Mirrors
/// [`ServingAuthorityReplacementEvidenceV1`], whose basis is anchored by an
/// embedded `SignedServingAuthorityIndependentRetirementV1`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedServingAuthoritySurvivingQuorumV1 {
    pub payload: ServingAuthoritySurvivingQuorumPayloadV1,
    pub envelope: BindingProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthoritySurvivingQuorumEvidenceV1 {
    pub attestation: SignedServingAuthoritySurvivingQuorumV1,
}

/// What [`verify_serving_authority_surviving_quorum`] checks the attestation
/// against. Replaces the bare `&ServingAuthorityInstanceV1` the verifier used
/// to take, which named no producer, no audience and no clock, and so gave the
/// `&dyn BindingProofVerifier` argument nothing to verify.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthoritySurvivingQuorumExpectationV1 {
    pub instance: ServingAuthorityInstanceV1,
    pub expected_committed_state_revision: u64,
    pub expected_committed_state_digest: BindingDigest32,
    pub expected_rejection_high_water_digest: BindingDigest32,
    pub expected_producer: crate::BindingProducerId,
    pub expected_audience: crate::BindingProducerId,
    pub now_unix_seconds: u64,
    pub maximum_clock_uncertainty_millis: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthorityReplacementEvidenceV1 {
    pub prior_instance: ServingAuthorityInstanceV1,
    pub replacement_instance: ServingAuthorityInstanceV1,
    pub prior_installation_issuance_digest: BindingDigest32,
    pub independent_retirement: crate::SignedServingAuthorityIndependentRetirementV1,
    pub prior_effect_path_fencing_digest: BindingDigest32,
    pub terminal_closure_digest: BindingDigest32,
    pub recovery_authority_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedServingAuthoritySurvivingQuorum(ServingAuthoritySurvivingQuorumEvidenceV1);

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedServingAuthorityReplacement(ServingAuthorityReplacementEvidenceV1);

impl VerifiedServingAuthoritySurvivingQuorum {
    #[must_use]
    pub fn evidence(&self) -> &ServingAuthoritySurvivingQuorumEvidenceV1 {
        &self.0
    }
}

impl VerifiedServingAuthorityReplacement {
    #[must_use]
    pub fn evidence(&self) -> &ServingAuthorityReplacementEvidenceV1 {
        &self.0
    }
}

pub fn verify_serving_authority_surviving_quorum(
    _verifier: &dyn crate::BindingProofVerifier,
    _evidence: ServingAuthoritySurvivingQuorumEvidenceV1,
    _expectation: &ServingAuthoritySurvivingQuorumExpectationV1,
) -> Result<VerifiedServingAuthoritySurvivingQuorum, crate::BindingProofVerificationError> {
    Err(crate::BindingProofVerificationError::NotImplemented)
}

pub fn verify_serving_authority_replacement(
    _verifier: &dyn crate::BindingProofVerifier,
    _evidence: ServingAuthorityReplacementEvidenceV1,
    _expected: &ServingAuthorityInstanceV1,
    _retirement: &crate::VerifiedServingAuthorityIndependentRetirement,
    _terminal_closure: &crate::VerifiedServingAuthorityTerminalClosure,
) -> Result<VerifiedServingAuthorityReplacement, crate::BindingProofVerificationError> {
    Err(crate::BindingProofVerificationError::NotImplemented)
}

#[derive(Debug, Eq, PartialEq)]
pub enum ServingAuthorityRestoreBasisV1 {
    FirstInstallation {
        surviving_partition: Box<VerifiedServingAuthoritySurvivingQuorum>,
        uninstalled_precondition_digest: BindingDigest32,
    },
    SurvivingQuorum(Box<VerifiedServingAuthoritySurvivingQuorum>),
    IndependentlyFencedReplacement(Box<VerifiedServingAuthorityReplacement>),
}

/// Independently observes the surviving-quorum restore basis and signs what it
/// read.
///
/// # Why this is an own-write attestation and not an external arrival
///
/// The wave rule is that a signed type whose subject is this store's own write
/// needs an independent local producer, while one whose subject is another
/// party's state arrives from outside and needs none. This type is the first
/// case, established from the payload rather than inferred:
///
/// - [`ServingAuthorityInstallationWriteSetPartsV1`] carries `instance` and
///   `next_rejection_high_water`, and its `precondition` carries a
///   `rejection_high_water` in every arm. The payload carries `instance` and
///   `rejection_high_water_digest`. The attestation therefore asserts a digest
///   of the very row the write set conditions on and advances, for the very
///   instance the write set installs.
/// - The sibling [`ServingAuthorityReplacementEvidenceV1`] shows how this
///   contract names a different party: `prior_instance` against
///   `replacement_instance`, `prior_installation_issuance_digest`,
///   `prior_effect_path_fencing_digest`. Every cross-party reference there is
///   explicitly qualified. This payload's `instance` and `committed_state_*`
///   are unqualified, which in that convention denotes the subject of the
///   operation.
/// - `quorum_evidence` does not make the subject external. An
///   `ImmutableEvidenceRefV1` is `authority_id`, `repository_id`, `object_id`,
///   `object_version`, `content_digest` — where the evidence is stored and what
///   it hashes to. It identifies provenance, not whose state is attested.
///
/// So a quorum of replicas of THIS partition attests THIS partition's committed
/// state, and without an independent producer the installing party would be
/// vouching for the state it is about to write.
///
/// Accepts a read authority and the instance as a lookup key only. Returns
/// `None` when there is no surviving committed state to observe. No new proof
/// domain is required: `BindingProofDomainV1::ServingAuthoritySurvivingQuorum`
/// already exists.
pub trait ServingAuthoritySurvivingQuorumObserver: Send + Sync {
    fn observe_surviving_quorum<'a>(
        &'a self,
        authority: &'a crate::ServingAuthorityReadAuthorityV1,
        instance: &'a ServingAuthorityInstanceV1,
    ) -> crate::BoxTenancyFuture<
        'a,
        Result<Option<SignedServingAuthoritySurvivingQuorumV1>, crate::ServingAuthorityStoreError>,
    >;
}
