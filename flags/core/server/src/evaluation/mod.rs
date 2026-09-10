//! evaluation subsystem for the `flags` capability server.

pub use flags_evaluation_domain::{
    AttrValue, Condition, EvalErrorCode, Evaluation, EvaluationContext, Flag, FlagKey, FlagSource,
    FlagSourceError, FlagValue, Operand, Operator, Reason, Rollout, Rule, RuleOutcome,
    TOTAL_BASIS_POINTS, Variant, VariantKey, evaluate,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_evaluation_seam_resolves_through_domain_engine() {
        // Smoke test that the server's evaluation seam reaches the deterministic domain engine.
        let flag = Flag {
            key: "smoke".into(),
            enabled: false,
            variants: vec![
                Variant {
                    key: "on".into(),
                    value: FlagValue::Bool(true),
                },
                Variant {
                    key: "off".into(),
                    value: FlagValue::Bool(false),
                },
            ],
            rules: vec![],
            default_rollout: None,
            default_variant: "off".into(),
            off_variant: "off".into(),
        };
        let ev = evaluate(&flag, &EvaluationContext::for_key("u1"));
        assert_eq!(ev.reason, Reason::Disabled);
        assert_eq!(ev.variant, "off");
    }
}
