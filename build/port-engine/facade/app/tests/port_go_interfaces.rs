//! Interfaces, end to end: impls from observed satisfaction, and what embedding becomes.
//!
//! Over the emitted crate rather than over hand-built nodes, because both properties are about
//! whole declarations relating to each other — which interfaces a type was SEEN satisfying, and
//! what a type gains from what it embeds.

use port_engine_app::driver;

#[test]
fn trait_impls_are_emitted_from_observed_satisfaction() {
    let report = driver::port_go_pipeline().expect("the Go corpus must port");
    let source = driver::assemble_modules(&report);

    for expected in [
        "impl crate::shapes::Named for Label",
        "impl crate::shapes::Named for Tag",
        "observed satisfying `crate::shapes::Named` at assertion",
        // The delegating body is a PATH call, not a method call: inside a trait impl, `self.name()`
        // resolves against the trait being implemented and recurses into itself.
        "Label::name(self)",
    ] {
        assert!(
            source.contains(expected),
            "emitted source must carry `{expected}`:\n{source}"
        );
    }
}

#[test]
fn a_trait_method_binds_the_receiver_its_implementors_need() {
    let report = driver::port_go_pipeline().expect("the Go corpus must port");
    let source = driver::assemble_modules(&report);

    assert!(
        source.contains("fn name(&self) -> String;"),
        "a read-only trait method must bind shared:\n{source}"
    );
    assert!(
        source.contains("fn rename(&mut self, next: String);"),
        "a mutating trait method must bind exclusive:\n{source}"
    );
}

#[test]
fn embedding_becomes_supertraits_and_forwarding_methods() {
    let report = driver::port_go_pipeline().expect("the Go corpus must port");
    let source = driver::assemble_modules(&report);

    for expected in [
        // An embedded interface is a REQUIREMENT, not a copy of its method set.
        "pub trait Job: Runner + Describer {}",
        // A promoted method is forwarded through the field it was promoted from.
        "self.engine.run()",
        // And the impl of the outer trait carries nothing of its own, because the trait does not.
        "impl Job for Driver {}",
        "impl Runner for Driver",
    ] {
        assert!(
            source.contains(expected),
            "emitted source must carry `{expected}`:\n{source}"
        );
    }
}

#[test]
fn a_forwarding_method_inherits_its_receiver_from_what_it_forwards_to() {
    let report = driver::port_go_pipeline().expect("the Go corpus must port");
    let source = driver::assemble_modules(&report);

    assert!(
        source.contains("pub fn run(&mut self) -> i64 {\n            self.engine.run()"),
        "a forwarding method for a mutating method must bind exclusively:\n{source}"
    );
}
