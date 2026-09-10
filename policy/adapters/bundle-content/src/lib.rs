//! Authored policy-bundle content identity and candidate construction.
#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use policy_pdp_kernel::{PolicyBundle, TemplateLink, TemplateSrc};
use serde::{Deserialize, Serialize};
use shared_audit_digest_adapter_awslc::Sha256Digester;
use shared_audit_event_kernel::Digester;
use shared_platform_contracts_kernel::pdp::PolicyVersion;

#[derive(Debug)]
pub enum ContentIdentityError {
    Encoding { detail: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicySource {
    pub schema_src: String,
    pub policies_src: String,
    pub tenant_policies: BTreeMap<String, String>,
    pub templates: Vec<TemplateSrc>,
    pub template_links: Vec<TemplateLink>,
    pub action_map: BTreeMap<String, String>,
}

impl PolicySource {
    /// # Errors
    /// Returns a serialization or version-contract refusal.
    pub fn content_version(&self) -> Result<PolicyVersion, ContentIdentityError> {
        let digest = content_digest(b"oyatie-policy-source/v1\0", self)?;
        PolicyVersion::new(digest).map_err(|violations| ContentIdentityError::Encoding {
            detail: format!("content identity violates version contract: {violations:?}"),
        })
    }

    /// # Errors
    /// Returns a content-identity refusal.
    pub fn candidate(&self) -> Result<PolicyBundle, ContentIdentityError> {
        Ok(PolicyBundle {
            version: self.content_version()?,
            schema_src: self.schema_src.clone(),
            policies_src: self.policies_src.clone(),
            tenant_policies: self.tenant_policies.clone(),
            templates: self.templates.clone(),
            template_links: self.template_links.clone(),
            action_map: self.action_map.clone(),
        })
    }
}

impl From<&PolicyBundle> for PolicySource {
    fn from(bundle: &PolicyBundle) -> Self {
        Self {
            schema_src: bundle.schema_src.clone(),
            policies_src: bundle.policies_src.clone(),
            tenant_policies: bundle.tenant_policies.clone(),
            templates: bundle.templates.clone(),
            template_links: bundle.template_links.clone(),
            action_map: bundle.action_map.clone(),
        }
    }
}

/// Compute a domain-separated digest over the serialized value.
///
/// # Errors
/// Returns the serializer's refusal with its original detail.
pub fn content_digest(
    domain: &[u8],
    value: &impl Serialize,
) -> Result<String, ContentIdentityError> {
    let mut bytes = domain.to_vec();
    serde_json::to_writer(&mut bytes, value).map_err(|error| ContentIdentityError::Encoding {
        detail: error.to_string(),
    })?;
    Ok(Sha256Digester.digest_hex(&bytes))
}
