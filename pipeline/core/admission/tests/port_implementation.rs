//! A port defined by a change carries its implementation in the same change.
//!
//! Every fixture is a single-line literal, so no line of this file begins with
//! a `trait` item and the file cannot trip the rule it pins.

use pipeline_admission::{ChangedSource, port_implementation_violations};

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
