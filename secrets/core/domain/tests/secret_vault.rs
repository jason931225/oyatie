#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use data_boundary_kernel::{DataClassification, OperationalDataClass};
use secrets_domain::{SecretError, SecretMaterial, SecretRef, SecretStatus, SecretVault};

#[test]
fn secret_vault_versions_provider_secret_without_debug_leak() {
    let secret_ref = SecretRef::new(
        "ten_alpha".into(),
        "cap.provider.openai".into(),
        "api-key".into(),
    )
    .expect("secret ref is valid");
    let mut vault = SecretVault::default();

    let version = vault
        .put(
            secret_ref.clone(),
            SecretMaterial::from_bytes(b"sk-live-secret".to_vec())
                .expect("secret material is non-empty"),
            None,
        )
        .expect("secret can be stored");
    assert_eq!(version.version.value, 1);
    assert_eq!(version.status, SecretStatus::Active);
    assert_eq!(
        version.material.classification(),
        DataClassification::Operational(OperationalDataClass::Secret)
    );
    assert_eq!(
        version.material.legacy_data_class(),
        data_boundary_kernel::DataClass::Secret
    );
    #[allow(deprecated)]
    {
        assert_eq!(
            version.material.data_class(),
            version.material.legacy_data_class()
        );
    }

    let debug = format!("{version:?}");
    assert!(!debug.contains("sk-live-secret"));
    assert!(debug.contains("REDACTED"));

    let lease = vault
        .get(&secret_ref, 1_000)
        .expect("current secret can be resolved");
    assert_eq!(lease.secret_ref(), &secret_ref);
    assert_eq!(lease.expose_for_provider(), b"sk-live-secret");
    assert!(!format!("{lease:?}").contains("sk-live-secret"));
}

#[test]
fn secret_vault_rotates_revokes_and_expires_current_version() {
    let secret_ref = SecretRef::new(
        "ten_alpha".into(),
        "cap.provider.openai".into(),
        "api-key".into(),
    )
    .expect("secret ref is valid");
    let mut vault = SecretVault::default();

    vault
        .put(
            secret_ref.clone(),
            SecretMaterial::from_bytes(b"old".to_vec()).expect("secret material is non-empty"),
            None,
        )
        .expect("initial secret is valid");
    let rotated = vault
        .rotate(
            &secret_ref,
            SecretMaterial::from_bytes(b"new".to_vec()).expect("secret material is non-empty"),
            Some(2_000),
        )
        .expect("rotation is valid");
    assert_eq!(rotated.version.value, 2);
    assert_eq!(rotated.previous_version.value, Some(1));
    assert_eq!(
        vault.get(&secret_ref, 1_999).unwrap().expose_for_provider(),
        b"new"
    );
    assert_eq!(
        vault.get(&secret_ref, 2_000),
        Err(SecretError::SecretExpired)
    );

    vault
        .rotate(
            &secret_ref,
            SecretMaterial::from_bytes(b"newer".to_vec()).expect("secret material is non-empty"),
            None,
        )
        .expect("second rotation is valid");
    vault.revoke(&secret_ref).expect("revoke is valid");
    assert_eq!(
        vault.get(&secret_ref, 2_001),
        Err(SecretError::SecretRevoked)
    );
}

#[test]
fn secret_ref_and_material_validate_inputs() {
    assert_eq!(
        SecretRef::new(
            "tenant".into(),
            "cap.provider.openai".into(),
            "api-key".into()
        ),
        Err(SecretError::InvalidTenantId)
    );
    assert_eq!(
        SecretRef::new(
            "ten_alpha".into(),
            "provider.openai".into(),
            "api-key".into()
        ),
        Err(SecretError::InvalidCapabilityId)
    );
    assert_eq!(
        SecretRef::new("ten_alpha".into(), "cap.provider.openai".into(), " ".into()),
        Err(SecretError::InvalidSecretName)
    );
    assert_eq!(
        SecretMaterial::try_from(Vec::new()),
        Err(SecretError::EmptySecretMaterial)
    );
    assert_eq!(
        SecretMaterial::from_bytes(Vec::new()),
        Err(SecretError::EmptySecretMaterial)
    );
}
