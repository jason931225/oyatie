//! Ceiling on contiguous comment runs in Rust sources.
//!
//! Every fixture is built with `repeat` rather than written out as a literal
//! block, so this file cannot trip the very rule it pins.

use pipeline_admission::comment_run_violations;

const RUST: &str = "network/core/route/src/lib.rs";

fn refusals(path: &str, text: &str) -> Vec<String> {
    comment_run_violations(path, text.as_bytes())
}

fn assert_admitted(path: &str, text: &str) {
    let refusals = refusals(path, text);
    assert!(refusals.is_empty(), "{path} refused: {refusals:?}");
}

fn assert_refused(path: &str, text: &str) -> String {
    let refusals = refusals(path, text);
    assert_eq!(refusals.len(), 1, "{path}: {refusals:?}");
    refusals.into_iter().next().expect("single refusal")
}

#[test]
fn the_boundary_is_refused_above_and_admitted_at_the_ceiling() {
    let refusal = assert_refused(RUST, &"// note\n".repeat(21));
    assert!(
        refusal.contains(&format!("{RUST}:1:")) && refusal.contains("21 consecutive comment lines"),
        "{refusal}"
    );
    assert_admitted(RUST, &"// note\n".repeat(20));
}

#[test]
fn a_blank_line_bridges_a_run_instead_of_breaking_it() {
    let bridged = format!("{}\n{}", "// note\n".repeat(11), "// note\n".repeat(11));
    let refusal = assert_refused(RUST, &bridged);
    assert!(
        refusal.contains(&format!("{RUST}:1:")) && refusal.contains("22 consecutive comment lines"),
        "{refusal}"
    );
}

#[test]
fn bridging_blanks_do_not_themselves_count_toward_the_run() {
    let padded = format!("{}\n\n\n{}", "// note\n".repeat(10), "// note\n".repeat(10));
    assert_admitted(RUST, &padded);
}

#[test]
fn every_line_comment_spelling_counts() {
    for opener in ["//", "///", "//!"] {
        assert_refused(RUST, &format!("{opener} note\n").repeat(21));
    }
    let mixed = format!(
        "{}{}{}",
        "//! module\n".repeat(7),
        "/// item\n".repeat(7),
        "// aside\n".repeat(7)
    );
    assert_refused(RUST, &mixed);
}

#[test]
fn block_comment_bodies_count_including_their_delimiters() {
    let block = format!("/*\n{}*/\n", "* note\n".repeat(20));
    let refusal = assert_refused(RUST, &block);
    assert!(
        refusal.contains("22 consecutive comment lines"),
        "{refusal}"
    );
    assert_admitted(RUST, &format!("/*\n{}*/\n", "* note\n".repeat(18)));
}

#[test]
fn a_nested_block_stays_open_until_its_outermost_close() {
    let nested = format!(
        "/*\n/* inner */\n{}*/\nlet code = 1;\n",
        "* note\n".repeat(19)
    );
    assert_refused(RUST, &nested);
}

#[test]
fn code_ends_a_run_and_the_next_run_reports_its_own_start() {
    assert_admitted(RUST, &"// note\nlet value = 1;\n".repeat(40));
    let after_code = format!("let value = 1;\n\n{}", "// note\n".repeat(21));
    let refusal = assert_refused(RUST, &after_code);
    assert!(refusal.contains(&format!("{RUST}:3:")), "{refusal}");
}

#[test]
fn code_trailing_a_block_close_ends_the_run() {
    let split = format!(
        "/*\n{}*/ let value = 1;\n\n{}",
        "* note\n".repeat(14),
        "// note\n".repeat(15)
    );
    assert_admitted(RUST, &split);
}

#[test]
fn the_closed_path_derived_exempt_set_is_honored() {
    let blob = "// note\n".repeat(230);
    for path in [
        "third-party/vendor/source.rs",
        "network/core/route/src/model.generated.rs",
    ] {
        assert_admitted(path, &blob);
    }
}

#[test]
fn sources_that_are_not_rust_are_left_to_their_own_conventions() {
    let blob = "// note\n".repeat(230);
    for path in [
        "network/core/route/src/route.proto",
        "network/core/route/src/policy.cedar",
        "network/core/route/src/setup.sql",
        "docs/decisions/ADR-0719-example.md",
    ] {
        assert_admitted(path, &blob);
    }
}

#[test]
fn invalid_utf8_does_not_switch_the_rule_off() {
    let mut bytes = "// note\n".repeat(21).into_bytes();
    bytes.push(0xff);
    assert!(!comment_run_violations(RUST, &bytes).is_empty());
}
