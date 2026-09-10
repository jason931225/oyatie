//! The refusal proofs: real Go the engine declines to translate, and what it says about it.
//!
//! Each class has its OWN corpus: one corpus carrying several classes proves whichever package the
//! transform reaches first and leaves the rest untested while looking tested.
//!
//! Every assertion checks that the message NAMES the thing — the construct, the site, the position,
//! and where the missing analysis belongs.

use port_engine_app::driver;

/// Discarding the companion value is sound exactly when it is the zero value, and a silent loss of
/// work when it is not — so the engine admits literals and the absent value, and refuses anything
/// computed rather than deciding that some expression is "obviously" zero.
#[test]
fn a_failing_return_that_carries_a_value_is_refused_with_its_reason() {
    let err = driver::port_go_refused_failure()
        .expect_err("a computed value beside a failure has no target shape");

    let message = err.to_string();
    assert!(
        message.contains("Sized"),
        "the refusal must name the declaration: {message}"
    );
    assert!(
        message.contains("carries only the failure"),
        "the refusal must say what the target's shape IS: {message}"
    );
    assert!(
        message.contains("lose work"),
        "the refusal must say what would be lost: {message}"
    );
}

#[test]
fn an_interface_in_an_undeclared_position_is_refused_with_its_reason() {
    let err = driver::port_go_refused_interface()
        .expect_err("an interface result has no declared target form");

    let message = err.to_string();
    assert!(
        message.contains("Speaker"),
        "the refusal must name the trait: {message}"
    );
    assert!(
        message.contains("result"),
        "the refusal must name the position: {message}"
    );
    assert!(
        message.contains("owns the value"),
        "the refusal must say what the missing decision IS: {message}"
    );
}

#[test]
fn the_refusal_corpus_is_refused_by_name() {
    let err = driver::port_go_refused().expect_err("the refusal corpus must not translate");

    let message = err.to_string();
    assert!(
        message.contains("ForStmt") || message.contains("DeferStmt"),
        "the refusal must name the construct it refused, got: {message}"
    );
    assert!(
        message.contains("census"),
        "the refusal must point at where the analysis lives, got: {message}"
    );
}

#[test]
fn an_escaping_receiver_is_refused_with_its_reason() {
    let err =
        driver::port_go_refused_ownership().expect_err("an escaping receiver has no borrow form");

    let message = err.to_string();
    assert!(
        message.contains("Itself"),
        "the refusal must name the site: {message}"
    );
    assert!(
        message.contains("escaping_owned"),
        "the refusal must name the disposition that declined: {message}"
    );
    assert!(
        message.contains("outlives the call"),
        "the refusal must carry the pack's recorded reason: {message}"
    );
}
