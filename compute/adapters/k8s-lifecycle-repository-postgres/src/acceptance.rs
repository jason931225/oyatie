use compute_k8s_api::{
    CloudComputeK8sAcceptCreateIntentCommand, CloudComputeK8sAcceptanceContract,
    CloudComputeK8sAcceptanceRepository, CloudComputeK8sAcceptanceRepositoryError as Error,
    CloudComputeK8sAcceptedCreateIntent, CloudComputeK8sOperationLookup,
    CloudComputeK8sOperationSnapshot, CloudComputeK8sReadCreateOperationQuery,
    CloudComputeK8sRepositoryFuture, cloud_compute_k8s_create_intent_fingerprint,
    validate_cloud_compute_k8s_operation_snapshot,
};
use shared_resource_provider_contract_kernel::OperationState;

use crate::{
    PgK8sLifecycleRepository,
    acceptance_integrity::{validate_key, verified_snapshot},
    acceptance_storage::{begin, complete, reserve, select, unavailable},
};

impl CloudComputeK8sAcceptanceRepository for PgK8sLifecycleRepository {
    fn accept_create_intent<'a>(
        &'a self,
        command: CloudComputeK8sAcceptCreateIntentCommand,
    ) -> CloudComputeK8sRepositoryFuture<'a, Result<CloudComputeK8sOperationSnapshot, Error>> {
        Box::pin(async move {
            validate_key(&command.operation_key)?;
            if command.request_id.trim().is_empty()
                || command.intent.tenant_id != command.operation_key.tenant_id
            {
                return Err(Error::IntegrityViolation);
            }
            let fingerprint = cloud_compute_k8s_create_intent_fingerprint(&command.intent)
                .map_err(|_| Error::IntegrityViolation)?;
            let mut tx = begin(&self.pool, &command.operation_key.tenant_id, false).await?;
            let accepted_at = reserve(&mut tx, &command, &fingerprint).await?;
            let snapshot = if let Some(accepted_at_epoch_seconds) = accepted_at {
                let snapshot = CloudComputeK8sOperationSnapshot {
                    receipt: CloudComputeK8sAcceptedCreateIntent {
                        request_contract: CloudComputeK8sAcceptanceContract::PendingIntent,
                        operation_key: command.operation_key.clone(),
                        intent: command.intent.clone(),
                        request_id: command.request_id,
                        accepted_at_epoch_seconds,
                    },
                    state: OperationState::Accepted,
                };
                validate_cloud_compute_k8s_operation_snapshot(
                    &snapshot,
                    &command.operation_key,
                    &command.intent.resource_id,
                    Some(&command.intent),
                )?;
                complete(&mut tx, &snapshot.receipt).await?;
                snapshot
            } else {
                let row = select(&mut tx, &command.operation_key, true)
                    .await?
                    .ok_or(Error::IntegrityViolation)?;
                let snapshot = verified_snapshot(&row, &command.operation_key, None)?;
                if cloud_compute_k8s_create_intent_fingerprint(&snapshot.receipt.intent)
                    .map_err(|_| Error::IntegrityViolation)?
                    != fingerprint
                {
                    return Err(Error::IdempotencyKeyReused);
                }
                snapshot
            };
            tx.commit().await.map_err(|_| Error::OutcomeUnknown)?;
            Ok(snapshot)
        })
    }

    fn get_create_operation<'a>(
        &'a self,
        query: CloudComputeK8sReadCreateOperationQuery,
    ) -> CloudComputeK8sRepositoryFuture<'a, Result<CloudComputeK8sOperationLookup, Error>> {
        Box::pin(async move {
            validate_key(&query.operation_key)?;
            if query.resource_id.trim().is_empty() {
                return Err(Error::IntegrityViolation);
            }
            let mut tx = begin(&self.pool, &query.operation_key.tenant_id, true).await?;
            let row = select(&mut tx, &query.operation_key, false).await?;
            let lookup = match row {
                Some(row) => CloudComputeK8sOperationLookup::Found(verified_snapshot(
                    &row,
                    &query.operation_key,
                    Some(&query.resource_id),
                )?),
                None => CloudComputeK8sOperationLookup::NotObserved,
            };
            tx.rollback().await.map_err(unavailable)?;
            Ok(lookup)
        })
    }
}
