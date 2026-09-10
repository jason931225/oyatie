//! One bundle admission path shared by validation, initial load, and reload.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::str::FromStr;

use cedar_policy::{
    EntityUid, PolicyId, PolicySet, Schema, SlotId, Template, ValidationMode, Validator,
};
use shared_pdp_kernel::{PdpError, PolicyBundle};
use shared_platform_contracts_kernel::pdp::{AuthorizationRequest, EntityRef, PolicyVersion};

use super::LoadedBundle;
use super::entity::entity_uid;
use super::overlay::compile_tenant_overlay;

/// # Errors
/// Returns the load-time refusal when bundle metadata, schema, policies,
/// templates, links, overlays, or action mappings cannot be admitted.
pub fn validate_bundle(bundle: &PolicyBundle) -> Result<(), PdpError> {
    compile(bundle).map(|_| ())
}

pub(super) fn compile(bundle: &PolicyBundle) -> Result<LoadedBundle, PdpError> {
    PolicyVersion::new(bundle.version.as_str()).map_err(|violations| PdpError::BundleRejected {
        detail: format!("bundle version token rejected: {violations:?}"),
    })?;
    let source_identity = serde_json::to_vec(bundle).map_err(|error| PdpError::BundleRejected {
        detail: format!("bundle input identity rejected: {error}"),
    })?;
    let (schema, _warnings) =
        Schema::from_cedarschema_str(&bundle.schema_src).map_err(|e| PdpError::BundleRejected {
            detail: format!("schema rejected: {e}"),
        })?;
    let action_map = admit_actions(&bundle.action_map, &schema)?;
    let parsed =
        PolicySet::from_str(&bundle.policies_src).map_err(|e| PdpError::BundleRejected {
            detail: format!("static policies rejected: {e}"),
        })?;
    let mut policy_set = PolicySet::new();
    for policy in parsed.policies() {
        let policy = match policy.annotation("id") {
            Some(id) => policy.new_id(PolicyId::new(id)),
            None => policy.clone(),
        };
        let policy_id = policy.id().clone();
        policy_set
            .add(policy)
            .map_err(|e| PdpError::BundleRejected {
                detail: format!("static policy {policy_id} rejected: {e}"),
            })?;
    }
    for template in &bundle.templates {
        let parsed = Template::parse(
            Some(PolicyId::new(&template.template_id)),
            template.src.as_str(),
        )
        .map_err(|e| PdpError::BundleRejected {
            detail: format!("template {} rejected: {e}", template.template_id),
        })?;
        policy_set
            .add_template(parsed)
            .map_err(|e| PdpError::BundleRejected {
                detail: format!("template {} rejected: {e}", template.template_id),
            })?;
    }
    for link in &bundle.template_links {
        let mut values = HashMap::new();
        values.insert(SlotId::principal(), entity_uid(&link.principal)?);
        values.insert(SlotId::resource(), entity_uid(&link.resource)?);
        policy_set
            .link(
                PolicyId::new(&link.template_id),
                PolicyId::new(&link.link_id),
                values,
            )
            .map_err(|e| PdpError::BundleRejected {
                detail: format!("template link {} rejected: {e}", link.link_id),
            })?;
    }
    let validation = Validator::new(schema.clone()).validate(&policy_set, ValidationMode::Strict);
    if !validation.validation_passed() {
        let errors: Vec<String> = validation
            .validation_errors()
            .map(|e| e.to_string())
            .collect();
        return Err(PdpError::BundleRejected {
            detail: format!("strict validation failed: {}", errors.join("; ")),
        });
    }
    let known_tenants: HashSet<&str> = bundle.tenant_policies.keys().map(String::as_str).collect();
    let mut tenant_policy_sets = BTreeMap::new();
    for (tenant_id, overlay_src) in &bundle.tenant_policies {
        let merged =
            compile_tenant_overlay(tenant_id, overlay_src, &policy_set, &schema, &known_tenants)?;
        tenant_policy_sets.insert(tenant_id.clone(), merged);
    }
    Ok(LoadedBundle {
        version: bundle.version.clone(),
        source_identity,
        schema,
        policy_set,
        tenant_policy_sets,
        action_map,
    })
}

fn admit_actions(
    mappings: &BTreeMap<String, String>,
    schema: &Schema,
) -> Result<BTreeMap<String, EntityUid>, PdpError> {
    let declared: HashSet<EntityUid> = schema.actions().cloned().collect();
    mappings
        .iter()
        .map(|(slug, source)| {
            validate_action_slug_via_request_contract(slug)?;
            let uid = EntityUid::from_str(source).map_err(|error| PdpError::BundleRejected {
                detail: format!("action map entry {slug:?} rejected: {error}"),
            })?;
            if !declared.contains(&uid) {
                return Err(PdpError::BundleRejected {
                    detail: format!("action map entry {slug:?} names undeclared action {uid}"),
                });
            }
            Ok((slug.clone(), uid))
        })
        .collect()
}

fn validate_action_slug_via_request_contract(slug: &str) -> Result<(), PdpError> {
    let request = AuthorizationRequest {
        request_id: "action-map-admission".to_owned(),
        tenant_id: "policy".to_owned(),
        principal: EntityRef {
            entity_type: "Principal".to_owned(),
            entity_id: "admission".to_owned(),
        },
        action: slug.to_owned(),
        resource: EntityRef {
            entity_type: "Resource".to_owned(),
            entity_id: "admission".to_owned(),
        },
        context: BTreeMap::new(),
        min_policy_version: None,
    };
    request
        .validate()
        .map_err(|violations| PdpError::BundleRejected {
            detail: format!("action map slug {slug:?} rejected: {violations:?}"),
        })
}
