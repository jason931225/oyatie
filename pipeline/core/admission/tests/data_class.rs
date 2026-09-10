//! The data-class vocabulary stays in one crate, and a classified struct
//! answers for every field it holds.

use pipeline_admission::{data_class_home_violations, unclassified_field_violations};

const OUTSIDE: &str = "network/core/route/src/lib.rs";
const CANONICAL: &str = "data/core/data-boundary-kernel/src/lib.rs";

fn home(path: &str, text: &str) -> Vec<String> {
    data_class_home_violations(path, text.as_bytes())
}

fn holes(text: &str) -> Vec<String> {
    unclassified_field_violations(OUTSIDE, text.as_bytes())
}

#[test]
fn a_new_parallel_definition_is_refused_outside_the_canonical_crate() {
    let refusals = home(OUTSIDE, "pub enum RouteDataClass {\n    Public,\n}\n");
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    let refusal = &refusals[0];
    assert!(
        refusal.contains(&format!("{OUTSIDE}:1:")) && refusal.contains("enum RouteDataClass"),
        "{refusal}"
    );
}

#[test]
fn the_canonical_crate_owns_the_vocabulary() {
    assert!(
        home(CANONICAL, "pub enum DataClass {\n    Public,\n}\n").is_empty(),
        "the canonical crate must be free to declare its own vocabulary"
    );
}

#[test]
fn every_grandfathered_declaration_is_admitted_where_it_stands() {
    for (path, name) in [
        ("iam/ports/tenant-rbac-api/src/lib.rs", "DataClassDto"),
        ("app/foundry/core/edits/src/property.rs", "WireDataClass"),
        (
            "intelligence/core/kernel/src/safety.rs",
            "EvidenceDataClass",
        ),
    ] {
        let text = format!("pub enum {name} {{\n    Public,\n}}\n");
        assert!(home(path, &text).is_empty(), "{path}: {name}");
    }
}

#[test]
fn a_grandfathered_declaration_relocated_loses_its_grandfather() {
    let text = "pub enum DataClassDto {\n    Public,\n}\n";
    assert!(
        !home("iam/ports/tenant-rbac-api/src/moved.rs", text).is_empty(),
        "the exemption is keyed to the declaration site, so a move must go red"
    );
    assert!(
        !home("iam/ports/other-api/src/lib.rs", text).is_empty(),
        "the exemption must not follow the name into another crate"
    );
}

#[test]
fn prose_naming_an_enum_is_not_a_declaration() {
    for line in [
        "// pub enum RouteDataClass {",
        "/// See `pub enum RouteDataClass {` for the shape.",
        " * pub enum RouteDataClass {",
    ] {
        let text = format!("{line}\npub struct Route;\n");
        assert!(home(OUTSIDE, &text).is_empty(), "{line}");
    }
}

#[test]
fn a_declaration_whose_brace_wrapped_to_the_next_line_is_still_refused() {
    let refusals = home(OUTSIDE, "pub enum RouteDataClass\n{\n    Public,\n}\n");
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert!(
        refusals[0].contains("enum RouteDataClass"),
        "{}",
        refusals[0]
    );
}

#[test]
fn an_unrelated_class_enum_is_not_a_data_class() {
    for name in ["DatastoreClass", "CapacityClass", "NodeClass"] {
        let text = format!("pub enum {name} {{\n    Public,\n}}\n");
        assert!(home(OUTSIDE, &text).is_empty(), "{name}");
    }
}

#[test]
fn visibility_spellings_do_not_hide_a_declaration() {
    for head in [
        "enum RouteDataClass {",
        "pub enum RouteDataClass {",
        "pub(crate) enum RouteDataClass {",
        "pub(in crate::route) enum RouteDataClass {",
        "    pub enum RouteDataClass {",
    ] {
        let text = format!("{head}\n    Public,\n}}\n");
        assert!(!home(OUTSIDE, &text).is_empty(), "{head}");
    }
}

#[test]
fn a_silent_primitive_beside_a_classified_field_is_refused() {
    let refusals =
        holes("pub struct Row {\n    pub id: Classified<String>,\n    pub email: String,\n}\n");
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert!(
        refusals[0].contains(&format!("{OUTSIDE}:3:")) && refusals[0].contains("Row.email"),
        "{}",
        refusals[0]
    );
}

#[test]
fn either_form_of_the_declaration_satisfies_the_rule() {
    for field in [
        "pub email: Classified<String>,",
        "pub email: String, // data_class: PII_IDENTIFYING",
        "pub email: PrivacyDataClass,",
        "pub email: DataClassification,",
    ] {
        let text = format!("pub struct Row {{\n    pub id: Classified<String>,\n    {field}\n}}\n");
        assert!(holes(&text).is_empty(), "{field}");
    }
}

#[test]
fn an_annotation_on_the_line_above_the_field_counts() {
    let text = "pub struct Row {\n    pub id: Classified<String>,\n    \
                // data_class: PII_IDENTIFYING\n    pub email: String,\n}\n";
    assert!(holes(text).is_empty(), "{:?}", holes(text));
}

#[test]
fn a_struct_that_classifies_nothing_is_left_alone() {
    let text = "pub struct Row {\n    pub id: String,\n    pub email: String,\n}\n";
    assert!(
        holes(text).is_empty(),
        "the rule only reaches structs that already classify a field"
    );
}

#[test]
fn a_domain_typed_field_delegates_its_class_and_is_not_a_hole() {
    let text =
        "pub struct Row {\n    pub id: Classified<String>,\n    pub scope: BudgetScope,\n}\n";
    assert!(
        holes(text).is_empty(),
        "a domain type declares its own fields' classes"
    );
}

#[test]
fn a_non_rust_path_carries_neither_rule() {
    let declaration = "pub enum RouteDataClass {\n    Public,\n}\n";
    assert!(!home(OUTSIDE, declaration).is_empty(), "positive control");
    assert!(home("network/core/route/README.txt", declaration).is_empty());

    let hole = "pub struct Row {\n    pub id: Classified<String>,\n    pub email: String,\n}\n";
    assert!(!holes(hole).is_empty(), "positive control");
    assert!(
        unclassified_field_violations("network/core/route/notes.txt", hole.as_bytes()).is_empty(),
        "a non-Rust path carries no field rule even when its text would refuse"
    );
}

#[test]
fn a_recorded_hole_is_admitted_only_where_it_stands() {
    let text = "pub struct Capability {\n    pub tier: Classified<u8>,\n    pub id: String,\n}\n";
    let admitted = unclassified_field_violations(
        "intelligence/core/capability-domain/src/lib.rs",
        text.as_bytes(),
    );
    assert!(admitted.is_empty(), "{admitted:?}");
    assert!(
        !unclassified_field_violations("intelligence/core/other/src/lib.rs", text.as_bytes())
            .is_empty(),
        "a recorded hole must not follow its struct into another file"
    );
}

#[test]
fn a_new_hole_in_a_file_that_already_has_one_is_still_refused() {
    let text = "pub struct Capability {\n    pub tier: Classified<u8>,\n    \
                pub id: String,\n    pub owner_email: String,\n}\n";
    let refusals = unclassified_field_violations(
        "intelligence/core/capability-domain/src/lib.rs",
        text.as_bytes(),
    );
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert!(
        refusals[0].contains("Capability.owner_email"),
        "{}",
        refusals[0]
    );
}

#[test]
fn a_neighbours_annotation_does_not_vouch_for_the_field_below_it() {
    let text = "pub struct Row {\n    pub id: Classified<String>,\n    \
                pub action: Action, // data_class: INTERNAL_ONLY\n    pub email: String,\n}\n";
    let refusals = holes(text);
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert!(refusals[0].contains("Row.email"), "{}", refusals[0]);
}

#[test]
fn a_declaration_outside_a_comment_is_not_an_annotation() {
    let text =
        "pub struct Row {\n    pub id: Classified<String>,\n    pub data_class: String,\n}\n";
    let refusals = holes(text);
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert!(refusals[0].contains("Row.data_class"), "{}", refusals[0]);
}

#[test]
fn a_generic_struct_head_does_not_hide_its_fields() {
    for head in [
        "pub struct Row<T> {",
        "pub struct Row<'a> {",
        "pub struct Row<'a, T: Debug> {",
    ] {
        let text = format!("{head}\n    pub id: Classified<String>,\n    pub email: String,\n}}\n");
        let refusals = holes(&text);
        assert_eq!(refusals.len(), 1, "{head}: {refusals:?}");
        assert!(refusals[0].contains("Row.email"), "{head}: {}", refusals[0]);
    }
}

#[test]
fn a_struct_nested_in_a_module_ends_at_its_own_brace() {
    let siblings = "mod m {\n    pub struct A {\n        pub id: Classified<String>,\n    }\n    \
                    pub struct B {\n        pub email: String,\n    }\n}\n";
    assert!(
        holes(siblings).is_empty(),
        "a silent sibling must not be read as a field of the classified struct above it: {:?}",
        holes(siblings)
    );
    let nested = "mod m {\n    pub struct A {\n        pub id: Classified<String>,\n        \
                  pub email: String,\n    }\n}\n";
    let refusals = holes(nested);
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert!(refusals[0].contains("A.email"), "{}", refusals[0]);
}

#[test]
fn a_generic_tuple_struct_head_does_not_adopt_the_next_struct() {
    let text = "pub struct Id<T>(pub u64);\npub struct Row {\n    \
                pub id: Classified<String>,\n    pub email: String,\n}\n";
    let refusals = holes(text);
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert!(refusals[0].contains("Row.email"), "{}", refusals[0]);
}

#[test]
fn a_struct_head_whose_brace_wrapped_past_a_where_clause_is_still_read() {
    let text = "pub struct Row<T>\nwhere\n    T: Debug,\n{\n    \
                pub id: Classified<String>,\n    pub email: String,\n}\n";
    let refusals = holes(text);
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert!(refusals[0].contains("Row.email"), "{}", refusals[0]);
}

#[test]
fn a_paren_after_the_brace_does_not_hide_the_struct_head() {
    let text = "pub struct Row { // (the wire shape)\n    \
                pub id: Classified<String>,\n    pub email: String,\n}\n";
    let refusals = holes(text);
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert!(refusals[0].contains("Row.email"), "{}", refusals[0]);
}
