//! Facade driver wiring: composes the kernel entry points with the adapters.
//!
//! Each module here is one composition: `smoke` proves a seam is inhabited, `pipeline` runs the
//! canary path, `port_go` runs the Go corpus, `probe` surveys source the engine has never seen, and
//! `report` carries what a run produced and how it can refuse.

mod pipeline;
mod port_go;
mod probe;
mod report;
mod smoke;

pub use pipeline::{
    smoke_canary_planted_defect, smoke_delta, smoke_emit_canary, smoke_materialize_canary,
    smoke_pipeline, smoke_receipt_golden, smoke_render, smoke_transform,
};
pub use port_go::{
    assemble_modules, port_go_delta, port_go_dispositions, port_go_drift_after,
    port_go_drift_before, port_go_from, port_go_pipeline, port_go_refused, port_go_refused_failure,
    port_go_refused_interface, port_go_refused_ownership, port_go_source,
};
pub use probe::{render_survey, survey_snapshot};
pub use report::{PipelineError, PipelineReport, PlanSmokeError};
pub use smoke::{
    fleet_pin, smoke_admit_snapshot, smoke_declarations, smoke_digest, smoke_engine_digest,
    smoke_plan, smoke_render_stub, smoke_rulepack, smoke_syn_quote_render, smoke_toolchain_digest,
};

pub use port_engine_kernel::{Verdict, emit, plan, verify};

#[must_use]
pub const fn w0_ready() -> bool {
    port_engine_api::w0_ready()
        && port_engine_source_pin::w0_ready()
        && port_engine_rust_ir::w0_ready()
        && port_engine_frontend_go::w0_ready()
        && port_engine_hash::w0_ready()
        && port_engine_rulepack::w0_ready()
        && port_engine_snapshot::w0_ready()
        && port_engine_identity::w0_ready()
        && port_engine_toolchain::w0_ready()
        && port_engine_transform::w0_ready()
        && port_engine_emit::w0_ready()
}

/// Adapter readiness snapshot for diagnostics.
///
/// Order: `(pin, rust_ir, frontend, hash, rulepack, snapshot, identity, toolchain, transform, emit)`.
#[must_use]
pub fn adapter_readiness() -> (bool, bool, bool, bool, bool, bool, bool, bool, bool, bool) {
    (
        port_engine_source_pin::w0_ready(),
        port_engine_rust_ir::w0_ready(),
        port_engine_frontend_go::w0_ready(),
        port_engine_hash::w0_ready(),
        port_engine_rulepack::w0_ready(),
        port_engine_snapshot::w0_ready(),
        port_engine_identity::w0_ready(),
        port_engine_toolchain::w0_ready(),
        port_engine_transform::w0_ready(),
        port_engine_emit::w0_ready(),
    )
}
