use crate::{OPERATION_NAME_PREFIX, OperationResult, ProviderError, ResourceProvider};

use super::{ConformanceFixture, ConformanceViolation, MAX_OPERATION_POLLS, violation};

pub async fn check_async_delete_operation<F: ConformanceFixture>(
    fixture: &F,
) -> Result<(), ConformanceViolation> {
    const CHECK: &str = "async_operation";
    let mut provider = fixture.fresh_provider();
    let name = fixture.resource_name(1)?;
    provider
        .create(
            &name,
            fixture.resource_payload(1),
            &fixture.idempotency_key(1)?,
        )
        .await
        .map_err(|e| violation(CHECK, format!("seeding create failed: {e}")))?;

    let delete_key = fixture.idempotency_key(2)?;
    let mut operation = provider
        .delete(&name, &delete_key)
        .await
        .map_err(|e| violation(CHECK, format!("delete failed: {e}")))?;
    if !operation.name.starts_with(OPERATION_NAME_PREFIX) {
        return Err(violation(
            CHECK,
            format!(
                "operation name {:?} lacks the {OPERATION_NAME_PREFIX:?} prefix",
                operation.name
            ),
        ));
    }
    operation
        .validate()
        .map_err(|e| violation(CHECK, format!("operation shape invalid: {e}")))?;

    let operation_name = operation.name.clone();
    let mut polls = 0;
    while !operation.done {
        if polls >= MAX_OPERATION_POLLS {
            return Err(violation(
                CHECK,
                format!("operation did not reach done within {MAX_OPERATION_POLLS} polls"),
            ));
        }
        polls += 1;
        operation = provider
            .poll_operation(&operation_name)
            .await
            .map_err(|e| violation(CHECK, format!("poll failed: {e}")))?;
        operation
            .validate()
            .map_err(|e| violation(CHECK, format!("polled operation shape invalid: {e}")))?;
    }
    match &operation.result {
        Some(OperationResult::Response(_)) => {}
        other => {
            return Err(violation(
                CHECK,
                format!("successful delete must carry a response result, got {other:?}"),
            ));
        }
    }

    match provider.get(&name).await {
        Err(ProviderError::NotFound { .. }) => {}
        other => {
            return Err(violation(
                CHECK,
                format!("resource must be gone once the delete operation is done, got {other:?}"),
            ));
        }
    }

    let replay = provider
        .delete(&name, &delete_key)
        .await
        .map_err(|e| violation(CHECK, format!("delete replay failed: {e}")))?;
    if replay.name != operation_name {
        return Err(violation(
            CHECK,
            format!(
                "delete replay under the same key must return operation {operation_name:?}, got {:?}",
                replay.name
            ),
        ));
    }

    let terminal = provider
        .poll_operation(&operation_name)
        .await
        .map_err(|e| violation(CHECK, format!("terminal re-poll failed: {e}")))?;
    if !terminal.done || terminal.result != operation.result {
        return Err(violation(
            CHECK,
            "terminal operations must be immutable once done".to_owned(),
        ));
    }

    match provider.poll_operation("operations/never-issued").await {
        Err(ProviderError::NotFound { .. }) => Ok(()),
        other => Err(violation(
            CHECK,
            format!("polling an unknown operation must fail with not_found, got {other:?}"),
        )),
    }
}
