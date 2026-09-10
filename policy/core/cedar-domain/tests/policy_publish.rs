#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

mod common;

use common::*;
use policy_cedar_domain::authz_engine::{
    AuthzDecision, AuthzRequest, EvalLogFilter, PrincipalType,
};
use policy_cedar_domain::*;
use serde_json::json;

#[test]
fn publish_accepts_global_and_tenant_scoped_semver_policy_versions() {
    let mut policies = PolicySet::default();

    let global = policies
        .publish(policy_version(
            "pol_global_reader",
            "1.0.0",
            PolicyScope::Global,
            None,
        ))
        .expect("global policy publishes");
    let tenant = policies
        .publish(policy_version(POLICY_ID, "1.0.0", tenant_scope(), None))
        .expect("tenant policy publishes");

    assert_eq!(global.scope, PolicyScope::Global);
    assert_eq!(
        tenant.scope,
        PolicyScope::Tenant(TEST_TENANT_ID.to_string())
    );
}

#[test]
fn publish_rejects_non_semver_and_duplicate_policy_versions() {
    let mut policies = PolicySet::default();

    assert_eq!(
        policies.publish(policy_version(POLICY_ID, "01.0.0", tenant_scope(), None)),
        Err(PolicyError::InvalidSemver)
    );

    policies
        .publish(policy_version(POLICY_ID, "1.0.0", tenant_scope(), None))
        .expect("initial policy publishes");
    assert_eq!(
        policies.publish(policy_version(POLICY_ID, "1.0.0", tenant_scope(), None)),
        Err(PolicyError::VersionAlreadyExists)
    );
}

#[test]
fn publish_enforces_supersession_chain_integrity() {
    let mut policies = PolicySet::default();
    policies
        .publish(policy_version(POLICY_ID, "1.0.0", tenant_scope(), None))
        .expect("initial policy publishes");
    policies
        .publish(policy_version(
            POLICY_ID,
            "1.1.0",
            tenant_scope(),
            Some("1.0.0"),
        ))
        .expect("newer policy can supersede older same-scope policy");

    let chain = policies
        .supersession_chain(POLICY_ID, "1.1.0")
        .expect("chain resolves");
    assert_eq!(
        chain
            .iter()
            .map(|policy| policy.version.as_str())
            .collect::<Vec<_>>(),
        vec!["1.1.0", "1.0.0"]
    );

    assert_eq!(
        policies.publish(policy_version(
            POLICY_ID,
            "1.2.0",
            PolicyScope::Global,
            Some("1.1.0")
        )),
        Err(PolicyError::SupersedesScopeMismatch)
    );
    assert_eq!(
        policies.publish(policy_version(
            POLICY_ID,
            "1.2.0",
            tenant_scope(),
            Some("2.0.0")
        )),
        Err(PolicyError::SupersedesNotOlder)
    );
    assert_eq!(
        policies.publish(policy_version(
            POLICY_ID,
            "1.2.0",
            tenant_scope(),
            Some("1.2.0")
        )),
        Err(PolicyError::SupersedesSelf)
    );
    assert_eq!(
        policies.publish(policy_version(
            POLICY_ID,
            "1.2.0",
            tenant_scope(),
            Some("1.0.1")
        )),
        Err(PolicyError::SupersedesMissing)
    );
}

#[test]
fn authorization_uses_only_active_unsuperseded_policy_versions() {
    let mut policies = PolicySet::default();
    policies
        .publish(policy_version_with_effect(
            POLICY_ID,
            "1.0.0",
            tenant_scope(),
            None,
            PolicyEffect::Allow,
        ))
        .expect("initial allow policy publishes");
    policies
        .publish(policy_version_with_effect(
            POLICY_ID,
            "1.1.0",
            tenant_scope(),
            Some("1.0.0"),
            PolicyEffect::Deny,
        ))
        .expect("new deny policy supersedes old allow policy");

    let decision = policies.authorize(&AuthorizationQuery {
        subject: AuthorizationSubject {
            tenant_id: TEST_TENANT_ID.to_string(),
            roles: vec!["tenant-admin".to_string()],
        },
        action: "tenant.settings.update".to_string(),
        resource: TEST_TENANT_RESOURCE.to_string(),
        attributes: BTreeMap::new(),
    });

    assert!(!decision.allowed);
    assert_eq!(decision.matched_policy.as_deref(), Some(POLICY_ID));
}
