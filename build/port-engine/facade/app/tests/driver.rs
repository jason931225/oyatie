//! Driver composition smoke: the canary path end to end, and every adapter reporting itself wired.

use port_engine_app::driver::*;
use port_engine_app::receipt_codec::matches_golden;

#[test]
fn driver_wiring_is_ready() {
    use std::time::{SystemTime, UNIX_EPOCH};

    assert!(w0_ready());
    fleet_pin().expect("fleet pin must load");
    smoke_render_stub().expect("empty renderer stub must emit");
    smoke_syn_quote_render().expect("syn/quote path must emit");
    let d = smoke_digest("port-engine");
    assert!(d.0.starts_with("sha256:"));
    let (pack_digest, fixtures) = smoke_rulepack().expect("rulepack must load");
    assert!(pack_digest.0.starts_with("sha256:"));
    assert!(fixtures >= 2);
    let steps = smoke_plan().expect("plan smoke must succeed");
    assert_eq!(steps, 3);
    let admitted = smoke_admit_snapshot().expect("snapshot fixture must admit");
    assert!(admitted.artifact_digest().0.starts_with("sha256:"));
    let eng = smoke_engine_digest();
    assert!(eng.0.starts_with("sha256:"));
    let tc = smoke_toolchain_digest();
    assert_eq!(
        tc.0,
        "sha256:21fa7a4116e8a0cf9b3714b3fa5d173ad535eba6610d9b30a1b7f74dbfb74c0d"
    );
    let regions = smoke_transform().expect("transform must succeed");
    assert_eq!(regions, 3);
    let report = smoke_pipeline().expect("pipeline must succeed");
    assert_eq!(report.plan_steps, 3);
    assert_eq!(report.emit_regions, 3);
    assert!(report.receipt.incomplete_axes().is_empty());
    assert_eq!(report.receipt.engine_digest, eng);
    assert_eq!(report.receipt.toolchain_digest, tc);
    assert!(matches_golden(&report.receipt));
    let golden = smoke_receipt_golden().expect("golden receipt");
    assert!(golden.contains("snapshot_digest=sha256:"));
    let (render_regions, render_digest) = smoke_render().expect("render");
    assert_eq!(render_regions, 3);
    assert_eq!(render_digest, report.emit_digest);
    let verification = smoke_delta().expect("delta re-run");
    assert_eq!(verification.verdict, port_engine_kernel::Verdict::Green);
    let canary = smoke_emit_canary().expect("canary emit");
    assert!(canary.region.0.ends_with("__canary_empty_unit"));
    let planted = smoke_canary_planted_defect().expect("planted defect");
    assert_eq!(planted.verdict, port_engine_kernel::Verdict::Red);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let out = std::env::temp_dir()
        .join(format!("pe-facade-canary-{nanos}"))
        .join(port_engine_emit::CANARY_OUT_DIRNAME);
    let (art, dest) = smoke_materialize_canary(&out).expect("materialize");
    assert_eq!(art.digest, canary.digest);
    assert_eq!(
        dest.file_name().and_then(|s| s.to_str()),
        Some(port_engine_emit::CANARY_FILENAME)
    );
    let _ = std::fs::remove_dir_all(out.parent().expect("parent"));
    let (pin, rust_ir, frontend, hash, rulepack, snapshot, identity, toolchain, transform, emit) =
        adapter_readiness();
    assert!(
        pin && rust_ir
            && frontend
            && hash
            && rulepack
            && snapshot
            && identity
            && toolchain
            && transform
            && emit
    );
}
