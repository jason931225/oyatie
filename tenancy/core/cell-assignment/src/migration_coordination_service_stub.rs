use crate::{
    AppendParticipantReceiptRequestV1, AppendTransferAuthorizationRequestV1, BindingContractError,
    BoxTenancyFuture, CloseParticipantPhaseRequestV1, GetTransferExecutionPermitRequestV1,
    IssueSourceFenceDirectiveRequestV1, IssueTransferExecutionPermitRequestV1,
    PublishTransferExecutionPermitRequestV1, PutParticipantManifestRequestV1,
    PutParticipantManifestResultV1, RecordTransferExecutionOutcomeRequestV1,
    SealTransferAuthorizationSetRequestV1, SignedParticipantPhaseClosureV1,
    SignedResidencyTransferAuthorizationSetV1, SignedTransferExecutionOutcomeV1,
    SignedTransferExecutionPermitV1, SourceFenceDirectiveIssueResultV1,
    TenancyMigrationCoordinationService, TransferAuthorizationJournalV1,
    TransferExecutionPermitGetResultV1, TransferExecutionPermitIssueResultV1,
    VerifiedBindingInvocation,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct NotImplementedTenancyMigrationCoordinationService;

fn not_implemented<'a, T>() -> BoxTenancyFuture<'a, Result<T, BindingContractError>> {
    Box::pin(async { Err(BindingContractError::NotImplemented) })
}

impl TenancyMigrationCoordinationService for NotImplementedTenancyMigrationCoordinationService {
    fn put_participant_manifest<'a>(
        &'a self,
        _: VerifiedBindingInvocation,
        _: PutParticipantManifestRequestV1,
    ) -> BoxTenancyFuture<'a, Result<PutParticipantManifestResultV1, BindingContractError>> {
        not_implemented()
    }

    fn append_participant_receipt<'a>(
        &'a self,
        _: VerifiedBindingInvocation,
        _: AppendParticipantReceiptRequestV1,
    ) -> BoxTenancyFuture<'a, Result<crate::ParticipantReceiptLedgerV1, BindingContractError>> {
        not_implemented()
    }

    fn close_participant_phase<'a>(
        &'a self,
        _: VerifiedBindingInvocation,
        _: CloseParticipantPhaseRequestV1,
    ) -> BoxTenancyFuture<'a, Result<SignedParticipantPhaseClosureV1, BindingContractError>> {
        not_implemented()
    }

    fn append_transfer_authorization<'a>(
        &'a self,
        _: VerifiedBindingInvocation,
        _: AppendTransferAuthorizationRequestV1,
    ) -> BoxTenancyFuture<'a, Result<TransferAuthorizationJournalV1, BindingContractError>> {
        not_implemented()
    }

    fn seal_transfer_authorization_set<'a>(
        &'a self,
        _: VerifiedBindingInvocation,
        _: SealTransferAuthorizationSetRequestV1,
    ) -> BoxTenancyFuture<'a, Result<SignedResidencyTransferAuthorizationSetV1, BindingContractError>>
    {
        not_implemented()
    }

    fn issue_transfer_execution_permit<'a>(
        &'a self,
        _: VerifiedBindingInvocation,
        _: IssueTransferExecutionPermitRequestV1,
    ) -> BoxTenancyFuture<'a, Result<TransferExecutionPermitIssueResultV1, BindingContractError>>
    {
        not_implemented()
    }

    fn publish_transfer_execution_permit<'a>(
        &'a self,
        _: VerifiedBindingInvocation,
        _: PublishTransferExecutionPermitRequestV1,
    ) -> BoxTenancyFuture<'a, Result<SignedTransferExecutionPermitV1, BindingContractError>> {
        not_implemented()
    }

    fn get_transfer_execution_permit<'a>(
        &'a self,
        _: VerifiedBindingInvocation,
        _: GetTransferExecutionPermitRequestV1,
    ) -> BoxTenancyFuture<'a, Result<TransferExecutionPermitGetResultV1, BindingContractError>>
    {
        not_implemented()
    }

    fn record_transfer_execution_outcome<'a>(
        &'a self,
        _: VerifiedBindingInvocation,
        _: RecordTransferExecutionOutcomeRequestV1,
    ) -> BoxTenancyFuture<'a, Result<SignedTransferExecutionOutcomeV1, BindingContractError>> {
        not_implemented()
    }

    fn issue_source_fence_directive<'a>(
        &'a self,
        _: VerifiedBindingInvocation,
        _: IssueSourceFenceDirectiveRequestV1,
    ) -> BoxTenancyFuture<'a, Result<SourceFenceDirectiveIssueResultV1, BindingContractError>> {
        not_implemented()
    }
}
