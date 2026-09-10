//! Per-adapter smoke entry points: the thinnest call that proves a seam is inhabited.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::engine::engine_digest;
use port_engine_api::{
    Declaration, Digest, Receipt, RegionId, RulePack, SourceModel, TargetIr, UnitId,
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

use crate::driver::report::PlanSmokeError;
use crate::driver::report::RulepackModel;

/// # Errors
/// Propagates [`port_engine_source_pin::PinError`] from the pin loader.
pub fn fleet_pin() -> Result<String, port_engine_source_pin::PinError> {
    let pin = load_embedded()?;
    Ok(receipt_pin(&pin))
}

/// # Errors
/// Propagates [`port_engine_api::PortError`] from [`port_engine_kernel::emit`].
pub fn smoke_render_stub() -> Result<(), port_engine_api::PortError> {
    let ir = RustIr::new(&["stub"]);
    let renderer = EmptyRenderer::new("slice3-fmt-stub");
    let _ = port_engine_kernel::emit(&renderer, &ir)?;
    Ok(())
}

/// Smoke the syn/quote typed emit path (not the fail-closed dyn Renderer).
///
/// # Errors
/// Propagates [`port_engine_api::PortError`] from [`RustRenderer::render_rust_ir`].
pub fn smoke_syn_quote_render() -> Result<(), port_engine_api::PortError> {
    let mut ir = RustIr::new(&["stub"]);
    ir.set_items(
        "stub",
        vec![RustItem::Function(RustFn {
            docs: Vec::new(),
            vis: Visibility::Public,
            name: "stub".into(),
            receiver: None,
            params: Vec::new(),
            ret: None,
            body: Some(Vec::new()),
        })],
    )?;
    let renderer = RustRenderer::new();
    let out = renderer.render_rust_ir(&ir)?;
    if out.len() != 1 {
        return Err(port_engine_api::PortError::Render {
            detail: format!("expected 1 region, got {}", out.len()),
        });
    }
    Ok(())
}

#[must_use]
pub fn smoke_digest(text: &str) -> Digest {
    digest_str(text)
}

/// Load the embedded fixture-gated rulepack; return digest + selecting-fixture count.
///
/// # Errors
/// Propagates [`RulepackError`] from the rulepack loader.
pub fn smoke_rulepack() -> Result<(Digest, usize), RulepackError> {
    let pack = LoadedRulePack::load_embedded()?;
    Ok((pack.digest(), pack.selecting_fixture_count()))
}

/// Plan the embedded v0 rulepack against its declared example units.
///
/// # Errors
/// [`RulepackError`] on load failure, or [`port_engine_api::PortError`] from kernel plan.
pub fn smoke_plan() -> Result<usize, PlanSmokeError> {
    let pack = LoadedRulePack::load_embedded().map_err(PlanSmokeError::Rulepack)?;
    let model = RulepackModel;
    let plan = port_engine_kernel::plan(&model, &pack).map_err(PlanSmokeError::Port)?;
    Ok(plan.steps.len())
}

/// # Errors
/// Propagates [`AdmitError`] from snapshot admission.
pub fn smoke_admit_snapshot() -> Result<AdmittedSnapshot, AdmitError> {
    admit_embedded_fixture()
}

/// Admit the v1 Go-corpus snapshot and summarise what each unit declares.
///
/// # Errors
/// Propagates [`AdmitError`] — including the digest mismatch that a drift between the Go
/// extractor's preimage encoder and the Rust one would produce.
pub fn smoke_declarations() -> Result<(AdmittedSnapshot, Vec<(String, usize)>), AdmitError> {
    let admitted = admit_embedded_fixture_v1()?;
    let mut summary = Vec::new();
    for unit in admitted.as_model().units() {
        let count = admitted
            .as_model()
            .declarations(&unit)
            .map_or(0, |declarations| declarations.len());
        summary.push((unit.0, count));
    }
    Ok((admitted, summary))
}

#[must_use]
pub fn smoke_engine_digest() -> Digest {
    engine_digest()
}

#[must_use]
pub fn smoke_toolchain_digest() -> Digest {
    toolchain_digest()
}
