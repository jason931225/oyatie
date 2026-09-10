//! Authoring-time lint over a policy version: conflicts, duplicates, and
//! shadowing, classified by blocking severity.

use serde::{Deserialize, Serialize};

use crate::policy::PolicyVersion;

/// Severity of a lint finding.
///
/// `Error` findings block publish; `Warning` findings are advisory.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum LintSeverity {
    Error,
    Warning,
}

/// A single finding produced by the authoring-time lint pass.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PolicyLintFinding {
    pub severity: LintSeverity,
    /// Indices into `PolicyVersion::rules` of the rules involved in this finding.
    pub rule_indices: Vec<usize>,
    pub reason: String,
}

/// The aggregated result of linting a candidate `PolicyVersion`.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PolicyLintReport {
    pub findings: Vec<PolicyLintFinding>,
}

impl PolicyLintReport {
    pub fn has_blocking(&self) -> bool {
        self.findings
            .iter()
            .any(|f| f.severity == LintSeverity::Error)
    }

    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }
}

/// Lint a candidate `PolicyVersion` without publishing it.
///
/// Pure, deterministic, no network or storage access.
pub fn lint_policy_version(version: &PolicyVersion) -> PolicyLintReport {
    let rules = &version.rules;
    let mut findings: Vec<PolicyLintFinding> = Vec::new();

    for i in 0..rules.len() {
        for j in (i + 1)..rules.len() {
            let a = &rules[i];
            let b = &rules[j];

            let same_tuple = a.principal_role == b.principal_role
                && a.action == b.action
                && a.resource_prefix == b.resource_prefix
                && a.required_attribute == b.required_attribute;

            if same_tuple {
                if a.effect == b.effect {
                    findings.push(duplicate_rule_finding(i, j));
                } else {
                    findings.push(conflicting_effect_finding(i, j));
                }
            } else if a.effect == b.effect
                && a.principal_role == b.principal_role
                && a.action == b.action
                && b.resource_prefix.starts_with(&a.resource_prefix)
                && attr_subsumed_by(&b.required_attribute, &a.required_attribute)
            {
                findings.push(shadowed_rule_finding(
                    i,
                    j,
                    &a.resource_prefix,
                    &b.resource_prefix,
                ));
            }
        }
    }

    PolicyLintReport { findings }
}

fn duplicate_rule_finding(i: usize, j: usize) -> PolicyLintFinding {
    PolicyLintFinding {
        severity: LintSeverity::Error,
        rule_indices: vec![i, j],
        reason: format!(
            "rules {i} and {j} are duplicates: identical (effect, principal_role, \
             action, resource_prefix, required_attribute)"
        ),
    }
}

fn conflicting_effect_finding(i: usize, j: usize) -> PolicyLintFinding {
    PolicyLintFinding {
        severity: LintSeverity::Error,
        rule_indices: vec![i, j],
        reason: format!(
            "rules {i} and {j} conflict: Allow and Deny on identical \
             (principal_role, action, resource_prefix, required_attribute)"
        ),
    }
}

fn shadowed_rule_finding(
    i: usize,
    j: usize,
    dominator_prefix: &str,
    shadowed_prefix: &str,
) -> PolicyLintFinding {
    PolicyLintFinding {
        severity: LintSeverity::Warning,
        rule_indices: vec![i, j],
        reason: format!(
            "rule {j} is unreachable: its resource_prefix {shadowed_prefix:?} is subsumed by \
             rule {i}'s prefix {dominator_prefix:?} with an equal-or-weaker attribute guard"
        ),
    }
}

/// Returns `true` if `candidate`'s attribute guard is subsumed by (i.e., at
/// least as restrictive as) `dominator`'s attribute guard.
fn attr_subsumed_by(
    candidate: &Option<(String, String)>,
    dominator: &Option<(String, String)>,
) -> bool {
    match (candidate, dominator) {
        (_, None) => true,
        (Some(c), Some(d)) => c == d,
        (None, Some(_)) => false,
    }
}
