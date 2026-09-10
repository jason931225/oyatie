#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

mod common;

use common::*;
use policy_cedar_domain::policy_diff::*;
use policy_cedar_domain::{PolicyEffect, PolicyRuleInput, PolicyScope, PolicyVersion};

#[test]
fn broadened_resource_prefix_widens() {
    // prev: Allow on "docs:private:" (narrower)
    // next: Allow on "docs:" (broader — shorter prefix)
    let prev = pv(
        "1.0.0",
        vec![allow_rule("editor", "doc.write", "docs:private:")],
    );
    let next = pv("1.1.0", vec![allow_rule("editor", "doc.write", "docs:")]);

    let report = diff_policy_versions(&prev, &next);

    assert!(
        report.has_widening(),
        "shortening the resource_prefix must widen; deltas={:?}",
        report.deltas
    );
    assert!(
        report
            .deltas
            .iter()
            .any(|d| matches!(d, RuleDelta::BroadenedAllow { .. })),
        "expected BroadenedAllow delta; deltas={:?}",
        report.deltas
    );
}

#[test]
fn dropping_required_attribute_widens() {
    let prev = pv(
        "1.0.0",
        vec![allow_rule_attr(
            "subscriber",
            "content.read",
            "content:",
            Some(("tier", "premium")),
        )],
    );
    let next = pv(
        "1.1.0",
        vec![allow_rule_attr(
            "subscriber",
            "content.read",
            "content:",
            None,
        )],
    );

    let report = diff_policy_versions(&prev, &next);

    assert!(
        report.has_widening(),
        "dropping required_attribute must widen; deltas={:?}",
        report.deltas
    );
    assert!(
        report
            .deltas
            .iter()
            .any(|d| matches!(d, RuleDelta::BroadenedAllow { .. })),
        "expected BroadenedAllow delta; deltas={:?}",
        report.deltas
    );
}

#[test]
fn added_deny_is_not_widening() {
    let prev = pv("1.0.0", vec![]);
    let next = pv("1.1.0", vec![deny_rule("guest", "admin.write", "admin:")]);

    let report = diff_policy_versions(&prev, &next);

    assert!(
        !report.has_widening(),
        "adding a Deny rule must NOT widen; deltas={:?}",
        report.deltas
    );
    assert!(
        report
            .deltas
            .iter()
            .any(|d| matches!(d, RuleDelta::AddedDeny(_))),
        "expected AddedDeny delta"
    );
}

#[test]
fn effect_flip_allow_to_deny_not_widening() {
    let prev_rule = allow_rule("ops", "cluster.drain", "k8s:cluster:");
    let next_rule = deny_rule("ops", "cluster.drain", "k8s:cluster:");

    let prev = pv("1.0.0", vec![prev_rule]);
    let next = pv("1.1.0", vec![next_rule]);

    let report = diff_policy_versions(&prev, &next);

    assert!(
        !report.has_widening(),
        "flipping Allow→Deny must NOT widen; deltas={:?}",
        report.deltas
    );
    assert!(
        report.deltas.iter().any(|d| matches!(
            d,
            RuleDelta::EffectFlipped { next_rule, .. } if next_rule.effect == PolicyEffect::Deny
        )),
        "expected EffectFlipped delta with next Deny; deltas={:?}",
        report.deltas
    );
}
