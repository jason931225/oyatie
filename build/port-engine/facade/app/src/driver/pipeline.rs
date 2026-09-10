//! The canary pipeline: pin → admit → plan → transform → emit → receipt.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::engine::engine_digest;
use port_engine_api::{
    Declaration, Digest, Receipt, RegionId, Renderer, RulePack, SourceModel, TargetIr, UnitId,
};
use port_engine_emit::{
    CanaryArtifact, EmitError, emit_canary_checked, materialize_canary_roundtrip, select_canary,
};
use port_engine_hash::digest_str;
use port_engine_rulepack::{LoadedRulePack, RulepackError};
use port_engine_rust_ir::{EmptyRenderer, RustFn, RustIr, RustItem, RustRenderer, Visibility};
use port_engine_snapshot::{
    AdmitError, AdmittedSnapshot, admit_embedded_fixture, admit_embedded_fixture_refused_v1,
    admit_embedded_fixture_v1,
};
use port_engine_source_pin::{load_embedded, receipt_pin};
use port_engine_toolchain::toolchain_digest;
use port_engine_transform::{TransformError, apply, apply_with_provenance, sanitize_ident};

use crate::receipt_codec::{emit_tree_digest, format_receipt, matches_golden};

use crate::driver::report::{PipelineError, PipelineReport, bind_receipt, bind_report};
use crate::driver::smoke::smoke_admit_snapshot;

/// Admit → plan → transform → RustIr, without emit or receipt.
///
/// # Errors
/// [`PipelineError`] on admit/rulepack/plan/transform refusal.
pub fn smoke_transform() -> Result<usize, PipelineError> {
    let admitted = smoke_admit_snapshot().map_err(PipelineError::Admit)?;
    let pack = LoadedRulePack::load_embedded().map_err(PipelineError::Rulepack)?;
    let plan = port_engine_kernel::plan(admitted.as_model(), &pack).map_err(PipelineError::Plan)?;
    let ir = apply(&plan, &pack, admitted.as_model()).map_err(PipelineError::Transform)?;
    Ok(ir.regions().len())
}

/// Run pin → admit → plan → transform → syn emit → receipt.
///
/// # Errors
/// [`PipelineError`] on any stage refusal.
pub fn smoke_pipeline() -> Result<PipelineReport, PipelineError> {
    let admitted = smoke_admit_snapshot().map_err(PipelineError::Admit)?;
    let pin = admitted.pin().to_owned();
    let pack = LoadedRulePack::load_embedded().map_err(PipelineError::Rulepack)?;
    let plan = port_engine_kernel::plan(admitted.as_model(), &pack).map_err(PipelineError::Plan)?;
    let transformed = apply_with_provenance(&plan, &pack, admitted.as_model())
        .map_err(PipelineError::Transform)?;

    let renderer = RustRenderer::new();
    let emitted = renderer
        .render_rust_ir(&transformed.ir)
        .map_err(PipelineError::Emit)?;

    let receipt = bind_receipt(
        "pipeline",
        pin,
        admitted.artifact_digest().clone(),
        &pack,
        &renderer,
    )?;

    Ok(bind_report(plan.steps.len(), transformed, emitted, receipt))
}

/// # Errors
/// [`PipelineError`] on admit/rulepack/plan/transform/emit refusal.
pub fn smoke_render() -> Result<(usize, Digest), PipelineError> {
    let report = smoke_pipeline()?;
    Ok((report.emit_regions, report.emit_digest))
}

/// Re-run the pipeline twice and classify with kernel [`verify`](port_engine_kernel::verify).
///
/// Identical axes + identical emit bytes → `Unchanged` / Green.
///
/// # Errors
/// [`PipelineError`] on pipeline failure, or unexpected verdict.
pub fn smoke_delta() -> Result<port_engine_kernel::Verification, PipelineError> {
    let previous = smoke_pipeline()?;
    let current = smoke_pipeline()?;
    let verification = port_engine_kernel::verify(
        &previous.receipt,
        &previous.emitted,
        &current.receipt,
        &current.emitted,
    );
    if verification.verdict != port_engine_kernel::Verdict::Green
        || !matches!(verification.delta, port_engine_kernel::Delta::Unchanged)
    {
        return Err(PipelineError::Emit(port_engine_api::PortError::Render {
            detail: format!(
                "deterministic re-run expected Unchanged/Green, got {:?}",
                verification
            ),
        }));
    }
    if previous.receipt != current.receipt || previous.emit_digest != current.emit_digest {
        return Err(PipelineError::Emit(port_engine_api::PortError::Render {
            detail: "deterministic re-run receipt/emit digests diverged".into(),
        }));
    }
    Ok(verification)
}

/// Pipeline receipt must match the embedded golden (fail closed).
///
/// # Errors
/// [`PipelineError`] on pipeline failure or golden mismatch.
pub fn smoke_receipt_golden() -> Result<String, PipelineError> {
    let report = smoke_pipeline()?;
    let text = format_receipt(&report.receipt);
    if !matches_golden(&report.receipt) {
        return Err(PipelineError::Emit(port_engine_api::PortError::Render {
            detail: format!("receipt golden mismatch:\n{text}"),
        }));
    }
    Ok(text)
}

/// Select the single canary region from a pipeline emit tree and check its golden.
///
/// # Errors
/// [`PipelineError`] on pipeline or canary emit refusal.
pub fn smoke_emit_canary() -> Result<CanaryArtifact, PipelineError> {
    let report = smoke_pipeline()?;
    emit_canary_checked(&report.emitted).map_err(PipelineError::Canary)
}

/// Pipeline → canary golden → materialize single file → read-back round-trip.
///
/// `out_dir` basename must be [`port_engine_emit::CANARY_OUT_DIRNAME`]; never `k8s/`.
///
/// # Errors
/// [`PipelineError`] on pipeline, canary, path, or round-trip refusal.
pub fn smoke_materialize_canary(
    out_dir: &Path,
) -> Result<(CanaryArtifact, PathBuf), PipelineError> {
    let artifact = smoke_emit_canary()?;
    let dest = materialize_canary_roundtrip(out_dir, &artifact).map_err(PipelineError::Canary)?;
    Ok((artifact, dest))
}

/// Plant a byte defect in the canary region; expect kernel Red / Unexplained.
///
/// Proves the single-fixture canary is on the verify surface without bulk corpus emit.
///
/// # Errors
/// [`PipelineError`] when selection fails or the planted defect is not detected as Unexplained.
pub fn smoke_canary_planted_defect() -> Result<port_engine_kernel::Verification, PipelineError> {
    let previous = smoke_pipeline()?;
    let canary = select_canary(&previous.emitted).map_err(PipelineError::Canary)?;
    let mut current_emitted = previous.emitted.clone();
    current_emitted.insert(
        canary.region.clone(),
        b"pub fn planted_canary_defect () { }".to_vec(),
    );
    let verification = port_engine_kernel::verify(
        &previous.receipt,
        &previous.emitted,
        &previous.receipt,
        &current_emitted,
    );
    let expected_red = matches!(
        (&verification.verdict, &verification.delta),
        (
            port_engine_kernel::Verdict::Red,
            port_engine_kernel::Delta::Unexplained { regions }
        ) if regions.contains(&canary.region)
    );
    if !expected_red {
        return Err(PipelineError::Emit(port_engine_api::PortError::Render {
            detail: format!(
                "planted canary defect expected Red/Unexplained containing `{}`, got {:?}",
                canary.region.0, verification
            ),
        }));
    }
    Ok(verification)
}
