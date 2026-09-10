//! Hand-rolled CLI for `port-engine-app`.
//!
//! No clap or argv crate: keeping the facade free of new lock-forcing dependencies is worth more
//! than the ergonomics, at this size.

mod pipeline;
mod seams;

use std::process::ExitCode;

use pipeline::{
    cmd_canary_defect, cmd_delta, cmd_dispositions, cmd_emit_canary, cmd_materialize_canary,
    cmd_pipeline, cmd_port_go, cmd_port_go_source, cmd_receipt, cmd_region_digests, cmd_render,
    cmd_survey, cmd_transform, cmd_verify_e2e,
};
use seams::{
    cmd_admit_snapshot, cmd_declarations, cmd_digest, cmd_emit_stub, cmd_emit_syn, cmd_engine,
    cmd_pin, cmd_plan, cmd_ready, cmd_rulepack, cmd_toolchain,
};

/// Every command [`run`] dispatches, in usage order.
///
/// The usage text below is prose around this list rather than a second copy of it;
/// `usage_lists_exactly_the_dispatchable_commands` holds the two together.
pub(crate) const COMMANDS: &[&str] = &[
    "help",
    "ready",
    "pin",
    "emit-stub",
    "emit-syn",
    "emit-canary",
    "materialize-canary",
    "canary-defect",
    "digest",
    "rulepack",
    "plan",
    "admit-snapshot",
    "declarations",
    "port-go",
    "port-go-source",
    "survey",
    "region-digests",
    "dispositions",
    "transform",
    "render",
    "engine",
    "toolchain",
    "pipeline",
    "receipt",
    "verify",
    "delta",
    "verify-e2e",
];

pub(crate) const USAGE: &str = "\
port-engine-app — owned deterministic port-engine driver

Usage:
  port-engine-app <command> [args]

Commands:
  help              Show this usage
  ready             Print adapter readiness (exit 0 when wired)
  pin               Print fleet upstream pin (peeled commit)
  emit-stub         Smoke empty-renderer emit via kernel
  emit-syn          Smoke syn/quote typed emit
  emit-canary       Select single canary region; fail closed vs golden
  materialize-canary <dir>
                    Write single canary.rs under allowlisted canary-out dir
  canary-defect     Plant canary byte defect; expect Red/Unexplained
  digest <text>     SHA-256 digest of UTF-8 text
  rulepack          Load fixture-gated rulepack v0; print digest + fixture count
  plan              Plan embedded rulepack against example units
  admit-snapshot    Admit hermetic OOB bootstrap snapshot fixture
  declarations      Admit the v1 Go-corpus snapshot; list what each unit declares
  port-go           Port the hermetic Go corpus; print the emitted Rust per region
  port-go-source    Print the assembled per-unit modules (fail closed vs golden)
  survey <path>     Measure this engine against an extracted snapshot it has never seen
  region-digests    Per-region digests, so a change's blast radius is countable
  dispositions      Print every ownership decision and its justification
  transform         Admit→plan→apply constructions → RustIr region count
  render            Transform+emit; print region count + emit tree digest
  engine            Print engine identity digest
  toolchain         Print dual-home toolchain corpus digest
  pipeline          pin→admit→plan→transform→emit→six-axis receipt
  receipt           Print canonical receipt; fail closed vs golden
  verify            Deterministic re-run classify (alias of delta)
  delta             Re-run pipeline twice; expect Unchanged/Green
  verify-e2e        Run six-axis receipt end-to-end scenarios

Exit codes: 0 ok · 1 error · 2 usage
";

/// Dispatch argv (skipping argv[0]). Returns a process exit code.
#[must_use]
pub fn run(args: &[String]) -> ExitCode {
    let cmd = args.first().map(String::as_str).unwrap_or("help");
    match cmd {
        "help" | "-h" | "--help" => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        "ready" => cmd_ready(),
        "pin" => cmd_pin(),
        "emit-stub" => cmd_emit_stub(),
        "emit-syn" => cmd_emit_syn(),
        "emit-canary" => cmd_emit_canary(),
        "materialize-canary" => cmd_materialize_canary(args.get(1).map(String::as_str)),
        "canary-defect" => cmd_canary_defect(),
        "digest" => cmd_digest(args.get(1).map(String::as_str)),
        "rulepack" => cmd_rulepack(),
        "plan" => cmd_plan(),
        "admit-snapshot" => cmd_admit_snapshot(),
        "declarations" => cmd_declarations(),
        "port-go" => cmd_port_go(),
        "port-go-source" => cmd_port_go_source(),
        "survey" => cmd_survey(args.get(1).map(String::as_str)),
        "region-digests" => cmd_region_digests(),
        "dispositions" => cmd_dispositions(),
        "transform" => cmd_transform(),
        "render" => cmd_render(),
        "engine" => cmd_engine(),
        "toolchain" => cmd_toolchain(),
        "pipeline" => cmd_pipeline(),
        "receipt" => cmd_receipt(),
        "verify" | "delta" => cmd_delta(),
        "verify-e2e" => cmd_verify_e2e(),
        other => {
            eprintln!("port-engine-app: unknown command `{other}`");
            eprint!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{COMMANDS, USAGE};

    /// The command names USAGE documents, read back out of the text it prints.
    fn documented() -> Vec<&'static str> {
        USAGE
            .lines()
            .skip_while(|line| *line != "Commands:")
            .skip(1)
            .take_while(|line| !line.starts_with("Exit codes:"))
            .filter(|line| line.starts_with("  ") && !line.starts_with("   "))
            .filter_map(|line| line.split_whitespace().next())
            .collect()
    }

    #[test]
    fn usage_lists_exactly_the_dispatchable_commands() {
        assert_eq!(
            documented(),
            COMMANDS.to_vec(),
            "the usage text and the command list must not drift apart"
        );
    }
}
