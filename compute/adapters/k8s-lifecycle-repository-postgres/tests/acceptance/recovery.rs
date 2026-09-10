use compute_k8s_api::{CloudComputeK8sAcceptanceApiError, CloudComputeK8sOperationLookup};
use compute_k8s_lifecycle_repository_postgres::{
    PgK8sLifecycleRepository, PgK8sLifecycleRuntimeContract,
};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::{Duration, Instant};

use super::fixtures::*;
use crate::support::tenant_count;

pub(super) async fn assert_recovery(
    setup: &PgPool,
    app: &PgPool,
    app_url: &str,
    contract: &PgK8sLifecycleRuntimeContract,
    reader: &PgK8sLifecycleRepository,
) {
    let single = PgPoolOptions::new()
        .max_connections(1)
        .connect(app_url)
        .await
        .unwrap();
    let backend: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&single)
        .await
        .unwrap();
    let writer = PgK8sLifecycleRepository::from_pool(single.clone(), contract)
        .await
        .unwrap();
    execute(setup, "CREATE FUNCTION compute_k8s_lifecycle.test_acceptance_boundary() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF current_setting('transaction_isolation') <> 'read committed' OR current_setting('lock_timeout') <> '5s' OR current_setting('statement_timeout') <> '10s' OR current_setting('oyatie.tenant_id') <> NEW.tenant_id THEN RAISE EXCEPTION 'incorrect acceptance transaction budgets or tenant'; END IF; IF NEW.idempotency_key = 'cancel-before-commit' THEN PERFORM pg_advisory_xact_lock(640004); END IF; IF NEW.idempotency_key = 'fail-before-commit' THEN RAISE EXCEPTION 'test receipt failure'; END IF; RETURN NEW; END $$").await;
    execute(setup, "CREATE TRIGGER test_acceptance_boundary BEFORE UPDATE ON compute_k8s_lifecycle.operations FOR EACH ROW WHEN (NEW.request_contract = 'pending_intent') EXECUTE FUNCTION compute_k8s_lifecycle.test_acceptance_boundary()").await;
    accept(&writer, pending_request("budget-probe", "budget-probe"))
        .await
        .unwrap();

    let mut blocker = setup.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock(640004)")
        .execute(&mut *blocker)
        .await
        .unwrap();
    let writing = writer.clone();
    let task = tokio::spawn(async move {
        accept(
            &writing,
            pending_request("cancelled-attempt", "cancel-before-commit"),
        )
        .await
    });
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let waiting: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_stat_activity WHERE pid = $1 AND wait_event = 'advisory' AND state = 'active')").bind(backend).fetch_one(setup).await.unwrap();
        if waiting {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "acceptance did not reach observed precommit boundary"
        );
        tokio::task::yield_now().await;
    }
    let original_query = operation_read_request("cancel-before-commit");
    assert_eq!(
        read(reader, original_query.clone()).await.unwrap(),
        CloudComputeK8sOperationLookup::NotObserved
    );
    assert_eq!(
        original_query.boundary.idempotency_key, "cancel-before-commit",
        "NotObserved while the original write is in flight never licenses a replacement identity"
    );
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    blocker.rollback().await.unwrap();
    drain_queued_rollback_on_connection(&single, backend).await;
    assert_eq!(tenant_count(app, "ten_alpha", "SELECT count(*)::bigint FROM compute_k8s_lifecycle.operations WHERE principal_id = 'sp-compute-live' AND surface = 'cloud.compute.k8s.cluster.create' AND idempotency_key = 'cancel-before-commit'").await, 0);

    let failed = accept(
        &writer,
        pending_request("failed-attempt", "fail-before-commit"),
    )
    .await;
    assert_eq!(
        failed.unwrap_err(),
        CloudComputeK8sAcceptanceApiError::RepositoryUnavailable
    );
    drain_queued_rollback_on_connection(&single, backend).await;
    assert_eq!(
        read(reader, operation_read_request("fail-before-commit"))
            .await
            .unwrap(),
        CloudComputeK8sOperationLookup::NotObserved
    );
    execute(
        setup,
        "DROP TRIGGER test_acceptance_boundary ON compute_k8s_lifecycle.operations",
    )
    .await;
    execute(
        setup,
        "DROP FUNCTION compute_k8s_lifecycle.test_acceptance_boundary()",
    )
    .await;
    for key in ["cancel-before-commit", "fail-before-commit"] {
        let retry = accept(&writer, pending_request("same-identity-retry", key))
            .await
            .unwrap();
        assert_eq!(retry.operation.receipt.operation_key.idempotency_key, key);
        assert_eq!(retry.operation.receipt.request_id, "same-identity-retry");
        assert_eq!(
            read(reader, operation_read_request(key)).await.unwrap(),
            CloudComputeK8sOperationLookup::Found(retry.operation)
        );
    }
    single.close().await;
}

async fn drain_queued_rollback_on_connection(single: &PgPool, expected_backend: i32) {
    let backend: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(single)
        .await
        .unwrap();
    assert_eq!(
        backend, expected_backend,
        "observe completion on the cancelled transaction's connection"
    );
}

async fn execute(setup: &PgPool, sql: &str) {
    sqlx::query(sql).execute(setup).await.unwrap();
}
