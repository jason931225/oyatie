use std::collections::HashSet;
use std::str::FromStr;

use cedar_policy::{Effect, PolicyId, PolicySet, Schema, ValidationMode, Validator};
use shared_pdp_kernel::PdpError;

/// A global `@id` is a Cedar id which never contains `/` in the seed corpus,
/// so namespacing overlay ids this way keeps them disjoint from global ids and
/// from each other.
const OVERLAY_ID_SEP: &str = "/";

pub(super) fn compile_tenant_overlay(
    tenant_id: &str,
    overlay_src: &str,
    global: &PolicySet,
    schema: &Schema,
    known_tenants: &HashSet<&str>,
) -> Result<PolicySet, PdpError> {
    let parsed = PolicySet::from_str(overlay_src).map_err(|e| PdpError::BundleRejected {
        detail: format!("tenant {tenant_id} overlay rejected: {e}"),
    })?;
    // Start from a clone of the global set so the structural forbid (and every
    // global permit) is present in the per-tenant decision — ONE algorithm.
    let mut merged = global.clone();
    for policy in parsed.policies() {
        let authored = match policy.annotation("id") {
            Some(id) => policy.new_id(PolicyId::new(id)),
            None => policy.clone(),
        };
        reject_misauthored_overlay_hygiene_only(tenant_id, &authored, known_tenants)?;
        let namespaced_id = format!("{tenant_id}{OVERLAY_ID_SEP}{}", authored.id());
        let namespaced = authored.new_id(PolicyId::new(&namespaced_id));
        merged
            .add(namespaced)
            .map_err(|e| PdpError::BundleRejected {
                detail: format!("tenant {tenant_id} overlay policy {namespaced_id} rejected: {e}"),
            })?;
    }
    strict_validate_merged_set(schema, &merged, tenant_id)?;
    Ok(merged)
}

fn strict_validate_merged_set(
    schema: &Schema,
    merged: &PolicySet,
    tenant_id: &str,
) -> Result<(), PdpError> {
    let validation = Validator::new(schema.clone()).validate(merged, ValidationMode::Strict);
    if validation.validation_passed() {
        return Ok(());
    }
    let errors: Vec<String> = validation
        .validation_errors()
        .map(|e| e.to_string())
        .collect();
    Err(PdpError::BundleRejected {
        detail: format!(
            "tenant {tenant_id} overlay strict validation failed: {}",
            errors.join("; ")
        ),
    })
}

/// Rejects, fail-closed: a policy whose EST names a KNOWN foreign tenant id as
/// a string literal, and a `permit` that [`permit_is_tenant_confined`] does not
/// accept.
fn reject_misauthored_overlay_hygiene_only(
    tenant_id: &str,
    policy: &cedar_policy::Policy,
    known_tenants: &HashSet<&str>,
) -> Result<(), PdpError> {
    let json = policy.to_json().map_err(|e| PdpError::BundleRejected {
        detail: format!(
            "tenant {tenant_id} overlay policy {} not introspectable: {e}",
            policy.id()
        ),
    })?;
    if let Some(foreign) = first_foreign_tenant_literal(&json, tenant_id, known_tenants) {
        return Err(PdpError::BundleRejected {
            detail: format!(
                "tenant {tenant_id} overlay policy {} names foreign tenant {foreign:?} — \
                 an overlay may never reference another tenant",
                policy.id()
            ),
        });
    }
    // A forbid can only ever DENY; it cannot grant across (or within) a tenant.
    if policy.effect() == Effect::Forbid {
        return Ok(());
    }
    if permit_is_tenant_confined(&json) {
        Ok(())
    } else {
        Err(PdpError::BundleRejected {
            detail: format!(
                "tenant {tenant_id} overlay permit {} is not tenant-confined: every overlay \
                 permit must carry the same-tenant guard \
                 `principal.tenant_id == resource.tenant_id` as a top-level conjunct of a \
                 `when` clause, with no `unless` clause and no enclosing `||`/`!`",
                policy.id()
            ),
        })
    }
}

/// Sound, conservative, fail-closed check that a `permit` EST is confined to a
/// single tenant by the same-tenant guard. Operates on the policy EST that
/// cedar-policy emits via `Policy::to_json`:
///
/// ```text
/// { "effect": "permit", ..., "conditions": [ { "kind": "when"|"unless", "body": <expr> }, ... ] }
/// ```
///
/// A permit is accepted ONLY when BOTH hold:
/// 1. it has NO `unless` clause (an `unless` can defeat any `when` guard), and
/// 2. at least one `when` clause has the same-tenant equality as a member of
///    its TOP-LEVEL CONJUNCTIVE SPINE — the leaves reached by descending only
///    through `&&` nodes. The walk NEVER descends into `||`, `!`, or any other
///    operator, so a guard buried under a disjunction/negation (where it does
///    not unconditionally bind) is NOT accepted.
///
/// Cedar AND-s all `when` clauses together, so a guard that is an unconditional
/// conjunct of any one `when` clause confines the whole permit.
fn permit_is_tenant_confined(policy_json: &serde_json::Value) -> bool {
    let Some(conditions) = policy_json
        .get("conditions")
        .and_then(serde_json::Value::as_array)
    else {
        return false;
    };
    let mut has_binding_when = false;
    for condition in conditions {
        let kind = condition.get("kind").and_then(serde_json::Value::as_str);
        let Some(body) = condition.get("body") else {
            return false;
        };
        match kind {
            // Any `unless` can flip the decision back to deny-by-omission for a
            // matching principal, so we cannot soundly accept a permit that
            // carries one. Fail closed.
            Some("unless") => return false,
            Some("when") => {
                if conjunctive_spine_has_same_tenant_guard(body) {
                    has_binding_when = true;
                }
            }
            _ => return false,
        }
    }
    has_binding_when
}

fn conjunctive_spine_has_same_tenant_guard(expr: &serde_json::Value) -> bool {
    if let Some(and) = expr.get("&&").and_then(serde_json::Value::as_object) {
        let left = and
            .get("left")
            .is_some_and(conjunctive_spine_has_same_tenant_guard);
        let right = and
            .get("right")
            .is_some_and(conjunctive_spine_has_same_tenant_guard);
        return left || right;
    }
    is_same_tenant_equality(expr)
}

fn is_same_tenant_equality(expr: &serde_json::Value) -> bool {
    let Some(eq) = expr.get("==").and_then(serde_json::Value::as_object) else {
        return false;
    };
    let (Some(left), Some(right)) = (eq.get("left"), eq.get("right")) else {
        return false;
    };
    let (Some(l), Some(r)) = (
        tenant_id_attr_access_var(left),
        tenant_id_attr_access_var(right),
    ) else {
        return false;
    };
    (l == "principal" && r == "resource") || (l == "resource" && r == "principal")
}

/// If `node` is an EST attribute access of the form
/// `{ ".": { "left": { "Var": V }, "attr": "tenant_id" } }`, return `V`.
fn tenant_id_attr_access_var(node: &serde_json::Value) -> Option<&str> {
    let access = node.get(".").and_then(serde_json::Value::as_object)?;
    if access.get("attr").and_then(serde_json::Value::as_str)? != "tenant_id" {
        return None;
    }
    access
        .get("left")?
        .get("Var")
        .and_then(serde_json::Value::as_str)
}

fn first_foreign_tenant_literal<'a>(
    node: &'a serde_json::Value,
    owning_tenant: &str,
    known_tenants: &HashSet<&str>,
) -> Option<&'a str> {
    if let Some(serde_json::Value::String(s)) = node.get("Value")
        && s.as_str() != owning_tenant
        && known_tenants.contains(s.as_str())
    {
        return Some(s.as_str());
    }
    match node {
        serde_json::Value::Object(map) => map
            .values()
            .find_map(|v| first_foreign_tenant_literal(v, owning_tenant, known_tenants)),
        serde_json::Value::Array(items) => items
            .iter()
            .find_map(|v| first_foreign_tenant_literal(v, owning_tenant, known_tenants)),
        _ => None,
    }
}
