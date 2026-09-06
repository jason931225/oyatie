use crate::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementActorEvidenceV1 {
    pub trust_domain: String,
    pub subject: String,
    pub credential_id: String,
    pub valid_from_unix_seconds: u64,
    pub valid_until_unix_seconds: u64,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedPlacementActor(PlacementActorEvidenceV1);

impl VerifiedPlacementActor {
    #[must_use]
    pub fn evidence(&self) -> &PlacementActorEvidenceV1 {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlacementPolicyMappingV1 {
    CellPlacementActionResource,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlacementPolicyResourceV1 {
    PlacementOperation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlacementBusinessOriginV1 {
    Direct {
        request_id: String,
    },
    Rebalance {
        partition: RebalanceJobPartitionV1,
        job_id: RebalanceJobId,
        evaluation_id: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementBusinessActionKeyV1 {
    pub origin: PlacementBusinessOriginV1,
    pub action_id: String,
    pub tenant_id: TenantId,
    pub operation: PlacementOperationKey,
    pub action: PlacementActionV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementAuthorizationRequestV1 {
    pub resource: PlacementPolicyResourceV1,
    pub mapping: PlacementPolicyMappingV1,
    pub actor: PlacementActorEvidenceV1,
    pub key: PlacementBusinessActionKeyV1,
    pub purpose: PlacementIntentPurposeV1,
    pub realm: RealmId,
    pub jurisdiction: JurisdictionId,
    pub business_digest: Digest32,
    pub dispatch_digest: Digest32,
    pub canonical_request: Vec<u8>,
    pub audience: ProducerId,
    pub deadline_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementPolicyObligationV1 {
    pub obligation_id: String,
    pub schema_version: u32,
    pub canonical_requirement: Vec<u8>,
    pub canonical_fulfillment: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementPolicyDecisionEvidenceV1 {
    pub request: PlacementAuthorizationRequestV1,
    pub decision_id: String,
    pub policy_version: PolicyVersionToken,
    pub determining_policy_ids: Vec<String>,
    pub obligations: Vec<PlacementPolicyObligationV1>,
    pub canonical_pdp_outcome: Vec<u8>,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementAuthorizationTrustV1 {
    pub identity_producer: ProducerId,
    pub decision_producer: ProducerId,
    pub audience: ProducerId,
    pub custody_configuration_digest: Digest32,
    pub admitted_mapping: PlacementPolicyMappingV1,
    pub exact_policy_version: PolicyVersionToken,
    pub now_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlacementAuthorizationError {
    NotImplemented,
    InvalidIdentity,
    Denied,
    DecisionUnavailable,
    MissingPolicyEvidence,
    UnadmittedMapping,
    UnsupportedObligation,
    UnmetObligation,
    RequestMismatch,
    PolicyFreshnessUnavailable,
    Expired,
    Proof(ProofVerificationError),
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedPlacementPolicyDecision(PlacementPolicyDecisionEvidenceV1);

impl VerifiedPlacementPolicyDecision {
    #[must_use]
    pub fn evidence(&self) -> &PlacementPolicyDecisionEvidenceV1 {
        &self.0
    }
}

pub fn verify_placement_actor(
    _verifier: &dyn CellProofVerifier,
    _evidence: PlacementActorEvidenceV1,
    _trust: &PlacementAuthorizationTrustV1,
) -> Result<VerifiedPlacementActor, PlacementAuthorizationError> {
    Err(PlacementAuthorizationError::NotImplemented)
}

pub fn verify_placement_policy_decision(
    _verifier: &dyn CellProofVerifier,
    _evidence: PlacementPolicyDecisionEvidenceV1,
    _expected: &PlacementAuthorizationRequestV1,
    _trust: &PlacementAuthorizationTrustV1,
) -> Result<VerifiedPlacementPolicyDecision, PlacementAuthorizationError> {
    Err(PlacementAuthorizationError::NotImplemented)
}

pub trait CellPlacementAuthorizer: Send + Sync {
    fn authorize<'a>(
        &'a self,
        actor: &'a VerifiedPlacementActor,
        request: &'a PlacementAuthorizationRequestV1,
    ) -> BoxCellFuture<'a, Result<PlacementPolicyDecisionEvidenceV1, PlacementAuthorizationError>>;
}

#[derive(Debug, Eq, PartialEq)]
pub struct AuthorizedDirectPlacementIssuanceV1 {
    decision: VerifiedPlacementPolicyDecision,
    invocation: PlacementInvocationPayloadV1,
}

impl AuthorizedDirectPlacementIssuanceV1 {
    pub fn assemble(
        _decision: VerifiedPlacementPolicyDecision,
        _invocation: PlacementInvocationPayloadV1,
    ) -> Result<Self, PlacementAuthorizationError> {
        Err(PlacementAuthorizationError::NotImplemented)
    }
    #[must_use]
    pub fn decision(&self) -> &VerifiedPlacementPolicyDecision {
        &self.decision
    }
    #[must_use]
    pub fn invocation(&self) -> &PlacementInvocationPayloadV1 {
        &self.invocation
    }
}

pub trait CellPlacementInvocationIssuer: Send + Sync {
    fn sign_committed<'a>(
        &'a self,
        issuance: &'a VerifiedCommittedRebalanceIssuance,
    ) -> BoxCellFuture<'a, Result<SignedPlacementInvocationV1, PlacementAuthorizationError>>;

    fn sign_direct<'a>(
        &'a self,
        issuance: &'a AuthorizedDirectPlacementIssuanceV1,
    ) -> BoxCellFuture<'a, Result<SignedPlacementInvocationV1, PlacementAuthorizationError>>;
}
