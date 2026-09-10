#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use secrets_domain::{
    SecretBootstrapError, SecretBootstrapRequest, SecretProviderKind, SecretReference,
    evaluate_secret_bootstrap,
};

#[test]
fn cloud_secret_reference_is_openbao_metadata_only_and_tenant_scoped() {
    let reference = SecretReference::openbao(
        "ten_alpha",
        "kr-seoul-1",
        "cell-kr-seoul-1-a-001",
        "secret/data/t/ten_alpha/bootstrap/postgres/admin",
        "v1",
        "evidence://cloud-secrets/bootstrap/ten_alpha/001",
    )
    .expect("metadata-only OpenBao secret reference is valid");

    assert_eq!(reference.provider(), SecretProviderKind::OpenBao);
    assert_eq!(reference.tenant_id(), "ten_alpha");
    assert_eq!(reference.region(), "kr-seoul-1");
    assert_eq!(reference.cell_id(), "cell-kr-seoul-1-a-001");
    assert_eq!(
        reference.vault_path().as_str(),
        "secret/data/t/ten_alpha/bootstrap/postgres/admin"
    );
    assert_eq!(reference.version_label(), "v1");
    assert_eq!(
        reference.evidence_ref(),
        "evidence://cloud-secrets/bootstrap/ten_alpha/001"
    );

    let debug = format!("{reference:?}");
    assert!(!debug.contains("sk-"));
    assert!(!debug.contains("password="));
    assert!(!debug.contains("token="));
}

#[test]
fn cloud_secret_reference_rejects_cross_tenant_paths_and_secret_material() {
    assert_eq!(
        SecretReference::openbao(
            "ten_alpha",
            "kr-seoul-1",
            "cell-kr-seoul-1-a-001",
            "secret/data/t/ten_beta/bootstrap/postgres/admin",
            "v1",
            "evidence://cloud-secrets/bootstrap/ten_alpha/001",
        ),
        Err(SecretBootstrapError::PathTenantMismatch)
    );

    assert_eq!(
        SecretReference::openbao(
            "ten_alpha",
            "kr-seoul-1",
            "cell-kr-seoul-1-a-001",
            "secret/data/t/ten_alpha/bootstrap/postgres/admin",
            "v1",
            "sk-live-secret-in-evidence",
        ),
        Err(SecretBootstrapError::EvidenceRefLooksLikeSecret)
    );
}

#[test]
fn secret_bootstrap_policy_fails_closed_without_external_store_or_sealed_channel() {
    let base = SecretBootstrapRequest {
        tenant_id: "ten_alpha".to_string(),
        region: "kr-seoul-1".to_string(),
        cell_id: "cell-kr-seoul-1-a-001".to_string(),
        external_secret_store_ready: true,
        sealed_bootstrap_channel_ready: true,
        plaintext_env_present: false,
        repo_secret_material_detected: false,
        evidence_ref: "evidence://cloud-secrets/bootstrap/ten_alpha/001".to_string(),
    };

    let allowed = evaluate_secret_bootstrap(base.clone()).expect("ready substrate is allowed");
    assert_eq!(allowed.tenant_id, "ten_alpha");
    assert_eq!(allowed.status.as_str(), "allowed");

    let mut missing_store = base.clone();
    missing_store.external_secret_store_ready = false;
    assert_eq!(
        evaluate_secret_bootstrap(missing_store),
        Err(SecretBootstrapError::ExternalSecretStoreUnavailable)
    );

    let mut missing_channel = base.clone();
    missing_channel.sealed_bootstrap_channel_ready = false;
    assert_eq!(
        evaluate_secret_bootstrap(missing_channel),
        Err(SecretBootstrapError::SealedBootstrapChannelUnavailable)
    );

    let mut plaintext_env = base.clone();
    plaintext_env.plaintext_env_present = true;
    assert_eq!(
        evaluate_secret_bootstrap(plaintext_env),
        Err(SecretBootstrapError::SecretMaterialInBootstrap)
    );

    let mut repo_material = base;
    repo_material.repo_secret_material_detected = true;
    assert_eq!(
        evaluate_secret_bootstrap(repo_material),
        Err(SecretBootstrapError::SecretMaterialInBootstrap)
    );
}
