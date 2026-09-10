#![allow(clippy::expect_used)]

use secrets_domain::{
    MAX_SECRET_REFERENCE_CACHE_TTL_SECONDS, SecretProviderKind, SecretReferenceUri,
    SecretReferenceUriError, clamp_secret_reference_cache_ttl_seconds,
};

#[test]
fn secret_reference_uri_parses_canonical_openbao_reference() {
    let reference = SecretReferenceUri::parse(
        "openbao:secret/tenant:a1b2c3d4e5f6g7h8/workflow-engine/oauth-client-secret@v42",
    )
    .expect("canonical SecretReference URI parses");

    assert_eq!(reference.provider(), SecretProviderKind::OpenBao);
    assert_eq!(
        reference.path(),
        "tenant:a1b2c3d4e5f6g7h8/workflow-engine/oauth-client-secret"
    );
    assert_eq!(reference.version(), Some(42));
    assert_eq!(
        reference.normalized_uri(),
        "openbao:secret/tenant:a1b2c3d4e5f6g7h8/workflow-engine/oauth-client-secret@v42"
    );
    assert_eq!(
        reference.path_segments().collect::<Vec<_>>(),
        vec![
            "tenant:a1b2c3d4e5f6g7h8",
            "workflow-engine",
            "oauth-client-secret"
        ]
    );
}

#[test]
fn secret_reference_uri_parses_wrapped_config_reference() {
    let reference = SecretReferenceUri::parse_config_reference(
        "${openbao:secret/shared/cloud-secrets/hsm-pin@v7}",
    )
    .expect("wrapped config SecretReference parses");

    assert_eq!(reference.path(), "shared/cloud-secrets/hsm-pin");
    assert_eq!(reference.version(), Some(7));
    assert_eq!(
        reference.normalized_config_reference(),
        "${openbao:secret/shared/cloud-secrets/hsm-pin@v7}"
    );
}

#[test]
fn secret_reference_uri_rejects_non_contract_shapes() {
    assert_eq!(
        SecretReferenceUri::parse("secret/data/t/ten_alpha/db/password"),
        Err(SecretReferenceUriError::MissingOpenBaoSecretPrefix)
    );
    assert_eq!(
        SecretReferenceUri::parse("openbao:secret/"),
        Err(SecretReferenceUriError::EmptyPath)
    );
    assert_eq!(
        SecretReferenceUri::parse("openbao:secret/tenant//secret"),
        Err(SecretReferenceUriError::EmptySegment)
    );
    assert_eq!(
        SecretReferenceUri::parse("openbao:secret/tenant/../secret"),
        Err(SecretReferenceUriError::TraversalSegment)
    );
    assert_eq!(
        SecretReferenceUri::parse("openbao:secret/tenant/secret?debug=true"),
        Err(SecretReferenceUriError::InvalidSegmentCharacter)
    );
    assert_eq!(
        SecretReferenceUri::parse("openbao:secret/tenant/secret@latest"),
        Err(SecretReferenceUriError::InvalidVersion)
    );
    assert_eq!(
        SecretReferenceUri::parse("openbao:secret/tenant/secret@v0"),
        Err(SecretReferenceUriError::ZeroVersion)
    );
    assert_eq!(
        SecretReferenceUri::parse_config_reference("${openbao:secret/tenant/secret"),
        Err(SecretReferenceUriError::MissingConfigWrapper)
    );
}

#[test]
fn secret_reference_cache_ttl_clamps_to_policy_ceiling() {
    assert_eq!(MAX_SECRET_REFERENCE_CACHE_TTL_SECONDS, 60);
    assert_eq!(clamp_secret_reference_cache_ttl_seconds(10), 10);
    assert_eq!(clamp_secret_reference_cache_ttl_seconds(60), 60);
    assert_eq!(clamp_secret_reference_cache_ttl_seconds(61), 60);
    assert_eq!(clamp_secret_reference_cache_ttl_seconds(u64::MAX), 60);
}
