//! `flags-evaluation-domain` — the cloud-agnostic, deterministic flag-evaluation
//! core for the `flags` capability.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod bucket;
pub mod engine;
pub mod model;
pub mod port;

pub use engine::{EvalErrorCode, Evaluation, Reason, evaluate};
pub use model::{
    AttrValue, Condition, EvaluationContext, Flag, FlagKey, FlagValue, Operand, Operator, Rollout,
    Rule, RuleOutcome, TOTAL_BASIS_POINTS, Variant, VariantKey,
};
pub use port::{FlagSource, FlagSourceError};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn bool_variants() -> Vec<Variant> {
        vec![
            Variant {
                key: "on".into(),
                value: FlagValue::Bool(true),
            },
            Variant {
                key: "off".into(),
                value: FlagValue::Bool(false),
            },
        ]
    }

    fn base_flag() -> Flag {
        Flag {
            key: "checkout.new-cart".into(),
            enabled: true,
            variants: bool_variants(),
            rules: vec![],
            default_rollout: None,
            default_variant: "off".into(),
            off_variant: "off".into(),
        }
    }

    #[test]
    fn disabled_flag_serves_off_variant() {
        let mut flag = base_flag();
        flag.enabled = false;
        let ev = evaluate(&flag, &EvaluationContext::for_key("user-1"));
        assert_eq!(ev.variant, "off");
        assert_eq!(ev.value, FlagValue::Bool(false));
        assert_eq!(ev.reason, Reason::Disabled);
    }

    #[test]
    fn no_rules_serves_default_variant() {
        let mut flag = base_flag();
        flag.default_variant = "on".into();
        let ev = evaluate(&flag, &EvaluationContext::for_key("user-1"));
        assert_eq!(ev.variant, "on");
        assert_eq!(ev.reason, Reason::Default);
    }

    #[test]
    fn targeting_rule_eq_matches_and_serves_fixed_variant() {
        let mut flag = base_flag();
        flag.rules = vec![Rule {
            id: "beta-tenants".into(),
            conditions: vec![Condition {
                attribute: "tenant".into(),
                operator: Operator::Eq,
                operand: Operand::Value(AttrValue::Str("acme".into())),
            }],
            outcome: RuleOutcome::Fixed("on".into()),
        }];

        let matching =
            EvaluationContext::for_key("u1").with_attr("tenant", AttrValue::Str("acme".into()));
        let ev = evaluate(&flag, &matching);
        assert_eq!(ev.variant, "on");
        assert_eq!(ev.reason, Reason::TargetingMatch);

        let non_matching =
            EvaluationContext::for_key("u1").with_attr("tenant", AttrValue::Str("other".into()));
        let ev2 = evaluate(&flag, &non_matching);
        assert_eq!(ev2.variant, "off");
        assert_eq!(ev2.reason, Reason::Default);
    }

    #[test]
    fn in_operator_matches_set_membership() {
        let mut flag = base_flag();
        flag.rules = vec![Rule {
            id: "internal".into(),
            conditions: vec![Condition {
                attribute: "ring".into(),
                operator: Operator::In,
                operand: Operand::Set(vec!["canary".into(), "internal".into()]),
            }],
            outcome: RuleOutcome::Fixed("on".into()),
        }];
        let ctx =
            EvaluationContext::for_key("u1").with_attr("ring", AttrValue::Str("canary".into()));
        assert_eq!(evaluate(&flag, &ctx).variant, "on");
        let ctx2 = EvaluationContext::for_key("u1").with_attr("ring", AttrValue::Str("ga".into()));
        assert_eq!(evaluate(&flag, &ctx2).variant, "off");
    }

    #[test]
    fn first_matching_rule_wins() {
        let mut flag = base_flag();
        flag.variants.push(Variant {
            key: "maybe".into(),
            value: FlagValue::Bool(true),
        });
        flag.rules = vec![
            Rule {
                id: "r1".into(),
                conditions: vec![Condition {
                    attribute: "plan".into(),
                    operator: Operator::Eq,
                    operand: Operand::Value(AttrValue::Str("pro".into())),
                }],
                outcome: RuleOutcome::Fixed("on".into()),
            },
            Rule {
                id: "r2".into(),
                conditions: vec![Condition {
                    attribute: "plan".into(),
                    operator: Operator::Eq,
                    operand: Operand::Value(AttrValue::Str("pro".into())),
                }],
                outcome: RuleOutcome::Fixed("maybe".into()),
            },
        ];
        let ctx = EvaluationContext::for_key("u1").with_attr("plan", AttrValue::Str("pro".into()));
        assert_eq!(evaluate(&flag, &ctx).variant, "on", "earlier rule must win");
    }

    #[test]
    fn fifty_fifty_rollout_is_sticky_and_split() {
        let mut flag = base_flag();
        flag.default_rollout = Some(Rollout {
            buckets: vec![("on".into(), 5_000), ("off".into(), 5_000)],
            salt: String::new(),
        });

        // Stickiness: same subject → same variant on repeated evaluation.
        let ctx = EvaluationContext::for_key("user-stable");
        let first = evaluate(&flag, &ctx);
        let second = evaluate(&flag, &ctx);
        assert_eq!(first.variant, second.variant);
        assert_eq!(first.reason, Reason::Split);

        // Split: across many subjects, both variants appear and the split is roughly even.
        let mut on = 0u32;
        let n = 20_000u32;
        for i in 0..n {
            let c = EvaluationContext::for_key(format!("u{i}"));
            if evaluate(&flag, &c).variant == "on" {
                on += 1;
            }
        }
        let ratio = f64::from(on) / f64::from(n);
        assert!(
            (0.45..=0.55).contains(&ratio),
            "rollout split skewed: {ratio}"
        );
    }

    #[test]
    fn rollout_unallocated_remainder_falls_through_to_default() {
        // Only 10% allocated to "on"; the other 90% is unallocated and must serve the default.
        let mut flag = base_flag();
        flag.default_variant = "off".into();
        flag.default_rollout = Some(Rollout {
            buckets: vec![("on".into(), 1_000)],
            salt: String::new(),
        });
        let mut on = 0u32;
        let mut off = 0u32;
        for i in 0..20_000u32 {
            let c = EvaluationContext::for_key(format!("u{i}"));
            match evaluate(&flag, &c).variant.as_str() {
                "on" => on += 1,
                "off" => off += 1,
                other => panic!("unexpected variant {other}"),
            }
        }
        let on_ratio = f64::from(on) / f64::from(on + off);
        assert!(
            (0.07..=0.13).contains(&on_ratio),
            "expected ~10% on, got {on_ratio}"
        );
    }

    #[test]
    fn anonymous_empty_key_still_buckets_deterministically() {
        let mut flag = base_flag();
        flag.default_rollout = Some(Rollout {
            buckets: vec![("on".into(), 5_000), ("off".into(), 5_000)],
            salt: String::new(),
        });
        let ctx = EvaluationContext::for_key("");
        assert_eq!(evaluate(&flag, &ctx).variant, evaluate(&flag, &ctx).variant);
    }

    #[test]
    fn unknown_variant_fails_closed_with_error_reason() {
        let mut flag = base_flag();
        flag.rules = vec![Rule {
            id: "broken".into(),
            conditions: vec![],
            outcome: RuleOutcome::Fixed("nonexistent".into()),
        }];
        let ev = evaluate(&flag, &EvaluationContext::for_key("u1"));
        assert_eq!(ev.reason, Reason::Error);
        assert_eq!(ev.error_code, Some(EvalErrorCode::UnknownVariant));
        // Fail-closed served value is the off variant (safe).
        assert_eq!(ev.variant, "off");
    }

    #[test]
    fn no_variants_fails_closed() {
        let flag = Flag {
            key: "empty".into(),
            enabled: true,
            variants: vec![],
            rules: vec![],
            default_rollout: None,
            default_variant: "x".into(),
            off_variant: "x".into(),
        };
        let ev = evaluate(&flag, &EvaluationContext::for_key("u1"));
        assert_eq!(ev.reason, Reason::Error);
        assert_eq!(ev.error_code, Some(EvalErrorCode::NoVariants));
    }

    #[test]
    fn object_variant_value_roundtrips() {
        let mut attrs = BTreeMap::new();
        attrs.insert("color".to_string(), "blue".to_string());
        let flag = Flag {
            key: "ui.theme".into(),
            enabled: true,
            variants: vec![Variant {
                key: "v".into(),
                value: FlagValue::Object(attrs.clone()),
            }],
            rules: vec![],
            default_rollout: None,
            default_variant: "v".into(),
            off_variant: "v".into(),
        };
        let ev = evaluate(&flag, &EvaluationContext::for_key("u1"));
        assert_eq!(ev.value, FlagValue::Object(attrs));
    }

    mod port_seam {
        use super::*;

        struct StaticSource(Vec<Flag>);
        impl FlagSource for StaticSource {
            fn get_flag(&self, key: &FlagKey) -> Result<Option<Flag>, FlagSourceError> {
                Ok(self.0.iter().find(|f| &f.key == key).cloned())
            }
        }

        #[test]
        fn flag_source_port_drives_engine_end_to_end() {
            let source = StaticSource(vec![base_flag()]);
            let key = "checkout.new-cart".to_string();
            let flag = source.get_flag(&key).unwrap().expect("flag present");
            let ev = evaluate(&flag, &EvaluationContext::for_key("u1"));
            assert_eq!(ev.variant, "off");
            assert_eq!(source.get_flag(&"absent".to_string()).unwrap(), None);
        }
    }
}
