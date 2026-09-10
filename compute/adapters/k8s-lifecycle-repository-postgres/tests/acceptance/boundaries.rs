use compute_k8s_api::*;
use compute_k8s_lifecycle_repository_postgres::PgK8sLifecycleRepository;
use shared_resource_provider_contract_kernel::OperationState;

use super::fixtures::*;

enum Boundary {
    LostAcknowledgement,
    UnsupportedState,
}

struct Transport<'a> {
    inner: &'a PgK8sLifecycleRepository,
    boundary: Boundary,
}

impl CloudComputeK8sAcceptanceRepository for Transport<'_> {
    fn accept_create_intent<'a>(
        &'a self,
        command: CloudComputeK8sAcceptCreateIntentCommand,
    ) -> CloudComputeK8sRepositoryFuture<
        'a,
        Result<CloudComputeK8sOperationSnapshot, CloudComputeK8sAcceptanceRepositoryError>,
    > {
        Box::pin(async move {
            let mut committed = self.inner.accept_create_intent(command).await?;
            match self.boundary {
                Boundary::LostAcknowledgement => {
                    Err(CloudComputeK8sAcceptanceRepositoryError::OutcomeUnknown)
                }
                Boundary::UnsupportedState => {
                    committed.state = OperationState::Running;
                    Ok(committed)
                }
            }
        })
    }

    fn get_create_operation<'a>(
        &'a self,
        query: CloudComputeK8sReadCreateOperationQuery,
    ) -> CloudComputeK8sRepositoryFuture<
        'a,
        Result<CloudComputeK8sOperationLookup, CloudComputeK8sAcceptanceRepositoryError>,
    > {
        Box::pin(async move {
            let mut lookup = self.inner.get_create_operation(query).await?;
            if let (Boundary::UnsupportedState, CloudComputeK8sOperationLookup::Found(snapshot)) =
                (&self.boundary, &mut lookup)
            {
                snapshot.state = OperationState::Succeeded;
            }
            Ok(lookup)
        })
    }
}

pub(super) async fn assert_boundaries(repository: &PgK8sLifecycleRepository) {
    let transport = Transport {
        inner: repository,
        boundary: Boundary::LostAcknowledgement,
    };
    let error = accept(&transport, pending_request("lost-original", "lost-ack"))
        .await
        .unwrap_err();
    assert_eq!(error, CloudComputeK8sAcceptanceApiError::OutcomeUnknown);
    assert_eq!(error.status_code(), 503);
    let lookup = read(repository, operation_read_request("lost-ack"))
        .await
        .unwrap();
    let CloudComputeK8sOperationLookup::Found(original) = lookup else {
        panic!("committed acknowledgement must recover");
    };
    assert_eq!(original.receipt.request_id, "lost-original");
    assert_eq!(original.receipt.operation_key.idempotency_key, "lost-ack");
    assert_eq!(original.state, OperationState::Accepted);
    let retried = accept(repository, pending_request("lost-retry", "lost-ack"))
        .await
        .unwrap();
    assert_eq!(retried.operation, original);

    let unsupported = Transport {
        inner: repository,
        boundary: Boundary::UnsupportedState,
    };
    assert_eq!(
        accept(
            &unsupported,
            pending_request("unsupported-original", "unsupported")
        )
        .await
        .unwrap_err(),
        CloudComputeK8sAcceptanceApiError::IntegrityViolation
    );
    assert_eq!(
        read(&unsupported, operation_read_request("unsupported"))
            .await
            .unwrap_err(),
        CloudComputeK8sAcceptanceApiError::IntegrityViolation
    );
    let unchanged = accept(repository, pending_request("retry", "unsupported"))
        .await
        .unwrap();
    assert_eq!(
        unchanged.operation.receipt.request_id,
        "unsupported-original"
    );
    assert_eq!(unchanged.operation.state, OperationState::Accepted);
}
