//! Thin CLI wrapper around [`data_ontology_scorecards_resolver`] kernel.
//!
//! Subcommands:
//!  - default       — validate every (ms, framework) combination resolves green;
//!  - `--emit-rollup` — rewrite `registry/hyperscaler-scorecards/index.json`;
//!  - `--check`     — verify rollup on disk is byte-identical to recomputed.
//!  - `<ms> <fw>`   — emit a single resolved scorecard JSON to stdout.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use data_ontology_scorecards_resolver::{
    FRAMEWORKS, MICROSERVICES, build_rollup, resolve_scorecard,
};
use serde_json::Value;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut emit_rollup = false;
    let mut check = false;
    let mut root = PathBuf::from(".");
    let mut positional: Vec<String> = Vec::new();
    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        match flag.as_str() {
            "--emit-rollup" => emit_rollup = true,
            "--check" => check = true,
            "--root" => {
                if let Some(p) = iter.next() {
                    root = PathBuf::from(p);
                } else {
                    eprintln!("--root requires an argument");
                    return ExitCode::from(2);
                }
            }
            arg if !arg.starts_with("--") => positional.push(arg.to_string()),
            other => {
                eprintln!("unknown flag: {other}");
                return ExitCode::from(2);
            }
        }
    }

    if check {
        return run_check(&root);
    }
    if emit_rollup {
        return run_emit_rollup(&root);
    }
    if positional.len() == 2 {
        return run_resolve_one(&root, &positional[0], &positional[1]);
    }
    run_default_validation(&root)
}

fn canonical_dir(root: &Path) -> PathBuf {
    root.join("specs/microservices/scorecards/canonical")
}

fn overrides_path(root: &Path, ms: &str) -> PathBuf {
    root.join(format!("microservices/{ms}/scorecards/overrides.json"))
}

fn rollup_path(root: &Path) -> PathBuf {
    root.join("registry/hyperscaler-scorecards/index.json")
}

fn load_json(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))
}

fn run_default_validation(root: &Path) -> ExitCode {
    let mut failed = 0usize;
    let total = MICROSERVICES.len() * FRAMEWORKS.len();
    for ms in MICROSERVICES {
        let Ok(overrides) = load_json(&overrides_path(root, ms)) else {
            eprintln!("missing overrides for {ms}");
            failed += 1;
            continue;
        };
        for (slug, _, _) in FRAMEWORKS {
            let canonical_path = canonical_dir(root).join(format!("{slug}.json"));
            let Ok(canonical) = load_json(&canonical_path) else {
                eprintln!("missing canonical for {slug}");
                failed += 1;
                continue;
            };
            match resolve_scorecard(slug, &canonical, &overrides, ms) {
                Ok(resolved) => {
                    if resolved.get("overall_status").and_then(|v| v.as_str()) != Some("green") {
                        eprintln!("non-green {ms} {slug}");
                        failed += 1;
                    }
                }
                Err(e) => {
                    eprintln!("resolve failed {ms} {slug}: {e}");
                    failed += 1;
                }
            }
        }
    }
    println!("resolved {} / {} scorecards", total - failed, total);
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn run_emit_rollup(root: &Path) -> ExitCode {
    let entries = match collect_overrides(root) {
        Ok(e) => e,
        Err(msg) => {
            eprintln!("{msg}");
            return ExitCode::FAILURE;
        }
    };
    let rollup = build_rollup("2026-05-18", &entries);
    let path = rollup_path(root);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut text = serde_json::to_string_pretty(&rollup).expect("serialize");
    text.push('\n');
    if let Err(e) = fs::write(&path, text) {
        eprintln!("write {}: {e}", path.display());
        return ExitCode::FAILURE;
    }
    println!("wrote {}", path.display());
    ExitCode::SUCCESS
}

fn run_check(root: &Path) -> ExitCode {
    let entries = match collect_overrides(root) {
        Ok(e) => e,
        Err(msg) => {
            eprintln!("{msg}");
            return ExitCode::FAILURE;
        }
    };
    let rollup = build_rollup("2026-05-18", &entries);
    let path = rollup_path(root);
    let on_disk = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("read {}: {e}", path.display());
            return ExitCode::FAILURE;
        }
    };
    let on_disk_value: Value = match serde_json::from_str(&on_disk) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("parse {}: {e}", path.display());
            return ExitCode::FAILURE;
        }
    };
    if on_disk_value == rollup {
        println!("scorecards --check: rollup matches recomputed view");
        ExitCode::SUCCESS
    } else {
        eprintln!("scorecards --check: rollup drift detected");
        ExitCode::FAILURE
    }
}

fn run_resolve_one(root: &Path, ms: &str, framework: &str) -> ExitCode {
    let canonical = match load_json(&canonical_dir(root).join(format!("{framework}.json"))) {
        Ok(v) => v,
        Err(msg) => {
            eprintln!("{msg}");
            return ExitCode::FAILURE;
        }
    };
    let overrides = match load_json(&overrides_path(root, ms)) {
        Ok(v) => v,
        Err(msg) => {
            eprintln!("{msg}");
            return ExitCode::FAILURE;
        }
    };
    match resolve_scorecard(framework, &canonical, &overrides, ms) {
        Ok(resolved) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&resolved).expect("serialize")
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn collect_overrides(root: &Path) -> Result<Vec<(String, Value)>, String> {
    let mut entries = Vec::new();
    for ms in MICROSERVICES {
        let path = overrides_path(root, ms);
        let v = load_json(&path)?;
        entries.push(((*ms).to_string(), v));
    }
    Ok(entries)
}
