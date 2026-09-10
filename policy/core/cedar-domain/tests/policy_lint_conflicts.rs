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
fn lint_report_serde_roundtrip_and_has_blocking_tracks_error_severity() {
    // A report with one Error finding is blocking.
    let error_finding = PolicyLintFinding {
        severity: LintSeverity::Error,
        rule_indices: vec![0, 2],
        reason: "rules 0 and 2 conflict: Allow and Deny on identical (principal_role, action, resource_prefix, required_attribute)".to_string(),
    };
    let report_with_error = PolicyLintReport {
        findings: vec![error_finding.clone()],
    };
    assert!(
        report_with_error.has_blocking(),
        "report with Error finding must be blocking"
    );
    assert!(
        !report_with_error.is_clean(),
        "report with Error finding must not be clean"
    );

    // Round-trip through serde_json.
    let json = serde_json::to_string(&report_with_error).expect("PolicyLintReport serializes");
    let roundtrip: PolicyLintReport =
        serde_json::from_str(&json).expect("PolicyLintReport deserializes");
    assert_eq!(report_with_error, roundtrip);
    assert!(roundtrip.has_blocking());

    // A report with only Warning findings is not blocking.
    let warning_finding = PolicyLintFinding {
        severity: LintSeverity::Warning,
        rule_indices: vec![1, 3],
        reason: "rule 3 is shadowed by rule 1".to_string(),
    };
    let report_warning_only = PolicyLintReport {
        findings: vec![warning_finding],
    };
    assert!(
        !report_warning_only.has_blocking(),
        "Warning-only report must not be blocking"
    );
    assert!(
        !report_warning_only.is_clean(),
        "Warning-only report must not be clean"
    );

    // An empty report is clean and not blocking.
    let empty_report = PolicyLintReport { findings: vec![] };
    assert!(empty_report.is_clean(), "empty report must be clean");
    assert!(
        !empty_report.has_blocking(),
        "empty report must not be blocking"
    );

    // LintSeverity serde: "Error" and "Warning" PascalCase wire values.
    let error_json =
        serde_json::to_string(&LintSeverity::Error).expect("LintSeverity::Error serializes");
    assert_eq!(error_json, "\"Error\"");
    let warning_json =
        serde_json::to_string(&LintSeverity::Warning).expect("LintSeverity::Warning serializes");
    assert_eq!(warning_json, "\"Warning\"");
}

#[test]
fn lint_detects_conflict_allow_deny_pair_emits_one_error_with_both_indices() {
    let version = PolicyVersion {
        policy_id: "pol_conflict_test".to_string(),
        version: "1.0.0".to_string(),
        scope: PolicyScope::Global,
        supersedes: None,
        rules: vec![
            PolicyRuleInput {
                effect: PolicyEffect::Allow,
                principal_role: "editor".to_string(),
                action: "doc.write".to_string(),
                resource_prefix: "docs:".to_string(),
                required_attribute: None,
                annotations: Vec::new(),
            },
            PolicyRuleInput {
                effect: PolicyEffect::Deny,
                principal_role: "editor".to_string(),
                action: "doc.write".to_string(),
                resource_prefix: "docs:".to_string(),
                required_attribute: None,
                annotations: Vec::new(),
            },
        ],
    };
    let report = lint_policy_version(&version);
    let errors: Vec<&PolicyLintFinding> = report
        .findings
        .iter()
        .filter(|f| f.severity == LintSeverity::Error)
        .collect();
    assert_eq!(
        errors.len(),
        1,
        "expected exactly one Error finding for Allow/Deny conflict"
    );
    assert_eq!(
        errors[0].rule_indices,
        vec![0usize, 1],
        "error finding must cite both rule indices 0 and 1"
    );
    assert!(report.has_blocking());
}

#[test]
fn lint_detects_conflict_with_required_attribute_emits_error() {
    let attr = Some(("tier".to_string(), "premium".to_string()));
    let version = PolicyVersion {
        policy_id: "pol_attr_conflict".to_string(),
        version: "1.0.0".to_string(),
        scope: PolicyScope::Global,
        supersedes: None,
        rules: vec![
            PolicyRuleInput {
                effect: PolicyEffect::Allow,
                principal_role: "subscriber".to_string(),
                action: "content.read".to_string(),
                resource_prefix: "content:premium:".to_string(),
                required_attribute: attr.clone(),
                annotations: Vec::new(),
            },
            PolicyRuleInput {
                effect: PolicyEffect::Deny,
                principal_role: "subscriber".to_string(),
                action: "content.read".to_string(),
                resource_prefix: "content:premium:".to_string(),
                required_attribute: attr,
                annotations: Vec::new(),
            },
        ],
    };
    let report = lint_policy_version(&version);
    let errors: Vec<&PolicyLintFinding> = report
        .findings
        .iter()
        .filter(|f| f.severity == LintSeverity::Error)
        .collect();
    assert_eq!(
        errors.len(),
        1,
        "expected one Error for attr-matching conflict"
    );
    assert_eq!(errors[0].rule_indices, vec![0usize, 1]);
}

#[test]
fn lint_detects_duplicate_rules_emits_error() {
    let rule = PolicyRuleInput {
        effect: PolicyEffect::Allow,
        principal_role: "reader".to_string(),
        action: "resource.read".to_string(),
        resource_prefix: "res:".to_string(),
        required_attribute: None,
        annotations: Vec::new(),
    };
    let version = PolicyVersion {
        policy_id: "pol_dup_test".to_string(),
        version: "1.0.0".to_string(),
        scope: PolicyScope::Global,
        supersedes: None,
        rules: vec![rule.clone(), rule],
    };
    let report = lint_policy_version(&version);
    let errors: Vec<&PolicyLintFinding> = report
        .findings
        .iter()
        .filter(|f| f.severity == LintSeverity::Error)
        .collect();
    assert_eq!(
        errors.len(),
        1,
        "expected exactly one Error finding for duplicate rules"
    );
    assert_eq!(errors[0].rule_indices, vec![0usize, 1]);
}
