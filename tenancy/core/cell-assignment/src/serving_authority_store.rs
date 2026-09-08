use crate::{
    BoxTenancyFuture, ServingAuthorityFreezeResultPayloadV1, ServingAuthorityFreezeWriteSetV1,
    ServingAuthorityInstallationResultPayloadV1, ServingAuthorityInstallationWriteSetV1,
    SignedServingAuthorityFreezeResultV1, SignedServingAuthorityInstallationResultV1,
};

/// Failures of the serving-authority store.
///
/// An implementation MUST return the most specific variant that applies;
/// [`Self::Conflict`] is the residual for precondition failures no other
/// variant names. Two variants never describe the same situation — where two
/// could plausibly apply, the boundary is stated on both, and any ordering
/// between them that matters is stated there too (for example
/// [`Self::ScopeMismatch`] is evaluated before any precondition, so it never
/// competes with [`Self::Conflict`]).
///
/// DECLARATION ORDER CARRIES NO MEANING. An earlier version of this header
/// claimed the variants ran from most specific to least; they did not, and the
/// residual sat third in the list. The claim is dropped rather than the list
/// resorted, because specificity here is a partial order and not a total one:
/// `PageLimitExceeded` and `StaleIncarnation` are not comparable, and
/// "evaluated first" and "most specific" are different axes that the old
/// wording conflated. Asserting a total order invites a future editor to
/// restore an order that cannot exist. What is enforceable is the MUST above
/// and the per-variant boundaries below.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServingAuthorityStoreError {
    /// No implementation exists yet. Every method of this contract returns this
    /// today; it is not a runtime condition and carries no information about
    /// durable state.
    NotImplemented,
    /// The store could not be reached, or could not answer. The caller learns
    /// NOTHING about whether the write took effect and must re-read to find
    /// out. Contrast [`Self::RetainedEvidenceUnavailable`], where the store did
    /// answer and the answer was that a required record is gone.
    Unavailable,
    /// A precondition in the write set did not match the durable record, and no
    /// more specific variant below applies. This is the residual
    /// compare-and-set failure on a `revision` or `record_digest`. Retrying
    /// after a fresh read may succeed. Contrast [`Self::StaleIncarnation`],
    /// where retrying with the same request can never succeed.
    Conflict,
    /// The invocation does not name the record the request targets: its
    /// `instance` or its `ServingAuthorityActionV1` addresses something other
    /// than the write set's subject. Evaluated BEFORE any precondition, so it
    /// never competes with [`Self::Conflict`] — a scope mismatch means the
    /// preconditions were never examined.
    ScopeMismatch,
    /// The request's [`crate::ServingAuthorityIncarnationV1`] is older than the
    /// durable one: a newer incarnation of this partition has taken over.
    /// Terminal for this request — unlike [`Self::Conflict`], re-reading and
    /// retrying with the same incarnation can never succeed.
    StaleIncarnation,
    /// The local precondition is
    /// [`crate::ServingAuthorityLocalPreconditionV1::Rejected`]: this instance
    /// is recorded in the rejection high water and may never install. Terminal
    /// for this instance. Contrast [`Self::FrozenAuthority`], where the
    /// authority did install and later stopped serving.
    RejectedInstallation,
    /// The authority is installed but frozen, so writes are refused until it is
    /// replaced. Distinguished from [`Self::RejectedInstallation`] by history:
    /// a frozen authority was installed; a rejected one never was.
    FrozenAuthority,
    /// A publication step was asked to act on an issuance whose commit is not
    /// durably observable. Distinguished from [`Self::Unavailable`]: the store
    /// answered, and the answer is that the issuance is not committed — a
    /// definite negative, not an unknown.
    UncommittedIssuance,
    /// The `idempotency_key` of [`crate::ServingAuthorityBusinessIdV1`] was
    /// already seen carrying a DIFFERENT `request_digest`. Keyed on the
    /// request. Contrast [`Self::ProofAlreadyApplied`], which is keyed on a
    /// proof and can fire even when the request is byte-identical.
    IdempotencyKeyReuse,
    /// A [`crate::ServingAuthorityProofConsumptionV1`] named by the write set
    /// has already been consumed. Keyed on the proof, not the request.
    ProofAlreadyApplied,
    /// Installation is refused because the prior authority's terminal closure
    /// or effect-path fencing is present but not durably complete. Contrast
    /// [`Self::RestoreEvidenceRequired`]: here the evidence exists and is
    /// insufficient; there it is absent.
    IncompletePriorAuthorityClosure,
    /// Installation is refused because no
    /// [`crate::ServingAuthorityRestoreBasisV1`] was supplied where the
    /// contract requires one. Absence of evidence, not incomplete evidence.
    RestoreEvidenceRequired,
    /// A caller-proposed successor restates a value this store OWNS, and the
    /// restatement disagrees with what the store derives.
    ///
    /// Successor records carry owner-authoritative fields --
    /// `binding_generation`, `binding_revision`, `binding_record_digest` and
    /// `write_authority_epoch`, all inside
    /// [`crate::ServingAuthorityInstanceV1`] -- alongside the fields the caller
    /// legitimately chooses. The store derives the owned ones itself and MUST
    /// refuse rather than accept or silently overwrite a disagreeing proposal.
    ///
    /// Distinct from [`Self::Conflict`]: a conflict is a precondition failing
    /// against durable state, and re-reading may resolve it. This is the
    /// caller's proposal contradicting a value it does not own, and re-reading
    /// resolves it only if the caller then stops restating the value.
    ProposedSuccessorMismatch,
    /// The issuance's publication lease is held by another worker whose lease
    /// has NOT expired against the claimant's `now_unix_seconds`.
    ///
    /// Distinct from [`Self::Conflict`], which contention previously collapsed
    /// into: a conflict invites a re-read and retry, whereas this says another
    /// worker legitimately holds the work and the caller should stop.
    ///
    /// This variant carries no payload, so it deliberately does NOT tell the
    /// caller when to come back -- an error that names a value it cannot carry
    /// is worse than one that names none. The expiry is obtained by reading:
    /// [`crate::ServingAuthorityPublicationReconciliationStore::get_publication_lease`]
    /// returns the held lease including its `expires_at_unix_seconds`.
    PublicationLeaseHeldByAnotherWorker,
    /// A page request asked for more records, or more encoded bytes, than the
    /// store will return in one response. The caller should re-request with a
    /// smaller bound; the durable state is unchanged.
    PageLimitExceeded,
    /// Evidence the contract requires to remain readable is no longer
    /// retrievable. Durable and non-retryable, unlike [`Self::Unavailable`].
    ///
    /// NOTE: this variant names no type elsewhere in this crate. Its is the
    /// weakest boundary in this enum and should be re-stated by whoever
    /// introduces the retention contract it refers to.
    RetainedEvidenceUnavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthorityResultQueryV1 {
    pub instance: crate::ServingAuthorityInstanceV1,
    pub business: crate::ServingAuthorityBusinessIdV1,
}

pub trait CellServingAuthorityStore: Send + Sync {
    /// Durably installs, returning the UNSIGNED result payload.
    ///
    /// A store must not mint a signature over the write it just performed: it
    /// would be vouching for itself. The signed
    /// [`SignedServingAuthorityInstallationResultV1`] is produced instead by
    /// [`ServingAuthorityResultObserver::observe_installation_result`], which
    /// re-reads the committed row by lookup key.
    fn install<'a>(
        &'a self,
        write_set: &'a ServingAuthorityInstallationWriteSetV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<ServingAuthorityInstallationResultPayloadV1, ServingAuthorityStoreError>,
    >;

    /// Durably freezes, returning the UNSIGNED result payload. See
    /// [`Self::install`] for why this is not signed here; the freeze result is
    /// the evidence on which a source is declared fenced, so it matters more
    /// here than anywhere else that the writer is not the voucher.
    fn freeze<'a>(
        &'a self,
        write_set: &'a ServingAuthorityFreezeWriteSetV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<ServingAuthorityFreezeResultPayloadV1, ServingAuthorityStoreError>,
    >;

    fn renew_write_authority_lease<'a>(
        &'a self,
        write_set: &'a crate::WriteAuthorityLeaseRenewalWriteSetV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<crate::WriteAuthorityLeaseRenewalResultV1, ServingAuthorityStoreError>,
    >;

    fn publish_write_authority_lease<'a>(
        &'a self,
        write_set: &'a crate::WriteAuthorityLeasePublicationWriteSetV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<crate::WriteAuthorityLeasePublicationResultV1, ServingAuthorityStoreError>,
    >;

    /// Reads back the UNSIGNED durable lease issuance record, or `None` when
    /// none is committed for that query.
    ///
    /// This returns the record rather than
    /// [`crate::CommittedWriteAuthorityLeaseIssuanceClaimV1`] because no write
    /// set writes the claim. Applying the per-record test: the claim's other
    /// member is a signed commit attestation, and
    /// `WriteAuthorityLeasePublicationWriteSetPartsV1` takes `committed_issuance`
    /// as an INPUT while writing `published_issuance`, a
    /// [`crate::WriteAuthorityLeaseIssuanceRecordV1`]. The attestation is
    /// consumed by publication, never stored as the record this getter returns.
    ///
    /// That matters most exactly where this getter is used. Its purpose is the
    /// pre-publication window -- finding an issuance that committed but was
    /// never published -- and in that window no attestation exists at all, so a
    /// claim-typed getter could only have been satisfied by the store minting
    /// one. The caller pairs this record with a fresh attestation from
    /// [`crate::WriteAuthorityLeaseCommitObserver::observe_lease_commit`],
    /// assembles the claim and verifies it.
    fn load_committed_write_authority_lease_issuance<'a>(
        &'a self,
        authority: &'a crate::ServingAuthorityReadAuthorityV1,
        query: &'a crate::CommittedWriteAuthorityLeaseIssuanceQueryV1,
        lease: &'a crate::ServingAuthorityPublicationLeaseV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<Option<crate::WriteAuthorityLeaseIssuanceRecordV1>, ServingAuthorityStoreError>,
    >;

    fn get_lease_state<'a>(
        &'a self,
        authority: &'a crate::ServingAuthorityReadAuthorityV1,
        instance: &'a crate::ServingAuthorityInstanceV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<Option<crate::WriteAuthorityLeaseStateV1>, ServingAuthorityStoreError>,
    >;

    fn get_latest_published_lease<'a>(
        &'a self,
        authority: &'a crate::ServingAuthorityReadAuthorityV1,
        instance: &'a crate::ServingAuthorityInstanceV1,
        minimum_valid_until_unix_seconds: u64,
    ) -> BoxTenancyFuture<
        'a,
        Result<Option<crate::PublishedWriteAuthorityLeaseV1>, ServingAuthorityStoreError>,
    >;

    /// Reads back the rejection high water for an instance's partition, or
    /// `None` when none has been recorded.
    ///
    /// Every mutation on this store takes a
    /// [`crate::ServingAuthorityRejectionHighWaterV1`] as a CAS precondition --
    /// it is a member of all three arms of
    /// [`crate::ServingAuthorityLocalPreconditionV1`], and installation also
    /// supplies a `next_rejection_high_water` -- and nothing returned one. The
    /// type was write-only, which left
    /// [`ServingAuthorityStoreError::Conflict`]'s stated recovery,
    /// "retrying after a fresh read may succeed", naming a read that did not
    /// exist. A stated recovery that cannot be performed is worse than an
    /// unstated one.
    ///
    /// READABLE IS NOT THE SAME AS PRESENT, and this method alone did not make
    /// a first `Install` assemblable. A never-rejected partition has no row for
    /// this read to return, and
    /// `ServingAuthorityLocalPreconditionV1::Uninstalled` required the value by
    /// value. Both halves are needed: this getter, and the `Option` on that
    /// arm. An earlier version of this comment claimed the gap closed while the
    /// second half was still missing.
    fn get_rejection_high_water<'a>(
        &'a self,
        authority: &'a crate::ServingAuthorityReadAuthorityV1,
        instance: &'a crate::ServingAuthorityInstanceV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<Option<crate::ServingAuthorityRejectionHighWaterV1>, ServingAuthorityStoreError>,
    >;

    /// Reads back the UNSIGNED durable installation result.
    ///
    /// This getter cannot return the signed form, because no signed form is
    /// ever in this store's durable state: `install` writes
    /// [`ServingAuthorityInstallationWriteSetV1`], whose `result` member is the
    /// unsigned payload, and no other write path carries a signed result. A
    /// getter typed to the signed form would have been either dead or
    /// self-vouching -- only the store itself could have produced what it
    /// claimed to be reading back.
    ///
    /// The signed form is produced on demand by
    /// [`ServingAuthorityResultObserver::observe_installation_result`], which
    /// re-reads this same row under its own authority.
    fn get_installation_result<'a>(
        &'a self,
        authority: &'a crate::ServingAuthorityReadAuthorityV1,
        query: &'a ServingAuthorityResultQueryV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<Option<ServingAuthorityInstallationResultPayloadV1>, ServingAuthorityStoreError>,
    >;

    /// Reads back the UNSIGNED durable freeze result. See
    /// [`Self::get_installation_result`] for why this is not the signed form;
    /// the signed form comes from
    /// [`ServingAuthorityResultObserver::observe_freeze_result`].
    fn get_freeze_result<'a>(
        &'a self,
        authority: &'a crate::ServingAuthorityReadAuthorityV1,
        query: &'a ServingAuthorityResultQueryV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<Option<ServingAuthorityFreezeResultPayloadV1>, ServingAuthorityStoreError>,
    >;
}

/// Independently observes a committed serving-authority control result and
/// signs what it read.
///
/// Separate from [`CellServingAuthorityStore`] on purpose: the party that
/// performed the write must not be the party that attests it committed. Both
/// methods accept a read authority and a lookup key only — never a caller's
/// result, and never the write set — so the observation can only describe a row
/// the observer itself re-read.
///
/// Neither method needs a new proof domain:
/// `BindingProofDomainV1::ServingAuthorityInstallationResult` and
/// `::ServingAuthorityFreezeResult` already exist, as do the verifiers and the
/// `VerifiedBindingProofRefV1` arms. Only the producer was missing.
/// Both methods return `None` when the row named by the lookup does not
/// exist. An observer signs what it read; when there is nothing to read
/// there is nothing to sign, and that is a normal answer rather than an
/// error -- the same rule already applied to every store lookup on this
/// lane, and `get_installation_result` takes the very same query.
pub trait ServingAuthorityResultObserver: Send + Sync {
    fn observe_installation_result<'a>(
        &'a self,
        authority: &'a crate::ServingAuthorityReadAuthorityV1,
        query: &'a ServingAuthorityResultQueryV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<Option<SignedServingAuthorityInstallationResultV1>, ServingAuthorityStoreError>,
    >;

    fn observe_freeze_result<'a>(
        &'a self,
        authority: &'a crate::ServingAuthorityReadAuthorityV1,
        query: &'a ServingAuthorityResultQueryV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<Option<SignedServingAuthorityFreezeResultV1>, ServingAuthorityStoreError>,
    >;
}
