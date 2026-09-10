#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(dead_code)]

use application_app::{
    AdversarialKind, AutonomyTier, CapabilityInvocationPrincipal, DataClass, EvalCaseInput,
    EvalMetric, EvalRunInput, EvalSetInput, Foundation, PolicyEffect, PolicyRuleInput, PolicyScope,
    PolicyVersion, PrivacyDataClass, REQUIRED_LINGUISTIC_COHORT_LOCALES,
};

pub fn seed_passing_eval(foundation: &mut Foundation, capability_id: &str) {
    foundation
        .register_capability_eval_set(passing_eval_set(capability_id))
        .expect("eval set gates capability publish");
    foundation
        .record_capability_eval_run(passing_eval_run(capability_id))
        .expect("passing eval run gates capability publish");
}

pub fn seed_invocation_policy(foundation: &mut Foundation, tenant_id: &str, role: &str) {
    foundation
        .publish_policy(PolicyVersion {
            policy_id: format!("pol_{}_{}_invoke", tenant_id, role.replace('-', "_")),
            version: "1.0.0".into(),
            scope: PolicyScope::Tenant(tenant_id.into()),
            supersedes: None,
            rules: vec![PolicyRuleInput {
                effect: PolicyEffect::Allow,
                principal_role: role.into(),
                action: "foundry.capability.invoke".into(),
                resource_prefix: "capability:".into(),
                required_attribute: None,
                annotations: vec![],
            }],
        })
        .expect("tenant invocation policy is valid");
}

fn passing_eval_set(capability_id: &str) -> EvalSetInput {
    let mut cases = vec![
        eval_case("case-alpha", REQUIRED_LINGUISTIC_COHORT_LOCALES[0], None),
        eval_case("case-beta", REQUIRED_LINGUISTIC_COHORT_LOCALES[1], None),
        eval_case("case-gamma", REQUIRED_LINGUISTIC_COHORT_LOCALES[2], None),
    ];
    for (case_id, kind) in [
        ("adv-prompt", AdversarialKind::PromptInjection),
        ("adv-class", AdversarialKind::DataClassViolation),
        ("adv-autonomy", AdversarialKind::AutonomyBypass),
        ("adv-tool", AdversarialKind::ToolExfiltration),
    ] {
        cases.push(eval_case(
            case_id,
            REQUIRED_LINGUISTIC_COHORT_LOCALES[0],
            Some(kind),
        ));
    }
    EvalSetInput {
        capability_id: capability_id.into(),
        version: "eval-v1".into(),
        metric: EvalMetric::ExactMatch,
        min_pass_rate_percent: 80,
        min_p95_score_percent: 80,
        signed: true,
        cases,
    }
}

fn passing_eval_run(capability_id: &str) -> EvalRunInput {
    EvalRunInput {
        capability_id: capability_id.into(),
        eval_set_version: "eval-v1".into(),
        pass_rate_percent: 95,
        p95_score_percent: 90,
        adversarial_passed: true,
        linguistic_passed: true,
        signed: true,
    }
}

fn eval_case(
    case_id: &str,
    locale: &str,
    adversarial_kind: Option<AdversarialKind>,
) -> EvalCaseInput {
    EvalCaseInput {
        case_id: case_id.into(),
        locale: locale.into(),
        input_ref: format!("inputs/{case_id}.json"),
        expected_ref: format!("expected/{case_id}.json"),
        adversarial_kind,
        deterministic_seed: Some(42),
    }
}

pub fn allow_capability_invocation(foundation: &mut Foundation, tenant_id: &str, role: &str) {
    let policy_id = format!("pol_{}_{}_invoke", tenant_id, role).replace('-', "_");
    foundation
        .publish_policy(PolicyVersion {
            policy_id,
            version: "1.0.0".into(),
            scope: PolicyScope::Tenant(tenant_id.into()),
            supersedes: None,
            rules: vec![PolicyRuleInput {
                effect: PolicyEffect::Allow,
                principal_role: role.into(),
                action: "foundry.capability.invoke".into(),
                resource_prefix: "capability:cap.".into(),
                required_attribute: None,
                annotations: vec![],
            }],
        })
        .expect("capability invocation policy is valid");
}

pub fn principal(
    tenant_id: &str,
    user_id: &str,
    autonomy_ceiling: AutonomyTier,
) -> CapabilityInvocationPrincipal {
    CapabilityInvocationPrincipal {
        tenant_id: tenant_id.into(),
        user_id: user_id.into(),
        autonomy_ceiling,
    }
}

pub fn privacy_data_class(data_class: DataClass) -> PrivacyDataClass {
    PrivacyDataClass::try_from(data_class).expect("test fixture uses privacy data class")
}
