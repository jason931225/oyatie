//! The failure convention, end to end over real source.
//!
//! Not one construct but a CONVENTION, which is why it blocked every real package: the source
//! returns failure as an extra result that nothing requires a caller to check, and the target says
//! it in the return type, where the compiler does. These proofs are over the emitted crate rather
//! than over hand-built nodes, because a convention only exists across whole declarations.

use port_engine_app::driver;

#[test]
fn a_fallible_signature_becomes_a_result() {
    let report = driver::port_go_pipeline().expect("the Go corpus must port");
    let source = driver::assemble_modules(&report);

    for expected in [
        // A value and a failure.
        "pub fn length(s: String) -> Result<i64, Box<dyn std::error::Error>>",
        // A failure alone still has to say it succeeded.
        "pub fn check(s: String) -> Result<(), Box<dyn std::error::Error>>",
        // The trailing operand decides the constructor, and a failing return drops its zero-value
        // companion because the target's failing return carries only the failure.
        "return Err(Box::<dyn std::error::Error>::from(\"empty\"));",
        "Ok(s.len() as i64)",
    ] {
        assert!(
            source.contains(expected),
            "emitted source must carry `{expected}`:\n{source}"
        );
    }
}

#[test]
fn the_propagation_idiom_becomes_the_try_operator() {
    let report = driver::port_go_pipeline().expect("the Go corpus must port");
    let source = driver::assemble_modules(&report);

    assert!(
        source.contains("let n = length(s)?;"),
        "a bind-and-check pair must collapse into one operator:\n{source}"
    );
    // The failure-only shape binds nothing, so the call stays a statement.
    assert!(
        source.contains("check(s)?;"),
        "a check with no value bound must stay a statement:\n{source}"
    );
    // Over CODE lines only. The corpus's own prose describes the idiom it exercises, and a check
    // that read documentation as output would be asserting about the comment rather than the port.
    let code: String = source
        .lines()
        .filter(|line| !line.trim_start().starts_with("///"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !code.contains("!= nil"),
        "no comparison against the source's absent value may survive:\n{code}"
    );
}

#[test]
fn a_mapped_call_is_answered_by_the_pack() {
    let report = driver::port_go_pipeline().expect("the Go corpus must port");
    let source = driver::assemble_modules(&report);

    assert!(
        source.contains("s.len() as i64"),
        "a builtin must be answered by the pack rather than emitted by name:\n{source}"
    );
    assert!(
        source.contains("Box::<dyn std::error::Error>::from(\"empty\")"),
        "a standard-library call must be answered by the pack:\n{source}"
    );
}
