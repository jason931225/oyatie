use std::collections::BTreeMap;
use std::sync::Arc;

use policy_pdp_cedar::CedarPdp;
use policy_pdp_kernel::{PdpRuntimeConfig, PdpRuntimeGuard, PolicyBundle, PolicyDecisionPoint};
use shared_platform_contracts_kernel::pdp::PolicyVersion;
use shared_ulid_id_kernel::SeededIdGenerator;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Surface {
    Invoke,
    Use,
}

impl Surface {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Invoke => "foundry.ontology.invoke",
            Self::Use => "foundry.ontology.use",
        }
    }

    /// The autonomy tier the surface asserts. Reading carries none — that
    /// is a property of acting, not of looking — and the read action's
    /// context does not declare the attribute.
    pub const fn autonomy_tier(self) -> Option<i64> {
        match self {
            Self::Invoke => Some(1),
            Self::Use => None,
        }
    }
}

const SCHEMA_SRC: &str = include_str!("../../../cedar/foundry.cedarschema");
const POLICIES_SRC: &str = include_str!("../../../cedar/foundry-policies.cedar");

pub const OPS_CONSOLE: &str = "ops-console";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PepError {
    BundleRejected { detail: String },
    Denied,
}

impl std::fmt::Display for PepError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BundleRejected { detail } => {
                write!(formatter, "the policy bundle was rejected: {detail}")
            }
            Self::Denied => write!(formatter, "the policy decision point refused"),
        }
    }
}

fn bundle(version: &str) -> Result<PolicyBundle, PepError> {
    Ok(PolicyBundle {
        version: PolicyVersion::new(version).map_err(|error| PepError::BundleRejected {
            detail: format!("{error:?}"),
        })?,
        schema_src: SCHEMA_SRC.to_owned(),
        policies_src: POLICIES_SRC.to_owned(),
        tenant_policies: BTreeMap::new(),
        templates: Vec::new(),
        template_links: Vec::new(),
        action_map: BTreeMap::from([
            (
                Surface::Invoke.slug().to_owned(),
                r#"Action::"InvokeAction""#.to_owned(),
            ),
            (
                Surface::Use.slug().to_owned(),
                r#"Action::"foundry.ontology.use""#.to_owned(),
            ),
        ]),
    })
}

pub fn load_guarded(version: &str) -> Result<PdpRuntimeGuard, PepError> {
    let pdp = CedarPdp::load(
        &bundle(version)?,
        Arc::new(SeededIdGenerator::default()),
        DECISION_CACHE_CAPACITY,
    )
    .map_err(|error| PepError::BundleRejected {
        detail: format!("{error:?}"),
    })?;
    Ok(PdpRuntimeGuard::new(
        Arc::new(pdp) as Arc<dyn PolicyDecisionPoint>,
        PdpRuntimeConfig::new(DECISION_DEADLINE, CIRCUIT_OPEN_AFTER_FAILURES),
    ))
}

const DECISION_CACHE_CAPACITY: usize = 256;
/// An in-process decision is microseconds of work; a deadline this wide
/// only ever fires on something pathological, which is exactly when a
/// fail-closed refusal is the right answer.
const DECISION_DEADLINE: std::time::Duration = std::time::Duration::from_millis(250);
const CIRCUIT_OPEN_AFTER_FAILURES: u32 = 5;
