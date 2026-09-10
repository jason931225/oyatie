//! What a pipeline run produced, and how a run can refuse.

use std::collections::BTreeMap;

use port_engine_api::{
    Declaration, Digest, Receipt, RegionId, Renderer, RulePack, SourceModel, UnitId,
};
use port_engine_emit::EmitError;
use port_engine_hash::digest_str;
use port_engine_rulepack::{LoadedRulePack, RulepackError};
use port_engine_rust_ir::RustRenderer;
use port_engine_snapshot::AdmitError;
use port_engine_transform::{DispositionRecord, TransformError, TransformOutput};

use crate::engine::engine_digest;
use crate::receipt_codec::emit_tree_digest;

/// Outcome of the pin→admit→plan→transform→emit→receipt pipeline.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PipelineReport {
    /// Which unit each emitted region came from, as reported by the transform rather than parsed
    /// back out of the region id.
    pub region_units: BTreeMap<RegionId, port_engine_api::UnitId>,
    pub receipt: Receipt,
    pub plan_steps: usize,
    pub emit_regions: usize,
    pub emitted: BTreeMap<RegionId, Vec<u8>>,
    pub emit_digest: Digest,
    /// Every ownership decision the run made, with its justification.
    pub dispositions: Vec<DispositionRecord>,
}

/// Bind every axis for one run, refusing rather than reporting an axis that says nothing.
///
/// `label` names the caller in the refusal.
///
/// # Errors
/// [`PipelineError::Emit`] when any axis is left empty.
pub(crate) fn bind_receipt(
    label: &str,
    pin: String,
    snapshot_digest: Digest,
    pack: &LoadedRulePack,
    renderer: &RustRenderer,
) -> Result<Receipt, PipelineError> {
    let receipt = Receipt {
        pin,
        snapshot_digest,
        engine_digest: engine_digest(),
        rulepack_digest: pack.digest(),
        toolchain_digest: port_engine_toolchain::toolchain_digest(),
        formatter_digest: digest_str(&renderer.formatter_digest().0),
    };
    if !receipt.incomplete_axes().is_empty() {
        return Err(PipelineError::Emit(port_engine_api::PortError::Render {
            detail: format!(
                "{label} receipt incomplete axes: {:?}",
                receipt.incomplete_axes()
            ),
        }));
    }
    Ok(receipt)
}

pub(crate) fn bind_report(
    plan_steps: usize,
    transformed: TransformOutput,
    emitted: BTreeMap<RegionId, Vec<u8>>,
    receipt: Receipt,
) -> PipelineReport {
    PipelineReport {
        plan_steps,
        emit_regions: emitted.len(),
        emit_digest: emit_tree_digest(&emitted),
        region_units: transformed.region_units,
        dispositions: transformed.dispositions,
        emitted,
        receipt,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PipelineError {
    Admit(AdmitError),
    Rulepack(RulepackError),
    Plan(port_engine_api::PortError),
    Transform(TransformError),
    Emit(port_engine_api::PortError),
    Canary(EmitError),
}

impl std::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Admit(err) => write!(f, "pipeline admit: {err}"),
            Self::Rulepack(err) => write!(f, "pipeline rulepack: {err}"),
            Self::Plan(err) => write!(f, "pipeline plan: {err}"),
            Self::Transform(err) => write!(f, "pipeline transform: {err}"),
            Self::Emit(err) => write!(f, "pipeline emit: {err}"),
            Self::Canary(err) => write!(f, "pipeline canary: {err}"),
        }
    }
}

impl std::error::Error for PipelineError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlanSmokeError {
    Rulepack(RulepackError),
    Port(port_engine_api::PortError),
}

impl std::fmt::Display for PlanSmokeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rulepack(err) => write!(f, "plan smoke rulepack: {err}"),
            Self::Port(err) => write!(f, "plan smoke kernel: {err}"),
        }
    }
}

impl std::error::Error for PlanSmokeError {}

pub(crate) struct RulepackModel;

impl SourceModel for RulepackModel {
    fn language(&self) -> &str {
        "go"
    }

    fn snapshot_digest(&self) -> Digest {
        digest_str("slice7-rulepack-model")
    }

    fn units(&self) -> Vec<UnitId> {
        vec![
            UnitId("example.com/a".into()),
            UnitId("example.com/b".into()),
        ]
    }

    /// This model carries no source and therefore declares nothing. `Some(vec![])` says exactly
    /// that, and is a different answer from the `None` an unknown unit gets.
    fn declarations(&self, unit: &UnitId) -> Option<Vec<Declaration>> {
        self.units().contains(unit).then(Vec::new)
    }
}
