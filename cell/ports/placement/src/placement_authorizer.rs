use crate::*;

/// The actor attestation's signed content.
///
/// `valid_from_unix_seconds` and `valid_until_unix_seconds` are the
/// credential's own validity window, which is not the proof envelope's
/// `issued_at`/`expires_at`; the two are checked separately.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementActorPayloadV1 {
    pub trust_domain: String,
    pub subject: String,
    pub credential_id: String,
    pub valid_from_unix_seconds: u64,
    pub valid_until_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedPlacementActorV1 {
    pub payload: PlacementActorPayloadV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedPlacementActor(SignedPlacementActorV1);

impl VerifiedPlacementActor {
    #[must_use]
    pub fn signed(&self) -> &SignedPlacementActorV1 {
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
    pub actor: SignedPlacementActorV1,
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

/// The policy decision's signed content: the exact request the policy decision
/// point evaluated, and what it decided.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementPolicyDecisionPayloadV1 {
    pub request: PlacementAuthorizationRequestV1,
    pub decision_id: String,
    pub policy_version: PolicyVersionToken,
    pub determining_policy_ids: Vec<String>,
    pub obligations: Vec<PlacementPolicyObligationV1>,
    pub canonical_pdp_outcome: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedPlacementPolicyDecisionV1 {
    pub payload: PlacementPolicyDecisionPayloadV1,
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

/// Every way placement authorization refuses.
///
/// Five surfaces return this type and they do not all reach every variant:
/// [`verify_placement_actor`] judges one actor attestation against
/// [`PlacementAuthorizationTrustV1`]; [`verify_placement_policy_decision`]
/// judges one already-obtained decision against an expected request and the
/// same trust; [`CellPlacementAuthorizer::authorize`] is the only surface that
/// talks to a policy decision point; and
/// [`AuthorizedDirectPlacementIssuanceV1::assemble`] and
/// [`CellPlacementInvocationIssuer`] relate already-verified values. Each
/// variant below names the surface that raises it and the sibling it is not.
///
/// Where two checks could both fire on one input, the earlier of these wins,
/// so the refusal an operator sees does not depend on implementation order:
///
/// 1. `Proof` — envelope and signature, before any field is read as meaningful
/// 2. `InvalidIdentity`, `ActorCredentialNotYetValid`, `ActorCredentialExpired`
/// 3. `UnadmittedMapping`
/// 4. `RequestMismatch`
/// 5. `PolicyVersionMismatch`
/// 6. `NoDeterminingPolicy`
/// 7. `UnsupportedObligation`, then `UnmetObligation`
/// 8. `Denied`
/// 9. `RequestDeadlinePassed`
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlacementAuthorizationError {
    NotImplemented,

    /// The actor attestation is cryptographically sound but its attested
    /// identity is not one this trust anchor admits: `trust_domain`, `subject`
    /// or `credential_id` is not issued by
    /// [`PlacementAuthorizationTrustV1::identity_producer`].
    ///
    /// Not `Proof`, which is signature and envelope failure and is checked
    /// first. Not `Denied`: nothing has been submitted to a policy decision
    /// point yet, and an admitted actor can still be denied. Raised only by
    /// [`verify_placement_actor`].
    InvalidIdentity,

    /// [`PlacementActorPayloadV1::valid_from_unix_seconds`] is later than
    /// [`PlacementAuthorizationTrustV1::now_unix_seconds`]. The credential is
    /// well formed and admitted; it has not started.
    ///
    /// This exists because the payload declares a validity window with two
    /// ends. Refusing a not-yet-valid credential as `InvalidIdentity` would
    /// report a permanent admission failure for a condition that resolves on
    /// its own. Raised only by [`verify_placement_actor`].
    ActorCredentialNotYetValid,

    /// [`PlacementActorPayloadV1::valid_until_unix_seconds`] is at or before
    /// [`PlacementAuthorizationTrustV1::now_unix_seconds`].
    ///
    /// This is the credential's OWN window, which is not the attestation
    /// envelope's `expires_at_unix_seconds`: a credential can sit inside its
    /// window under an expired envelope, and a live envelope can carry a
    /// credential whose window has closed. Envelope expiry is
    /// `Proof(ProofVerificationError::Expired)` and is checked first. Raised
    /// only by [`verify_placement_actor`].
    ActorCredentialExpired,

    /// [`PlacementAuthorizationRequestV1::deadline_unix_seconds`] is at or
    /// before [`PlacementAuthorizationTrustV1::now_unix_seconds`]: authorizing
    /// this request could only produce an invocation that is already too late
    /// to dispatch.
    ///
    /// A third distinct clock, separate from both the actor credential window
    /// and the proof envelope. It is checked last, so a request that is also
    /// wrong on its merits reports the substantive refusal rather than the
    /// deadline.
    RequestDeadlinePassed,

    /// A policy decision point evaluated the request and returned deny.
    ///
    /// The only variant that reports a policy OUTCOME. Every other variant
    /// means authorization did not get a usable answer, or got one that failed
    /// a check the authorizer owns. A decision that permits but leaves an
    /// obligation unfulfilled is `UnmetObligation`, never this.
    Denied,

    /// The policy decision point could not be reached, or reached and would
    /// not answer. No decision exists.
    ///
    /// Raised only by [`CellPlacementAuthorizer::authorize`].
    /// [`verify_placement_policy_decision`] takes the decision by value and so
    /// can never raise it. Not `Denied`: nothing evaluated. Not
    /// `NoDeterminingPolicy`: no decision arrived at all, rather than one
    /// arriving that fails to describe itself. Retrying may succeed.
    DecisionUnavailable,

    /// A decision arrived and permits, but
    /// [`PlacementPolicyDecisionPayloadV1::determining_policy_ids`] is empty:
    /// it does not name the policies that produced it, so the permit cannot be
    /// audited or replayed against a policy set.
    ///
    /// The decision itself is present and signed; it is the decision's
    /// self-description that is absent. The name says exactly that: no
    /// determining policy is named. It is deliberately not "missing evidence",
    /// which would assert an absence this surface cannot observe -
    /// [`verify_placement_policy_decision`] takes the decision BY VALUE, so a
    /// decision is always present at the only site that raises this. Contrast
    /// `DecisionUnavailable`, where no decision arrived at all.
    NoDeterminingPolicy,

    /// [`PlacementAuthorizationRequestV1::mapping`] is not
    /// [`PlacementAuthorizationTrustV1::admitted_mapping`]: the request asks to
    /// be judged under a resource mapping this trust anchor does not admit.
    ///
    /// Checked BEFORE `RequestMismatch`, deliberately. A request whose mapping
    /// is inadmissible is always this and never `RequestMismatch`, even when it
    /// also differs from the expected request in other fields, because an
    /// inadmissible mapping is a configuration fault while a mismatch is a
    /// substitution.
    UnadmittedMapping,

    /// The request embedded in the decision payload is not equal to the request
    /// the caller expected, on some field other than `mapping`.
    ///
    /// This is the anti-substitution check: it catches a genuine decision for a
    /// different actor, action, tenant, purpose, realm, jurisdiction, digest,
    /// audience or deadline being presented for this one. See `UnadmittedMapping`
    /// for the `mapping` field, which is checked first. Raised only by
    /// [`verify_placement_policy_decision`].
    RequestMismatch,

    /// [`PlacementPolicyDecisionPayloadV1::policy_version`] is not
    /// [`PlacementAuthorizationTrustV1::exact_policy_version`].
    ///
    /// Both values are required and present at every raise site, so this
    /// condition is always determinate. It replaces a variant that named an
    /// availability failure the type structurally cannot have: there is no
    /// freshness oracle here, only an exact version the caller pins. A policy
    /// decision point that cannot state the version it evaluated under fails
    /// earlier, as `DecisionUnavailable`. Raised only by
    /// [`verify_placement_policy_decision`].
    PolicyVersionMismatch,

    /// An obligation on the decision carries an `obligation_id` or
    /// `schema_version` this authorizer does not implement, so whether it is
    /// satisfied cannot be decided at all.
    ///
    /// Checked before `UnmetObligation` and never collapsed into it: an
    /// unknown obligation must never be reported as an unsatisfied one,
    /// because "we cannot tell" and "we checked and it failed" call for
    /// different operator responses.
    UnsupportedObligation,

    /// A supported obligation's `canonical_fulfillment` does not satisfy its
    /// `canonical_requirement`.
    ///
    /// The obligation was understood and evaluated. Contrast
    /// `UnsupportedObligation`, which was not understood, and `Denied`, which
    /// is the decision point's own verdict rather than the authorizer's check
    /// of an obligation attached to a permit.
    UnmetObligation,

    /// Envelope and signature failure on either the actor attestation or the
    /// policy decision: wrong domain, wrong producer, wrong audience, wrong
    /// key, rejected signature, payload digest mismatch, or an envelope
    /// outside its own validity window.
    ///
    /// Checked first on every signed input, so no variant above ever fires on
    /// a payload whose signature has not already been accepted. Envelope
    /// expiry belongs here; credential expiry is `ActorCredentialExpired` and
    /// request expiry is `RequestDeadlinePassed`.
    Proof(ProofVerificationError),
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedPlacementPolicyDecision(SignedPlacementPolicyDecisionV1);

impl VerifiedPlacementPolicyDecision {
    #[must_use]
    pub fn signed(&self) -> &SignedPlacementPolicyDecisionV1 {
        &self.0
    }
}

pub fn verify_placement_actor(
    _verifier: &dyn CellProofVerifier,
    _signed: SignedPlacementActorV1,
    _trust: &PlacementAuthorizationTrustV1,
) -> Result<VerifiedPlacementActor, PlacementAuthorizationError> {
    Err(PlacementAuthorizationError::NotImplemented)
}

pub fn verify_placement_policy_decision(
    _verifier: &dyn CellProofVerifier,
    _signed: SignedPlacementPolicyDecisionV1,
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
    ) -> BoxCellFuture<'a, Result<SignedPlacementPolicyDecisionV1, PlacementAuthorizationError>>;
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
