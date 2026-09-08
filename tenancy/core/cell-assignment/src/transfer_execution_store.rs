use crate::{
    BindingReadAuthorityV1, BindingReconciliationReadAuthorityV1, BindingStoreError,
    BoxTenancyFuture, CapabilityTransferEffectWriteSetV1, IssueTransferExecutionPermitWriteSetV1,
    PublishTransferExecutionPermitWriteSetV1, RecordTransferExecutionOutcomeWriteSetV1,
    SignedTransferExecutionCommitObservationV1, SignedTransferExecutionPermitV1,
    TransferExecutionLedgerV1, TransferExecutionPermitIssuanceLookupV1,
    TransferExecutionPermitIssuanceRecordV1, TransferExecutionRepairMutationResultV1,
    TransferExecutionRepairWriteSetV1, VerifiedCommittedTransferExecutionPermitIssuance,
    VerifiedTransferExecutionPermit,
};

pub trait TransferExecutionStore: Send + Sync {
    /// Durably records an unsigned transfer-execution permit issuance.
    ///
    /// The returned record never carries a signature. A signed permit is
    /// PRODUCED only along one path — [`TransferExecutionCommitObserver`],
    /// [`crate::verify_committed_transfer_execution_permit_issuance`],
    /// [`TransferExecutionPermitAuthority::sign_committed`], then
    /// [`TransferExecutionStore::publish_permit`] — and this enumeration is of
    /// production, not of reachability. Once published the permit is a durable
    /// field of the item row and is READ BACK by
    /// [`TransferExecutionStore::load_item`]; an earlier version of this
    /// sentence said "only reachable" and was falsified by that row.
    fn issue_permit<'a>(
        &'a self,
        write_set: &'a IssueTransferExecutionPermitWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<TransferExecutionPermitIssuanceRecordV1, BindingStoreError>>;

    /// Re-reads a permit issuance by explicit key, for republication after a
    /// lost reply. Absence distinguishes "never durably issued" from
    /// "issued, reply lost".
    fn load_permit_issuance<'a>(
        &'a self,
        authority: &'a BindingReadAuthorityV1,
        lookup: &'a TransferExecutionPermitIssuanceLookupV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<Option<TransferExecutionPermitIssuanceRecordV1>, BindingStoreError>,
    >;

    fn load_permit_issuance_for_reconciliation<'a>(
        &'a self,
        authority: &'a BindingReconciliationReadAuthorityV1,
        lookup: &'a TransferExecutionPermitIssuanceLookupV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<Option<TransferExecutionPermitIssuanceRecordV1>, BindingStoreError>,
    >;

    /// Publishes the signed permit for an already-committed issuance.
    ///
    /// This store MINTS NOTHING. It receives the already-signed permit as a
    /// write-set input and persists it; the signature is minted by
    /// [`TransferExecutionPermitAuthority::sign_committed`], the neighbouring
    /// port in this file. What this method is the sole path for is
    /// PUBLICATION -- the durable write that makes a minted permit readable --
    /// and the write set can only be assembled from a verified commit
    /// observation.
    ///
    /// Republishing the same immutable issuance is permitted; byte-identical
    /// signatures are not required.
    fn publish_permit<'a>(
        &'a self,
        write_set: &'a PublishTransferExecutionPermitWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<SignedTransferExecutionPermitV1, BindingStoreError>>;

    fn record_outcome<'a>(
        &'a self,
        write_set: &'a RecordTransferExecutionOutcomeWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<TransferExecutionLedgerV1, BindingStoreError>>;

    /// Reads the transfer-execution ledger row for one operation.
    ///
    /// `None` MEANS THE STORE LOOKED AND FOUND NO LEDGER ROW: no permit has
    /// ever been issued for this operation. That is a normal answer and the
    /// exact fact a first `issue_permit` needs, because
    /// [`crate::IssueTransferExecutionPermitWriteSetPartsV1::expected_ledger_revision`]
    /// is `None` in precisely that case and `Some(ledger.revision)` otherwise.
    /// Until both docs existed, that required precondition had no legal value
    /// at a first write and the only read that could supply it said nothing
    /// about what absence asserted.
    ///
    /// `None` is not "the operation does not exist" and not an authorization
    /// refusal; both of those are on the error channel.
    fn get_ledger<'a>(
        &'a self,
        authority: &'a BindingReadAuthorityV1,
        operation: &'a crate::BindingOperationKey,
    ) -> BoxTenancyFuture<'a, Result<Option<TransferExecutionLedgerV1>, BindingStoreError>>;

    /// The reconciliation-authority twin of
    /// [`TransferExecutionStore::get_ledger`]. `None` asserts the same thing.
    fn get_ledger_for_reconciliation<'a>(
        &'a self,
        authority: &'a BindingReconciliationReadAuthorityV1,
        operation: &'a crate::BindingOperationKey,
    ) -> BoxTenancyFuture<'a, Result<Option<TransferExecutionLedgerV1>, BindingStoreError>>;

    /// Reads one transfer-execution item row by its issuance address, under
    /// [`BindingReadAuthorityV1`] -- the read authority a holder of the
    /// [`crate::BindingPersistenceAuthorityV1`] that `issue_permit`, `publish_permit`
    /// and `record_outcome` take DERIVES from
    /// [`crate::BindingPersistenceAuthorityV1::read_authority`].
    ///
    /// An earlier version of this sentence said "the SAME ordinary authority the
    /// issue, publish and outcome writes take", and that was false of the types
    /// as written: those three take `BindingPersistenceAuthorityV1` and this
    /// takes `BindingReadAuthorityV1`, two distinct newtypes; and
    /// [`crate::VerifiedBindingInvocation`] is not `Clone` while both `into_*`
    /// constructors consume `self`, so one verified invocation minted exactly
    /// one of the two. Deriving the second silently meant re-verifying the
    /// caller's own invocation inside the service, which no doc described. The
    /// premise is true now because the ordering is stated in types; it was not
    /// true when it was written.
    ///
    /// `None` MEANS THE STORE LOOKED AND FOUND NO ITEM ROW for that address: no
    /// permit has been issued for that effect. `Some` carries the row's current
    /// `disposition`, `revision`, `record_digest` and its published `permit`.
    ///
    /// TWO THINGS WERE UNREACHABLE WITHOUT THIS READ, and both were reported
    /// independently.
    ///
    /// First, `record_outcome` requires
    /// [`crate::RecordTransferExecutionOutcomeWriteSetPartsV1::item_precondition`]
    /// by value, and after this wave inserted `publish_permit` as a THIRD
    /// writer of the item row, the commit observation the publisher holds
    /// carries pre-publication values. No write on this store returns the item —
    /// `issue_permit` returns the issuance record, `publish_permit` the signed
    /// permit, `record_outcome` the ledger — so the only carrier of the item's
    /// current revision and record digest was
    /// [`TransferExecutionStore::read_item_page_for_reconciliation`], gated on
    /// `BindingReconciliationReadAuthorityV1` PLUS a reconciliation lease. The
    /// ordinary outcome path would have had to restate store-derived values the
    /// publish write set forbids it from restating, or take out a lease the same
    /// doc says that path must not hold. This read is the third option, and the
    /// authority it adds is one the write's own authority already subsumes --
    /// which is a weaker and true claim than the "adds no new authority" this
    /// sentence used to make.
    ///
    /// Second,
    /// [`crate::TenancyMigrationCoordinationService::get_transfer_execution_permit`]
    /// promises three distinguishable states, the third being "issuance
    /// present, permit present — published". The signed permit is durable on
    /// this row, but every other surface that yields one is a write or a mint,
    /// so over the declared store that third state could only have been reached
    /// by re-minting a signature on a read. It is now an ordinary read.
    ///
    /// This does not weaken the enumeration on
    /// [`TransferExecutionStore::issue_permit`]: that names how a permit is
    /// PRODUCED, and this method only reads back what publication durably made
    /// readable.
    fn load_item<'a>(
        &'a self,
        authority: &'a BindingReadAuthorityV1,
        address: &'a crate::TransferExecutionPermitIssuanceAddressV1,
    ) -> BoxTenancyFuture<'a, Result<Option<crate::TransferExecutionItemV1>, BindingStoreError>>;

    fn apply_repair<'a>(
        &'a self,
        write_set: &'a TransferExecutionRepairWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<TransferExecutionRepairMutationResultV1, BindingStoreError>>;

    fn read_item_page_for_reconciliation<'a>(
        &'a self,
        authority: &'a BindingReconciliationReadAuthorityV1,
        reconciliation_lease: &'a crate::BindingReconciliationLeaseV1,
        request: &'a crate::TransferExecutionItemPageRequestV1,
    ) -> BoxTenancyFuture<'a, Result<crate::TransferExecutionItemPageV1, BindingStoreError>>;
}

/// Independently re-reads a committed permit issuance and signs what it read.
///
/// The port accepts a lookup key only. It is not given, and cannot be given, a
/// caller-supplied record or a caller's claim that a commit occurred: every
/// field of the emitted observation describes what the observer itself read
/// under [`crate::TransferExecutionIssuanceReadIsolationV1`].
pub trait TransferExecutionCommitObserver: Send + Sync {
    fn observe_committed_permit_issuance<'a>(
        &'a self,
        authority: &'a BindingReadAuthorityV1,
        lookup: &'a TransferExecutionPermitIssuanceLookupV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<Option<SignedTransferExecutionCommitObservationV1>, BindingStoreError>,
    >;
}

/// Mints the transfer-execution permit signature.
///
/// The input is a private-field verified wrapper, so the caller must have gone
/// through
/// [`crate::verify_committed_transfer_execution_permit_issuance`] to obtain
/// one. That, and not `verify_transfer_execution_permit`, is this port's
/// minter: the INPUT is a
/// [`crate::VerifiedCommittedTransferExecutionPermitIssuance`], while
/// `verify_transfer_execution_permit` mints from this port's OUTPUT after
/// publication and is named correctly in the closing paragraph below. An
/// earlier version of this paragraph reasoned about the wrong one; the
/// conclusion survived, because both verifiers arrive as
/// `&dyn BindingProofVerifier`, but the citation did not — and it RESOLVES, so
/// no rustdoc gate could have seen it.
///
/// THAT IS A DEPLOYMENT OBLIGATION, NOT A TYPE-LEVEL REFUSAL, and this doc
/// previously claimed the stronger thing. The private field refuses DIRECT
/// construction and nothing more: both verifiers take their verifier as
/// `&dyn BindingProofVerifier`, a public single-method trait, so an
/// out-of-crate type can implement that trait and this port together and mint
/// the wrapper by passing ITSELF as the verifier. The barrier holds when the
/// verifier actually deployed verifies; the type system cannot make it.
///
/// The output is the raw signed value, NOT a verified wrapper. A signing
/// adapter lives outside this crate and cannot construct a private-field
/// wrapper DIRECTLY, so returning one would leave self-verification — the
/// signer checking its own signature against an expectation it built itself —
/// as the only implementable shape, which gates nothing. That reasoning is
/// unaffected by the correction above, and is its sharpest instance:
/// self-verification is exactly the route the deployment obligation must
/// exclude. The caller mints the wrapper
/// by passing this value through
/// [`crate::verify_transfer_execution_permit`].
pub trait TransferExecutionPermitAuthority: Send + Sync {
    fn sign_committed<'a>(
        &'a self,
        issuance: &'a VerifiedCommittedTransferExecutionPermitIssuance,
    ) -> BoxTenancyFuture<'a, Result<SignedTransferExecutionPermitV1, BindingStoreError>>;
}

pub trait CapabilityTransferEffectStore: Send + Sync {
    fn consume_before_effect<'a>(
        &'a self,
        write_set: &'a CapabilityTransferEffectWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<crate::BindingDigest32, BindingStoreError>>;
}
