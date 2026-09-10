use compute_k8s_api::CloudComputeK8sAcceptanceApiError as Error;
use compute_k8s_lifecycle_repository_postgres::PgK8sLifecycleRepository;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};

use super::fixtures::*;

pub(super) async fn assert_integrity(setup: &PgPool, repository: &PgK8sLifecycleRepository) {
    let constraints: Vec<(String, String)> = sqlx::query_as("SELECT conname::text, pg_get_constraintdef(oid) FROM pg_constraint WHERE conrelid = 'compute_k8s_lifecycle.operations'::regclass AND conname IN ('operations_receipt_surface_matches', 'operations_contract_state', 'operations_request_contract', 'operations_database_schema_version') ORDER BY conname")
        .fetch_all(setup).await.unwrap();
    assert_eq!(constraints.len(), 4);
    let original = accept(
        repository,
        pending_request("integrity-original", "integrity"),
    )
    .await
    .unwrap();
    let row = sqlx::query("SELECT receipt_json, receipt_digest FROM compute_k8s_lifecycle.operations WHERE tenant_id = 'ten_alpha' AND principal_id = 'sp-compute-live' AND surface = 'cloud.compute.k8s.cluster.create' AND idempotency_key = 'integrity'").fetch_one(setup).await.unwrap();
    let original_json: Value = row.get("receipt_json");
    let original_digest: String = row.get("receipt_digest");
    for field in [
        "digest",
        "key",
        "principal",
        "surface",
        "tenant",
        "resource",
        "request",
        "timestamp",
        "intent",
        "contract",
        "malformed",
    ] {
        let mut json = original_json.clone();
        match field {
            "digest" => json["request_id"] = "tampered".into(),
            "key" => json["operation_key"]["idempotency_key"] = "other".into(),
            "principal" => json["operation_key"]["principal_id"] = "other".into(),
            "surface" => json["operation_key"]["surface"] = "other".into(),
            "tenant" => json["intent"]["tenant_id"] = "ten_beta".into(),
            "resource" => {
                json["intent"]["resource_id"] =
                    crate::support::cluster_id("ten_alpha", "other").into()
            }
            "request" => json["request_id"] = " ".into(),
            "timestamp" => json["accepted_at_epoch_seconds"] = 1.into(),
            "intent" => json["intent"]["control_plane_private"] = false.into(),
            "contract" => json["request_contract"] = "unknown".into(),
            "malformed" => json = Value::Null,
            _ => unreachable!(),
        }
        let digest = if field == "digest" {
            original_digest.clone()
        } else {
            digest(&json)
        };
        update_receipt(setup, &json, &digest).await;
        assert_refused(repository, field).await;
        update_receipt(setup, &original_json, &original_digest).await;
    }
    sqlx::query("UPDATE compute_k8s_lifecycle.operations SET receipt_kind = NULL, receipt_json = NULL, receipt_digest = NULL, completed_at = NULL WHERE idempotency_key = 'integrity'").execute(setup).await.unwrap();
    assert_refused(repository, "incomplete").await;
    sqlx::query("UPDATE compute_k8s_lifecycle.operations SET receipt_kind = 'create', receipt_json = $1, receipt_digest = $2, completed_at = now() WHERE idempotency_key = 'integrity'").bind(&original_json).bind(&original_digest).execute(setup).await.unwrap();
    for (column, value) in [("request_fingerprint", "wrong"), ("receipt_kind", "delete")] {
        if column == "receipt_kind" {
            sqlx::query("ALTER TABLE compute_k8s_lifecycle.operations DROP CONSTRAINT operations_receipt_surface_matches").execute(setup).await.unwrap();
        }
        let saved: String = sqlx::query_scalar(&format!("SELECT {column} FROM compute_k8s_lifecycle.operations WHERE idempotency_key = 'integrity'")).fetch_one(setup).await.unwrap();
        sqlx::query(&format!("UPDATE compute_k8s_lifecycle.operations SET {column} = $1 WHERE idempotency_key = 'integrity'")).bind(value).execute(setup).await.unwrap();
        assert_refused(repository, column).await;
        sqlx::query(&format!("UPDATE compute_k8s_lifecycle.operations SET {column} = $1 WHERE idempotency_key = 'integrity'")).bind(saved).execute(setup).await.unwrap();
    }
    drop_contract_constraints_to_probe_runtime_refusal(setup).await;
    for mutation in [
        "operation_state = NULL",
        "operation_state = 'running'",
        "request_contract = 'unknown'",
        "schema_version = 2",
        "request_contract = 'trusted_envelope'",
    ] {
        sqlx::query(&format!("UPDATE compute_k8s_lifecycle.operations SET {mutation} WHERE idempotency_key = 'integrity'")).execute(setup).await.unwrap();
        assert_refused(repository, mutation).await;
        sqlx::query("UPDATE compute_k8s_lifecycle.operations SET operation_state = 'accepted', request_contract = 'pending_intent', schema_version = 1 WHERE idempotency_key = 'integrity'").execute(setup).await.unwrap();
    }
    assert_eq!(
        accept(repository, pending_request("retry", "integrity"))
            .await
            .unwrap(),
        original
    );
    for (name, definition) in constraints {
        sqlx::query(&format!(
            "ALTER TABLE compute_k8s_lifecycle.operations ADD CONSTRAINT {} {definition}",
            crate::support::quote_identifier(&name)
        ))
        .execute(setup)
        .await
        .unwrap();
    }
}

async fn drop_contract_constraints_to_probe_runtime_refusal(setup: &PgPool) {
    for sql in [
        "ALTER TABLE compute_k8s_lifecycle.operations DROP CONSTRAINT operations_contract_state",
        "ALTER TABLE compute_k8s_lifecycle.operations DROP CONSTRAINT operations_request_contract",
        "ALTER TABLE compute_k8s_lifecycle.operations DROP CONSTRAINT operations_database_schema_version",
    ] {
        sqlx::query(sql).execute(setup).await.unwrap();
    }
}

async fn assert_refused(repository: &PgK8sLifecycleRepository, mutation: &str) {
    assert_eq!(
        read(repository, operation_read_request("integrity"))
            .await
            .unwrap_err(),
        Error::IntegrityViolation,
        "lookup: {mutation}"
    );
    assert_eq!(
        accept(repository, pending_request("retry", "integrity"))
            .await
            .unwrap_err(),
        Error::IntegrityViolation,
        "accept: {mutation}"
    );
}

async fn update_receipt(setup: &PgPool, json: &Value, digest: &str) {
    let result = sqlx::query("UPDATE compute_k8s_lifecycle.operations SET receipt_json = $1, receipt_digest = $2 WHERE tenant_id = 'ten_alpha' AND principal_id = 'sp-compute-live' AND surface = 'cloud.compute.k8s.cluster.create' AND idempotency_key = 'integrity'").bind(json).bind(digest).execute(setup).await.unwrap();
    assert_eq!(result.rows_affected(), 1);
}

fn digest(value: &Value) -> String {
    let mut canonical = value.clone();
    canonical.sort_all_objects();
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&canonical).unwrap())
    )
}
