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
fn lint_clean_policy_is_clean() {
    let version = PolicyVersion {
        policy_id: "pol_clean".to_string(),
        version: "1.0.0".to_string(),
        scope: PolicyScope::Global,
        supersedes: None,
        rules: vec![
            PolicyRuleInput {
                effect: PolicyEffect::Allow,
                principal_role: "admin".to_string(),
                action: "tenant.settings.update".to_string(),
                resource_prefix: "tenant:".to_string(),
                required_attribute: None,
                annotations: Vec::new(),
            },
            PolicyRuleInput {
                effect: PolicyEffect::Allow,
                principal_role: "reader".to_string(),
                action: "tenant.settings.read".to_string(),
                resource_prefix: "tenant:".to_string(),
                required_attribute: None,
                annotations: Vec::new(),
            },
        ],
    };
    let report = lint_policy_version(&version);
    assert!(
        report.is_clean(),
        "clean policy must yield is_clean() == true"
    );
    assert!(!report.has_blocking());
    assert!(report.findings.is_empty());
}

#[test]
fn lint_detects_shadowed_rule_under_broader_prefix_emits_warning() {
    let version = PolicyVersion {
        policy_id: "pol_shadow_test".to_string(),
        version: "1.0.0".to_string(),
        scope: PolicyScope::Global,
        supersedes: None,
        rules: vec![
            // Earlier broader rule: resource_prefix = "docs:"
            PolicyRuleInput {
                effect: PolicyEffect::Allow,
                principal_role: "editor".to_string(),
                action: "doc.write".to_string(),
                resource_prefix: "docs:".to_string(),
                required_attribute: None,
                annotations: Vec::new(),
            },
            // Later narrower rule: resource_prefix = "docs:project:" (subsumed)
            PolicyRuleInput {
                effect: PolicyEffect::Allow,
                principal_role: "editor".to_string(),
                action: "doc.write".to_string(),
                resource_prefix: "docs:project:".to_string(),
                required_attribute: None,
                annotations: Vec::new(),
            },
        ],
    };
    let report = lint_policy_version(&version);
    let warnings: Vec<&PolicyLintFinding> = report
        .findings
        .iter()
        .filter(|f| f.severity == LintSeverity::Warning)
        .collect();
    assert_eq!(
        warnings.len(),
        1,
        "expected exactly one Warning for shadowed rule"
    );
    assert_eq!(
        warnings[0].rule_indices,
        vec![0usize, 1],
        "warning must cite earlier rule 0 and shadowed rule 1"
    );
}

#[test]
fn lint_sibling_prefixes_not_shadowed() {
    let version = PolicyVersion {
        policy_id: "pol_siblings".to_string(),
        version: "1.0.0".to_string(),
        scope: PolicyScope::Global,
        supersedes: None,
        rules: vec![
            PolicyRuleInput {
                effect: PolicyEffect::Allow,
                principal_role: "editor".to_string(),
                action: "doc.write".to_string(),
                resource_prefix: "docs:projectA:".to_string(),
                required_attribute: None,
                annotations: Vec::new(),
            },
            PolicyRuleInput {
                effect: PolicyEffect::Allow,
                principal_role: "editor".to_string(),
                action: "doc.write".to_string(),
                resource_prefix: "docs:projectB:".to_string(),
                required_attribute: None,
                annotations: Vec::new(),
            },
        ],
    };
    let report = lint_policy_version(&version);
    let shadow_warnings: Vec<&PolicyLintFinding> = report
        .findings
        .iter()
        .filter(|f| f.severity == LintSeverity::Warning)
        .collect();
    assert!(
        shadow_warnings.is_empty(),
        "sibling prefixes must not produce shadow warnings, got: {shadow_warnings:?}"
    );
    assert!(report.is_clean());
}

#[test]
fn lint_broader_attr_on_later_rule_is_not_shadowed() {
    let version = PolicyVersion {
        policy_id: "pol_attr_broader".to_string(),
        version: "1.0.0".to_string(),
        scope: PolicyScope::Global,
        supersedes: None,
        rules: vec![
            // Earlier rule: narrower attribute guard
            PolicyRuleInput {
                effect: PolicyEffect::Allow,
                principal_role: "editor".to_string(),
                action: "doc.write".to_string(),
                resource_prefix: "docs:".to_string(),
                required_attribute: Some(("tier".to_string(), "premium".to_string())),
                annotations: Vec::new(),
            },
            // Later rule: same prefix, NO attribute guard (broader) — not shadowed
            PolicyRuleInput {
                effect: PolicyEffect::Allow,
                principal_role: "editor".to_string(),
                action: "doc.write".to_string(),
                resource_prefix: "docs:".to_string(),
                required_attribute: None,
                annotations: Vec::new(),
            },
        ],
    };
    let report = lint_policy_version(&version);
    let shadow_warnings: Vec<&PolicyLintFinding> = report
        .findings
        .iter()
        .filter(|f| f.severity == LintSeverity::Warning)
        .collect();
    assert!(
        shadow_warnings.is_empty(),
        "later rule with broader (None) attr must not be flagged as shadowed"
    );
}
