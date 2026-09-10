//! Everything the loader refuses — including every field that used to parse and be dropped.

#![allow(dead_code)]

mod common;

use common::*;
use port_engine_rulepack::{LoadedRulePack, RulepackError};

#[test]
fn refuses_undeclared_apply() {
    let json = r#"{
  "pair": {"source": "go", "target": "rust"},
  "rules": [{
"id": "identity",
"version": "0",
"precondition": "unit_present",
"construction": "pass_through",
"selecting_fixtures": [{"id": "f", "unit": "u", "selects": true}]
  }],
  "applies": {"u": ["missing"]}
}"#;
    let err = LoadedRulePack::load_from_str(json).expect_err("undeclared apply");
    assert!(matches!(err, RulepackError::UndeclaredApply { .. }));
}

#[test]
fn refuses_rule_without_selecting_fixture() {
    let json = r#"{
  "pair": {"source": "go", "target": "rust"},
  "rules": [{
"id": "orphan",
"version": "0",
"precondition": "unit_present",
"construction": "pass_through",
"selecting_fixtures": []
  }],
  "applies": {}
}"#;
    let err = LoadedRulePack::load_from_str(json).expect_err("missing fixture must refuse");
    assert!(matches!(
        err,
        RulepackError::MissingSelectingFixture { rule } if rule == "orphan"
    ));
}

#[test]
fn refuses_rule_with_omitted_selecting_fixtures_field() {
    let json = r#"{
  "pair": {"source": "go", "target": "rust"},
  "rules": [{
"id": "bare",
"version": "0",
"precondition": "unit_present",
"construction": "pass_through"
  }],
  "applies": {}
}"#;
    let err = LoadedRulePack::load_from_str(json).expect_err("omitted fixtures must refuse");
    assert!(matches!(
        err,
        RulepackError::MissingSelectingFixture { rule } if rule == "bare"
    ));
}

#[test]
fn refuses_positive_fixture_that_does_not_select() {
    let json = r#"{
  "pair": {"source": "go", "target": "rust"},
  "rules": [{
"id": "orphan",
"version": "0",
"precondition": "unit_present",
"construction": "pass_through",
"selecting_fixtures": [{"id": "positive", "unit": "u", "selects": true}]
  }],
  "applies": {}
}"#;
    let err = LoadedRulePack::load_from_str(json)
        .expect_err("a positive fixture must be selected by applies");
    assert!(matches!(
        err,
        RulepackError::FixtureExpectationMismatch {
            rule,
            fixture,
            expected: true,
            actual: false,
            ..
        } if rule == "orphan" && fixture == "positive"
    ));
}

#[test]
fn refuses_negative_fixture_that_selects() {
    let json = r#"{
  "pair": {"source": "go", "target": "rust"},
  "rules": [{
"id": "unexpected",
"version": "0",
"precondition": "unit_present",
"construction": "pass_through",
"selecting_fixtures": [
  {"id": "positive", "unit": "v", "selects": true},
  {"id": "negative", "unit": "u", "selects": false}
]
  }],
  "applies": {"u": ["unexpected"], "v": ["unexpected"]}
}"#;
    let err = LoadedRulePack::load_from_str(json)
        .expect_err("a negative fixture must not be selected by applies");
    assert!(matches!(
        err,
        RulepackError::FixtureExpectationMismatch {
            rule,
            fixture,
            expected: false,
            actual: true,
            ..
        } if rule == "unexpected" && fixture == "negative"
    ));
}

#[test]
fn refuses_rule_with_only_negative_fixtures() {
    let json = r#"{
  "pair": {"source": "go", "target": "rust"},
  "rules": [{
"id": "never",
"version": "0",
"precondition": "unit_present",
"construction": "pass_through",
"selecting_fixtures": [{"id": "negative", "unit": "u", "selects": false}]
  }],
  "applies": {}
}"#;
    let err = LoadedRulePack::load_from_str(json)
        .expect_err("every loaded rule needs a positive fixture");
    assert!(matches!(
        err,
        RulepackError::NoPositiveFixture {
            rule,
            fixture_count: 1,
        } if rule == "never"
    ));
}

#[test]
fn refuses_declared_semantics_the_engine_does_not_implement() {
    for field in ["required_diagnostics", "proof_obligations"] {
        let json = format!(
            r#"{{"pair":{{"source":"go","target":"rust"}},"rules":[{{"id":"r","version":"0",
               "precondition":"unit_present","construction":"pass_through","{field}":["x"],
               "selecting_fixtures":[{{"id":"f","unit":"u","selects":true}}]}}],
               "applies":{{"u":["r"]}}}}"#
        );
        let err = LoadedRulePack::load_from_str(&json)
            .expect_err("declared-but-unimplemented semantics must refuse");
        assert!(
            matches!(err, RulepackError::UnimplementedSemantics { field: f, .. } if f == field),
            "{field}: {err}"
        );
    }
}

#[test]
fn refuses_precedence_that_disagrees_with_declaration_order() {
    let json = r#"{"pair":{"source":"go","target":"rust"},"rules":[
        {"id":"first","version":"0","precondition":"unit_present","construction":"pass_through",
         "precedence":10,"selecting_fixtures":[{"id":"a","unit":"u","selects":true}]},
        {"id":"second","version":"0","precondition":"unit_present","construction":"pass_through",
         "precedence":5,"selecting_fixtures":[{"id":"b","unit":"u","selects":true}]}],
        "applies":{"u":["first","second"]}}"#;
    let err = LoadedRulePack::load_from_str(json).expect_err("out-of-order precedence refuses");
    assert!(matches!(
        err,
        RulepackError::PrecedenceDisagreesWithOrder { ref rule, .. } if rule == "second"
    ));
}

#[test]
fn refuses_a_conflict_policy_with_no_implementation() {
    let json = r#"{"pair":{"source":"go","target":"rust"},"rules":[
        {"id":"r","version":"0","precondition":"unit_present","construction":"pass_through",
         "conflict":"last_wins","selecting_fixtures":[{"id":"f","unit":"u","selects":true}]}],
        "applies":{"u":["r"]}}"#;
    let err = LoadedRulePack::load_from_str(json).expect_err("unimplemented policy refuses");
    assert!(matches!(err, RulepackError::UnknownConflictPolicy { .. }));
}

#[test]
fn refuses_a_deferral_without_a_recorded_reason() {
    let json = r#"{"pair":{"source":"go","target":"rust"},
        "deferred_kinds":[{"kind":"var","reason":"   "}],
        "rules":[{"id":"r","version":"0","precondition":"unit_present",
         "construction":"pass_through","selecting_fixtures":[{"id":"f","unit":"u","selects":true}]}],
        "applies":{"u":["r"]}}"#;
    let err = LoadedRulePack::load_from_str(json).expect_err("reasonless deferral refuses");
    assert!(matches!(
        err,
        RulepackError::Schema {
            field: "deferred_kinds[].reason"
        }
    ));
}

#[test]
fn refuses_a_kind_that_is_both_captured_and_deferred() {
    let json = r#"{"pair":{"source":"go","target":"rust"},
        "deferred_kinds":[{"kind":"const","reason":"not yet"}],
        "rules":[{"id":"r","version":"0","precondition":"unit_present","captures":["const"],
         "construction":"rust_const","selecting_fixtures":[{"id":"f","unit":"u","selects":true}]}],
        "applies":{"u":["r"]}}"#;
    let err = LoadedRulePack::load_from_str(json).expect_err("contradiction refuses");
    assert!(matches!(
        err,
        RulepackError::DeferredKindAlsoCaptured { .. }
    ));
}

#[test]
fn refuses_an_unknown_key_rather_than_ignoring_it() {
    let json = r#"{"pair":{"source":"go","target":"rust"},"type_map_override":{},
        "rules":[{"id":"r","version":"0","precondition":"unit_present",
         "construction":"pass_through","selecting_fixtures":[{"id":"f","unit":"u","selects":true}]}],
        "applies":{"u":["r"]}}"#;
    let err = LoadedRulePack::load_from_str(json).expect_err("unknown key refuses");
    assert!(matches!(err, RulepackError::Parse { .. }), "{err}");
}
