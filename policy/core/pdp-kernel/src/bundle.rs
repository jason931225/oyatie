use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use shared_platform_contracts_kernel::pdp::{EntityRef, PolicyVersion};

/// A named policy template as compiled into a bundle by the policy store.
/// The id is explicit (templates are linked by id, never by source position).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateSrc {
    pub template_id: String, // data_class: INTERNAL_ONLY
    pub src: String,         // data_class: INTERNAL_ONLY
}

/// A PBAC template instantiation (policy-as-data): the policy store links a
/// template per grant instead of authoring ad-hoc policies. Precedent:
/// Amazon Verified Permissions policy templates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateLink {
    /// Id of the template being instantiated (e.g. `pbac-resource-read-grant`).
    pub template_id: String, // data_class: INTERNAL_ONLY
    /// Unique id of this instantiation; appears in determining-policy ids.
    pub link_id: String, // data_class: INTERNAL_ONLY
    pub principal: EntityRef, // data_class: TENANT_SCOPED
    pub resource: EntityRef,  // data_class: TENANT_SCOPED
}

/// A policy bundle as pushed by the policy-store control plane.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyBundle {
    /// Opaque policy-store version token (content address upstream).
    pub version: PolicyVersion, // data_class: INTERNAL_ONLY
    /// Cedar-schema source for the entity/action model.
    pub schema_src: String, // data_class: INTERNAL_ONLY
    /// Static policy set source (structural forbid + RBAC/ABAC policies).
    pub policies_src: String, // data_class: INTERNAL_ONLY
    /// `tenant_id` -> tenant-scoped Cedar policy source. Cross-tenant
    /// isolation for an overlay permit is enforced at RUNTIME by the global
    /// `structural-tenant-isolation` forbid, not by any load-time check.
    #[serde(default)]
    pub tenant_policies: BTreeMap<String, String>, // data_class: TENANT_SCOPED
    /// Named templates for PBAC instantiations.
    pub templates: Vec<TemplateSrc>, // data_class: INTERNAL_ONLY
    /// PBAC template instantiations compiled into this bundle.
    pub template_links: Vec<TemplateLink>, // data_class: TENANT_SCOPED
    /// Contract-action-slug -> engine-action-uid map, compiled by the policy
    /// store (contract actions are slug-form per the locked PDP contract;
    /// engine action ids are namespaced uids). Unknown slugs fail closed.
    pub action_map: BTreeMap<String, String>, // data_class: INTERNAL_ONLY
}
