//! The Cedar seed held to the REAL engine: the schema parses, the
//! policy set passes the STRICT validator, and a permit is present — a
//! policy set that cannot permit anything is a nullified file, not a
//! posture.

use std::str::FromStr;

use cedar_policy::{PolicySet, Schema, ValidationMode, Validator};

const SCHEMA_SRC: &str = include_str!("../../../cedar/foundry.cedarschema");
const POLICIES_SRC: &str = include_str!("../../../cedar/foundry-policies.cedar");

fn schema() -> Schema {
    let (schema, _warnings) =
        Schema::from_cedarschema_str(SCHEMA_SRC).expect("foundry.cedarschema must parse");
    schema
}

fn policy_set() -> PolicySet {
    PolicySet::from_str(POLICIES_SRC).expect("foundry-policies.cedar must parse")
}
/// The seed must pass the STRICT validator — a policy that references a
/// vocabulary the schema does not declare never ships.
#[test]
fn the_seed_strict_validates() {
    let validator = Validator::new(schema());
    let result = validator.validate(&policy_set(), ValidationMode::Strict);
    assert!(
        result.validation_passed(),
        "strict validation failed: {:?}",
        result.validation_errors().collect::<Vec<_>>(),
    );
}

/// No bare forbid: a policy set that cannot permit anything is a
/// nullified file, not a posture.
#[test]
fn the_seed_contains_a_permit() {
    assert!(
        policy_set()
            .policies()
            .any(|policy| { format!("{:?}", policy.effect()).contains("Permit") }),
        "the policy set must carry at least one permit",
    );
}

/// The tenant-isolation belt stays present: the structural cross-tenant
/// forbid is defense-in-depth the behavioral suite cannot pin — a mutant
/// deleting it left every test green, because the sole permit's own
/// tenant conjunct masks the deletion under today's policy set. Its
/// PRESENCE is therefore pinned here, mirroring the permit pin, so the
/// belt cannot be silently removed before some future permit lacks the
/// conjunct.
#[test]
fn the_seed_keeps_the_cross_tenant_forbid() {
    assert!(
        policy_set()
            .policies()
            .any(|policy| { format!("{:?}", policy.effect()).contains("Forbid") }),
        "the structural cross-tenant forbid must stay in the seed",
    );
}

/// The read surface has its own action. The shell's module card is gated
/// on it, and gating a card on the WRITE action would hide the capability
/// from a read-only operator — so the vocabulary carries both.
#[test]
fn the_seed_declares_the_read_action() {
    let referencing_the_read_action = PolicySet::from_str(
        r#"permit(
            principal is Principal,
            action == Action::"foundry.ontology.use",
            resource is OntologyObject
        );"#,
    )
    .expect("the probe policy parses");
    let result =
        Validator::new(schema()).validate(&referencing_the_read_action, ValidationMode::Strict);
    assert!(
        result.validation_passed(),
        "the schema must declare the read action: {:?}",
        result.validation_errors().collect::<Vec<_>>(),
    );
}

/// A declared action with no permit is a vocabulary entry nothing can
/// ever satisfy. The read surface must be reachable by the operator the
/// seed grants, or the shell renders a card no one can open.
#[test]
fn the_read_action_carries_a_permit() {
    assert!(
        policy_set().policies().any(|policy| {
            let rendered = format!("{policy}");
            rendered.contains("foundry.ontology.use")
                && format!("{:?}", policy.effect()).contains("Permit")
        }),
        "the read action must carry a permit of its own",
    );
}

/// Tenant isolation is structural, not per-action: the belt must cover
/// every action the vocabulary declares, so a future action cannot be
/// born outside it. Deleting the belt is caught by the presence pin
/// above; leaving it scoped to one action is what this pins.
#[test]
fn the_cross_tenant_forbid_covers_the_read_action() {
    assert!(
        policy_set().policies().any(|policy| {
            let rendered = format!("{policy}");
            format!("{:?}", policy.effect()).contains("Forbid")
                && (rendered.contains("foundry.ontology.use") || !rendered.contains("action =="))
        }),
        "the cross-tenant forbid must reach the read action, not only invocation",
    );
}
