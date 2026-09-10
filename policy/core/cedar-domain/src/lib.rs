//! Cedar-shaped authorization policy kernel.
//!
//! This is deliberately pure: it stores versioned policy records and evaluates
//! role + attribute predicates without network, storage, or runtime side effects.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod authorization;
pub mod authz_engine;
pub mod backbone_write;
pub mod lint;
pub mod obligations;
pub mod policy;
pub mod policy_diff;
pub mod policy_set;
pub mod rebac;
pub mod runtime_evaluator;

pub use authorization::{
    AuthorizationDecision, AuthorizationQuery, AuthorizationSubject, PolicyError,
};
pub use backbone_write::{
    BACKBONE_WRITE_POLICY_VERSION, BackboneWriteOperation, backbone_write_policy_versions,
};
pub use lint::{LintSeverity, PolicyLintFinding, PolicyLintReport, lint_policy_version};
pub use obligations::{AnnotationKind, PolicyAnnotation};
pub use policy::{
    PolicyEffect, PolicyRule, PolicyRuleInput, PolicyScope, PolicyVersion, PublishedPolicy,
};
pub use policy_diff::{ImpactReport, RuleDelta, diff_policy_versions};
pub use policy_set::PolicySet;
pub use runtime_evaluator::{
    CedarEvaluationLogEntry, CedarRuntimeError, CedarRuntimeEvaluation, CedarRuntimeEvaluator,
};
