#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use data_boundary_kernel::{DataClass, DataClassification, Purpose};
use iam_identity_domain::{
    CredentialRequest, CredentialRequestKind, IdentityError, IdpBinding, Principal, User, UserId,
    issue_credential,
};

#[test]
fn sts_credentials_are_short_lived_scope_bound_and_redacted() {
    let principal =
        Principal::human("ten_alpha".into(), "usr_admin".into()).expect("human principal is valid");
    let credential = issue_credential(CredentialRequest {
        principal: principal.clone(),
        kind: CredentialRequestKind::Sts,
        purpose: Purpose::CapabilityInvocation,
        scopes: vec!["cloud.iam.read".into(), "foundry.invoke".into()],
        ttl_seconds: 900,
        issued_at_epoch_seconds: 1_000,
    })
    .expect("STS credential is valid");

    assert_eq!(credential.principal, principal);
    assert_eq!(credential.purpose.value, Purpose::CapabilityInvocation);
    assert_eq!(credential.expires_at_epoch_seconds.value, 1_900);
    assert_eq!(
        credential.scopes.data_class,
        DataClassification::from(DataClass::InternalOnly)
    );
    assert!(credential.is_active(1_899));
    assert!(!credential.is_active(1_900));
    assert!(!format!("{credential:?}").contains("bearer"));
    assert!(credential.token_fingerprint.value.starts_with("sts1:"));
}

#[test]
fn identity_user_requires_tenant_user_and_idp_binding_shapes() {
    let binding = IdpBinding::new(
        "pack-alpha".into(),
        "idp_kr_oidc".into(),
        "oidc://kr.example/admin".into(),
        1_700_000_000,
    )
    .expect("idp binding is valid");
    let user = User::new(
        "ten_alpha".into(),
        "usr_admin".into(),
        "admin@kr.example".into(),
        "KR Admin".into(),
        vec!["tenant.admin".into()],
        binding.clone(),
    )
    .expect("tenant-bound user is valid");

    assert_eq!(user.tenant_id(), "ten_alpha");
    assert_eq!(user.user_id().as_str(), "usr_admin");
    assert_eq!(user.id.value, UserId::new("usr_admin").unwrap());
    assert_eq!(user.primary_identifier.value, "admin@kr.example");
    assert_eq!(
        user.display_name.data_class,
        DataClassification::from(DataClass::PiiQuasiIdentifier)
    );
    assert_eq!(
        user.idp_binding.data_class,
        DataClassification::from(DataClass::InternalOnly)
    );
    assert_eq!(user.idp_binding(), &binding);
    assert_eq!(user.idp_binding().region_pack.value, "pack-alpha");
    assert_eq!(
        user.idp_binding().external_subject.data_class,
        DataClassification::from(DataClass::PiiIdentifying)
    );
    assert_eq!(user.schema_version.value, 1);
    assert_eq!(user.idp_binding().schema_version.value, 1);
}

#[test]
fn identity_user_rejects_invalid_user_and_idp_binding_shapes() {
    let valid_binding = IdpBinding::new(
        "pack-alpha".into(),
        "idp_kr_oidc".into(),
        "oidc://kr.example/admin".into(),
        1_700_000_000,
    )
    .expect("idp binding is valid");

    assert_eq!(
        User::new(
            "tenant".into(),
            "usr_admin".into(),
            "admin@kr.example".into(),
            "KR Admin".into(),
            vec!["tenant.admin".into()],
            valid_binding.clone(),
        ),
        Err(IdentityError::InvalidTenantId)
    );
    assert_eq!(
        User::new(
            "ten_alpha".into(),
            "user".into(),
            "admin@kr.example".into(),
            "KR Admin".into(),
            vec!["tenant.admin".into()],
            valid_binding.clone(),
        ),
        Err(IdentityError::InvalidUserId)
    );
    assert_eq!(
        User::new(
            "ten_alpha".into(),
            "usr_admin".into(),
            " ".into(),
            "KR Admin".into(),
            vec!["tenant.admin".into()],
            valid_binding,
        ),
        Err(IdentityError::EmptyPrimaryIdentifier)
    );
    assert_eq!(
        IdpBinding::new(
            "kr".into(),
            "idp_kr_oidc".into(),
            "oidc://kr.example/admin".into(),
            1_700_000_000,
        ),
        Err(IdentityError::InvalidRegionPack)
    );
    assert_eq!(
        IdpBinding::new(
            "pack-alpha".into(),
            "provider".into(),
            "oidc://kr.example/admin".into(),
            1_700_000_000,
        ),
        Err(IdentityError::InvalidIdentityProviderId)
    );
    assert_eq!(
        IdpBinding::new(
            "pack-alpha".into(),
            "idp_kr_oidc".into(),
            " ".into(),
            1_700_000_000,
        ),
        Err(IdentityError::EmptyExternalSubject)
    );
}

#[test]
fn identity_kernel_rejects_long_lived_or_unscoped_credentials() {
    let principal = Principal::service(
        "ten_alpha".into(),
        "sp_foundry".into(),
        "cap.provider.openai".into(),
    )
    .expect("service principal is valid");

    assert_eq!(
        issue_credential(CredentialRequest {
            principal: principal.clone(),
            kind: CredentialRequestKind::LongLivedApiKey,
            purpose: Purpose::CapabilityInvocation,
            scopes: vec!["foundry.invoke".into()],
            ttl_seconds: 900,
            issued_at_epoch_seconds: 1_000,
        }),
        Err(IdentityError::LongLivedCredentialForbidden)
    );
    assert_eq!(
        issue_credential(CredentialRequest {
            principal: principal.clone(),
            kind: CredentialRequestKind::Sts,
            purpose: Purpose::CapabilityInvocation,
            scopes: vec![],
            ttl_seconds: 900,
            issued_at_epoch_seconds: 1_000,
        }),
        Err(IdentityError::MissingCredentialScope)
    );
    assert_eq!(
        issue_credential(CredentialRequest {
            principal,
            kind: CredentialRequestKind::Sts,
            purpose: Purpose::CapabilityInvocation,
            scopes: vec!["foundry.invoke".into()],
            ttl_seconds: 3_601,
            issued_at_epoch_seconds: 1_000,
        }),
        Err(IdentityError::TokenTtlTooLong)
    );
}

#[test]
fn principal_constructors_validate_identity_shapes() {
    assert_eq!(
        Principal::human("tenant".into(), "usr_admin".into()),
        Err(IdentityError::InvalidTenantId)
    );
    assert_eq!(
        Principal::human("ten_alpha".into(), "user".into()),
        Err(IdentityError::InvalidUserId)
    );
    assert_eq!(
        Principal::service("ten_alpha".into(), "service".into(), "cap.provider".into()),
        Err(IdentityError::InvalidServicePrincipalId)
    );
}
