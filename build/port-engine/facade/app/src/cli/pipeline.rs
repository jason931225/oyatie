//! Commands that run a whole composition: the canary pipeline, the Go port, and the receipt.

use std::path::Path;
use std::process::ExitCode;

use crate::cli::USAGE;
use crate::driver;
use crate::receipt_codec;
use crate::receipt_e2e;

pub(crate) fn cmd_transform() -> ExitCode {
    match driver::smoke_transform() {
        Ok(regions) => {
            println!("transform=ok regions={regions}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("port-engine-app: transform failed: {err}");
            ExitCode::from(1)
        }
    }
}

pub(crate) fn cmd_render() -> ExitCode {
    match driver::smoke_render() {
        Ok((regions, digest)) => {
            println!("render=ok regions={regions} emit_digest={}", digest.0);
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("port-engine-app: render failed: {err}");
            ExitCode::from(1)
        }
    }
}

pub(crate) fn cmd_pipeline() -> ExitCode {
    match driver::smoke_pipeline() {
        Ok(report) => {
            println!(
                "pipeline=ok plan_steps={} emit_regions={} emit_digest={}",
                report.plan_steps, report.emit_regions, report.emit_digest.0
            );
            print!("{}", receipt_codec::format_receipt(&report.receipt));
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("port-engine-app: pipeline failed: {err}");
            ExitCode::from(1)
        }
    }
}

pub(crate) fn cmd_receipt() -> ExitCode {
    match driver::smoke_receipt_golden() {
        Ok(text) => {
            println!("receipt=ok golden=true");
            print!("{text}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("port-engine-app: receipt failed: {err}");
            ExitCode::from(1)
        }
    }
}

pub(crate) fn cmd_delta() -> ExitCode {
    match driver::smoke_delta() {
        Ok(verification) => {
            println!(
                "delta=ok verdict={:?} delta={:?}",
                verification.verdict, verification.delta
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("port-engine-app: delta/verify failed: {err}");
            ExitCode::from(1)
        }
    }
}

pub(crate) fn cmd_verify_e2e() -> ExitCode {
    match receipt_e2e::run_six_axis_e2e() {
        Ok(report) => {
            println!("verify-e2e=ok pin={}", report.pin);
            for s in &report.scenarios {
                println!(
                    "scenario={} verdict={:?} delta={:?}",
                    s.name, s.verification.verdict, s.verification.delta
                );
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("port-engine-app: verify-e2e failed: {err}");
            ExitCode::from(1)
        }
    }
}

pub(crate) fn cmd_emit_canary() -> ExitCode {
    match driver::smoke_emit_canary() {
        Ok(art) => {
            println!(
                "emit-canary=ok region={} digest={} bytes={}",
                art.region.0,
                art.digest.0,
                art.bytes.len()
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("port-engine-app: emit-canary failed: {err}");
            ExitCode::from(1)
        }
    }
}

pub(crate) fn cmd_materialize_canary(out_dir: Option<&str>) -> ExitCode {
    let Some(out_dir) = out_dir else {
        eprintln!("port-engine-app: materialize-canary requires <dir>");
        eprint!("{USAGE}");
        return ExitCode::from(2);
    };
    match driver::smoke_materialize_canary(Path::new(out_dir)) {
        Ok((art, dest)) => {
            println!(
                "materialize-canary=ok region={} digest={} path={}",
                art.region.0,
                art.digest.0,
                dest.display()
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("port-engine-app: materialize-canary failed: {err}");
            ExitCode::from(1)
        }
    }
}

pub(crate) fn cmd_canary_defect() -> ExitCode {
    match driver::smoke_canary_planted_defect() {
        Ok(verification) => {
            println!(
                "canary-defect=ok verdict={:?} delta={:?}",
                verification.verdict, verification.delta
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("port-engine-app: canary-defect failed: {err}");
            ExitCode::from(1)
        }
    }
}

pub(crate) fn cmd_port_go() -> ExitCode {
    match driver::port_go_pipeline() {
        Ok(report) => {
            println!(
                "port-go=ok steps={} regions={} emit_digest={}",
                report.plan_steps, report.emit_regions, report.emit_digest.0
            );
            for (region, bytes) in &report.emitted {
                let text = String::from_utf8_lossy(bytes);
                println!("--- {}", region.0);
                println!("{text}");
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("port-engine-app: port-go failed: {err}");
            ExitCode::from(1)
        }
    }
}

pub(crate) fn cmd_dispositions() -> ExitCode {
    match driver::port_go_dispositions() {
        Ok(report) => {
            print!("{report}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("port-engine-app: dispositions failed: {err}");
            ExitCode::from(1)
        }
    }
}

/// Print the per-region digest manifest for the ported corpus.
pub(crate) fn cmd_region_digests() -> ExitCode {
    match driver::port_go_pipeline() {
        Ok(report) => {
            for (region, digest) in crate::receipt_codec::region_digests(&report.emitted) {
                println!("{digest}  {region}");
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("port-engine-app: region-digests failed: {err}");
            ExitCode::from(1)
        }
    }
}

/// Survey a snapshot of source the engine has never seen, and report what it could not do.
///
/// Exits SUCCESS whatever the coverage. A survey that failed on incomplete coverage would be a
/// gate, and this is an instrument — the number it reports is the thing being measured, not a
/// threshold being enforced.
pub(crate) fn cmd_survey(path: Option<&str>) -> ExitCode {
    let Some(path) = path else {
        eprintln!("port-engine-app: survey needs a snapshot path");
        return ExitCode::from(2);
    };
    match driver::survey_snapshot(std::path::Path::new(path)) {
        Ok(report) => {
            print!("{}", driver::render_survey(&report));
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("port-engine-app: survey failed: {err}");
            ExitCode::from(1)
        }
    }
}

pub(crate) fn cmd_port_go_source() -> ExitCode {
    match driver::port_go_source() {
        Ok((source, matches_golden)) => {
            print!("{source}");
            if matches_golden {
                ExitCode::SUCCESS
            } else {
                eprintln!(
                    "port-engine-app: port-go source differs from src/port-go-golden-v1.txt; \
                     if the change is intended, refresh it with \
                     `port-engine-app port-go-source > .../src/port-go-golden-v1.txt`"
                );
                ExitCode::from(1)
            }
        }
        Err(err) => {
            eprintln!("port-engine-app: port-go-source failed: {err}");
            ExitCode::from(1)
        }
    }
}
