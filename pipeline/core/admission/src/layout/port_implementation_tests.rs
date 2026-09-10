//! A port defined by a change carries its implementation in the same change.
//!
//! Every fixture is a single-line literal, so no line of this file begins with
//! a `trait` item and the file cannot trip the rule it pins.

use super::*;

const PORT: &str = "cell/ports/placement/src/store.rs";
const ADAPTER: &str = "cell/adapters/postgres/src/lib.rs";

fn added<'a>(path: &'a str, head: &'a str) -> ChangedSource<'a> {
    ChangedSource {
        path,
        head: head.as_bytes(),
        base: None,
    }
}

fn refusals(changed: &[ChangedSource<'_>]) -> Vec<String> {
    port_implementation_violations(changed)
}

#[test]
fn a_port_with_no_implementation_is_refused_at_its_definition() {
    let refused = refusals(&[added(
        PORT,
        "use x;\npub trait CellStore: Send + Sync {\n}\n",
    )]);
    assert_eq!(refused.len(), 1, "{refused:?}");
    assert!(refused[0].starts_with(&format!("{PORT}:2:")), "{refused:?}");
    assert!(refused[0].contains("`CellStore`"), "{refused:?}");
}

#[test]
fn a_stub_is_the_absence_of_an_implementation_not_one() {
    let port = "pub trait CellStore {}\npub struct NotImplementedCellStore;\nimpl CellStore for NotImplementedCellStore {}\n";
    assert_eq!(refusals(&[added(PORT, port)]).len(), 1);
}

#[test]
fn an_adapter_in_another_crate_admits_the_port() {
    let port = added(PORT, "pub trait CellStore {}\n");
    let adapter = added(
        ADAPTER,
        "impl cell_placement::CellStore for PostgresCellStore {}\n",
    );
    assert!(refusals(&[port, adapter]).is_empty());
}

#[test]
fn a_test_fake_admits_the_port_because_a_fake_means_a_caller() {
    let port = "pub trait CellStore {}\n#[cfg(test)]\nmod tests {\n    impl super::CellStore for Fake {}\n}\n";
    assert!(refusals(&[added(PORT, port)]).is_empty());
}

#[test]
fn a_port_the_base_already_defined_is_not_charged_to_an_edit_beside_it() {
    let before = "pub trait CellStore {}\n";
    let after = "pub trait CellStore {}\npub fn touched() {}\n";
    let edited = ChangedSource {
        path: PORT,
        head: after.as_bytes(),
        base: Some(before.as_bytes()),
    };
    assert!(refusals(&[edited]).is_empty());
}

#[test]
fn a_port_added_to_an_existing_file_is_still_charged() {
    let before = "pub trait CellStore {}\n";
    let after = "pub trait CellStore {}\npub trait CellDrain {}\n";
    let edited = ChangedSource {
        path: PORT,
        head: after.as_bytes(),
        base: Some(before.as_bytes()),
    };
    let refused = refusals(&[edited]);
    assert_eq!(refused.len(), 1, "{refused:?}");
    assert!(refused[0].contains("`CellDrain`"), "{refused:?}");
}

#[test]
fn a_port_moved_between_files_is_inherited_not_introduced() {
    let from = ChangedSource {
        path: PORT,
        head: b"",
        base: Some(b"pub trait CellStore {}\n"),
    };
    let to = added(
        "cell/ports/placement/src/cell_store.rs",
        "pub trait CellStore {}\n",
    );
    assert!(refusals(&[from, to]).is_empty());
}

#[test]
fn every_visibility_and_modifier_spelling_is_a_definition() {
    for head in [
        "trait CellStore {}\n",
        "pub(crate) trait CellStore {}\n",
        "pub(in crate::port) trait CellStore {}\n",
        "pub unsafe trait CellStore {}\n",
        "unsafe trait CellStore {}\n",
    ] {
        assert_eq!(refusals(&[added(PORT, head)]).len(), 1, "{head:?}");
    }
}

#[test]
fn a_generic_implementation_header_names_its_trait() {
    let port = added(PORT, "pub trait CellStore<T> {}\n");
    let adapter = added(
        ADAPTER,
        "impl<'a, T: for<'b> Fn(&'b T)> CellStore<T> for Adapter<'a, T> {}\n",
    );
    assert!(refusals(&[port, adapter]).is_empty());
}

#[test]
fn a_port_named_only_in_prose_is_not_a_definition() {
    for head in [
        "// trait CellStore {}\n",
        "/// pub trait CellStore {}\n",
        "/*\npub trait CellStore {}\n*/\n",
    ] {
        assert!(refusals(&[added(PORT, head)]).is_empty(), "{head:?}");
    }
}

#[test]
fn an_inherent_implementation_implements_nothing() {
    assert_eq!(
        refusals(&[added(PORT, "pub trait CellStore {}\nimpl CellStore {}\n")]).len(),
        1
    );
}

#[test]
fn only_rust_sources_are_read() {
    assert!(
        refusals(&[added(
            "cell/ports/placement/README.md",
            "pub trait CellStore {}\n"
        )])
        .is_empty()
    );
}

#[test]
fn the_refusal_names_no_decision_identifier() {
    let refused = refusals(&[added(PORT, "pub trait CellStore {}\n")]).join("\n");
    assert!(!refused.is_empty(), "no refusal to inspect");
    assert!(
        !refused.contains("ADR-") && !refused.contains("D-"),
        "{refused}"
    );
}

#[test]
fn a_header_rustfmt_wraps_before_for_still_names_its_trait() {
    let port = added(
        PORT,
        "pub trait IdentityProviderRegistrySnapshotRepository {}\n",
    );
    let adapter = added(
        ADAPTER,
        "impl IdentityProviderRegistrySnapshotRepository\n    for InMemoryIdentityProviderRegistrySnapshotRepository\n{\n}\n",
    );
    assert!(refusals(&[port, adapter]).is_empty());
}

#[test]
fn a_header_whose_generics_wrap_still_names_its_trait() {
    for header in [
        "impl<\n    A: Send,\n    B: Send,\n> CellStore<A, B> for Postgres<A, B> {\n}\n",
        "impl\nCellStore<\n    CandidateRequest,\n    CandidateArtifact,\n> for ReindeerCandidate {\n}\n",
    ] {
        let adapter = added(ADAPTER, header);
        assert!(
            refusals(&[added(PORT, "pub trait CellStore<A, B> {}\n"), adapter]).is_empty(),
            "{header:?}"
        );
    }
}

#[test]
fn a_stub_hidden_in_a_wrapper_or_a_wrapped_header_is_still_a_stub() {
    for port in [
        "pub trait CellStore {}\nimpl CellStore\n    for NotImplementedCellStore\n{\n}\n",
        "pub trait CellStore {}\nimpl CellStore for Box<NotImplementedCellStore> {}\n",
    ] {
        assert_eq!(refusals(&[added(PORT, port)]).len(), 1, "{port:?}");
    }
}

#[test]
fn an_impl_that_only_forwards_to_other_implementers_is_not_one() {
    for port in [
        "pub trait CellStore {}\nimpl CellStore for NotImplementedCellStore {}\nimpl<T: CellStore + ?Sized> CellStore for std::sync::Arc<T> {}\n",
        "pub trait CellStore {}\nimpl<T> CellStore for Arc<T>\nwhere\n    T: CellStore + ?Sized,\n{\n}\n",
    ] {
        assert_eq!(refusals(&[added(PORT, port)]).len(), 1, "{port:?}");
    }
    let blanket = "pub trait CellStore {}\nimpl<T: CellStoreRead> CellStore for T {}\n";
    assert!(refusals(&[added(PORT, blanket)]).is_empty());
}

#[test]
fn a_standard_library_trait_of_the_same_name_implements_nothing() {
    let port = added(PORT, "pub trait Error {}\n");
    let unrelated = added(ADAPTER, "impl std::error::Error for PlacementError {}\n");
    assert_eq!(refusals(&[port, unrelated]).len(), 1);
}

#[test]
fn reference_slice_unit_and_macro_targets_are_implementations() {
    for adapter in [
        "impl CellStore for &FakeClock {}\n",
        "impl CellStore for [u8] {}\n",
        "impl CellStore for () {}\n",
        "macro_rules! adapt {\n    ($t:ty) => {\n        impl CellStore for $t {}\n    };\n}\n",
    ] {
        let port = added(PORT, "pub trait CellStore {}\n");
        assert!(
            refusals(&[port, added(ADAPTER, adapter)]).is_empty(),
            "{adapter:?}"
        );
    }
}

#[test]
fn a_return_arrow_does_not_close_an_inline_generic_list() {
    let port = added(PORT, "pub trait Clock {}\n");
    let adapter = added(ADAPTER, "impl<F: Fn() -> u64> Clock for F {}\n");
    assert!(refusals(&[port, adapter]).is_empty());

    let named = "pub trait Response {}\nimpl<F: Fn() -> Response> Clock for F {}\n";
    assert_eq!(refusals(&[added(PORT, named)]).len(), 1);
}

#[test]
fn a_bound_naming_another_crates_trait_does_not_forward() {
    let port = added(PORT, "pub trait Write {}\n");
    let adapter = added(ADAPTER, "impl<W: std::io::Write> Write for LogSink<W> {}\n");
    assert!(refusals(&[port, adapter]).is_empty());

    let service = added(PORT, "pub trait Service {}\n");
    let tower = added(
        ADAPTER,
        "impl<S> Service for TowerAdapter<S>\nwhere\n    S: tower::Service<Request>,\n{\n}\n",
    );
    assert!(refusals(&[service, tower]).is_empty());
}

#[test]
fn an_implementation_for_a_dyn_form_of_the_same_trait_forwards() {
    let port = "pub trait CellStore {}\npub struct NotImplementedCellStore;\nimpl CellStore for NotImplementedCellStore {}\nimpl CellStore for Arc<dyn CellStore + Send + Sync> {}\n";
    assert_eq!(refusals(&[added(PORT, port)]).len(), 1);
}

#[test]
fn an_impl_line_inside_a_string_does_not_swallow_the_next_impl() {
    let port = added(PORT, "pub trait CellStore {}\n");
    let adapter = added(
        ADAPTER,
        "fn fixture() -> &'static str {\n    r#\"\nimpl Broken\n\"#\n}\n\nimpl CellStore for Postgres {}\n",
    );
    assert!(refusals(&[port, adapter]).is_empty());
}

#[test]
fn a_stub_word_inside_a_body_on_the_header_line_is_not_the_target() {
    let port = "pub trait CellStore {}\nimpl CellStore for PostgresCellStore { fn get(&self) { NotImplementedYet::refuse() } }\n";
    assert!(refusals(&[added(PORT, port)]).is_empty());
}
