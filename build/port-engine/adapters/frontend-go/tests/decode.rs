//! Snapshot decode: the envelope, the closed vocabularies, and every refusal.

use port_engine_api::{Digest, UnitId};
use port_engine_frontend_go::{
    GoSourceModel, PRODUCER_BOOTSTRAP_GO, SCHEMA_VERSION_DECLARATIONS, SCHEMA_VERSION_FLAT_TYPES,
    SCHEMA_VERSION_IDENTITY_ONLY, SnapshotError, w0_ready,
};

const FIXTURE: &str = r#"{
  "language": "go",
  "snapshot_digest": "sha256:fixture-slice4",
  "packages": [
{"unit_id": "example.com/a", "producer": "bootstrap-go-packages-go-types"},
{"unit_id": "example.com/b", "producer": "bootstrap-go-packages-go-types"}
  ]
}"#;

#[test]
fn slice4_claims_readiness() {
    assert!(w0_ready());
}

#[test]
fn decodes_ordered_units_and_producers() {
    let model = GoSourceModel::decode_str(FIXTURE).expect("fixture must decode");
    assert_eq!(model.language(), "go");
    assert_eq!(
        model.snapshot_digest(),
        Digest("sha256:fixture-slice4".into())
    );
    assert_eq!(
        model.units(),
        vec![
            UnitId("example.com/a".into()),
            UnitId("example.com/b".into())
        ]
    );
    assert_eq!(
        model.producer_for(&UnitId("example.com/a".into())),
        Some(PRODUCER_BOOTSTRAP_GO)
    );
}

#[test]
fn refuses_unknown_producer() {
    let json = r#"{"language":"go","snapshot_digest":"d","packages":[{"unit_id":"x","producer":"gccgo"}]}"#;
    let err = GoSourceModel::decode_str(json).expect_err("unknown producer must refuse");
    assert!(matches!(err, SnapshotError::UnknownProducer { .. }));
}

#[test]
fn refuses_duplicate_unit() {
    let json = r#"{"language":"go","snapshot_digest":"d","packages":[{"unit_id":"x","producer":"bootstrap-go-packages-go-types"},{"unit_id":"x","producer":"bootstrap-go-packages-go-types"}]}"#;
    let err = GoSourceModel::decode_str(json).expect_err("duplicate unit must refuse");
    assert!(matches!(err, SnapshotError::DuplicateUnit { .. }));
}

#[test]
fn refuses_nul_in_unit_identity() {
    let json = r#"{"language":"go","snapshot_digest":"d","packages":[{"unit_id":"x\u0000y","producer":"bootstrap-go-packages-go-types"}]}"#;
    let err = GoSourceModel::decode_str(json)
        .expect_err("NUL would make the semantic snapshot preimage ambiguous");
    assert_eq!(
        err,
        SnapshotError::Schema {
            field: "packages.unit_id",
        }
    );
}

const STRUCTURED: &str = r#"{
  "schema_version": 2,
  "language": "go",
  "snapshot_digest": "sha256:fixture-v1",
  "packages": [
{
  "unit_id": "example.com/a",
  "producer": "bootstrap-go-packages-go-types",
  "declarations": [
    {"kind": "const", "name": "Max", "type": {"kind": "basic", "name": "int"}, "flags": ["exported"]},
    {"kind": "func", "name": "Add", "flags": ["exported"], "children": [
      {"kind": "param", "name": "a", "type": {"kind": "basic", "name": "int"}},
      {"kind": "param", "name": "b", "type": {"kind": "basic", "name": "int"}},
      {"kind": "result", "name": "", "type": {"kind": "basic", "name": "int"}}
    ]}
  ]
}
  ]
}"#;

fn with_declarations(body: &str) -> String {
    format!(
        r#"{{"schema_version":2,"language":"go","snapshot_digest":"d","packages":[{{"unit_id":"x","producer":"{PRODUCER_BOOTSTRAP_GO}","declarations":[{body}]}}]}}"#
    )
}

#[test]
fn decodes_the_structured_declaration_tree() {
    let model = GoSourceModel::decode_str(STRUCTURED).expect("the structured fixture must decode");
    assert_eq!(model.schema_version(), SCHEMA_VERSION_DECLARATIONS);

    let declarations = model
        .declarations_for(&UnitId("example.com/a".into()))
        .expect("unit is present");
    assert_eq!(declarations.len(), 2);

    let add = &declarations[1];
    assert_eq!(add.kind, "func");
    assert!(add.has_flag("exported"));
    assert_eq!(add.children.len(), 3);
    assert_eq!(add.children_of_kind("param").len(), 2);
    assert_eq!(add.children_of_kind("result")[0].type_ref.kind, "basic");
    assert_eq!(add.children_of_kind("result")[0].type_ref.name, "int");
}

#[test]
fn v0_artifact_still_decodes_and_declares_nothing() {
    let model = GoSourceModel::decode_str(FIXTURE).expect("v0 fixture must still decode");
    assert_eq!(model.schema_version(), SCHEMA_VERSION_IDENTITY_ONLY);
    assert_eq!(
        model.declarations_for(&UnitId("example.com/a".into())),
        Some(Vec::new())
    );
}

#[test]
fn refuses_unknown_schema_version() {
    let json = r#"{"schema_version":9,"language":"go","snapshot_digest":"d","packages":[]}"#;
    let err = GoSourceModel::decode_str(json).expect_err("a future version must refuse");
    assert_eq!(err, SnapshotError::UnknownSchemaVersion { actual: 9 });
}

#[test]
fn refuses_the_superseded_flat_type_version() {
    let json = r#"{"schema_version":1,"language":"go","snapshot_digest":"d","packages":[]}"#;
    let err = GoSourceModel::decode_str(json).expect_err("a v1 artifact must be regenerated");
    assert_eq!(
        err,
        SnapshotError::UnknownSchemaVersion {
            actual: SCHEMA_VERSION_FLAT_TYPES
        }
    );
}

#[test]
fn refuses_v0_envelope_carrying_declarations() {
    let json = format!(
        r#"{{"language":"go","snapshot_digest":"d","packages":[{{"unit_id":"x","producer":"{PRODUCER_BOOTSTRAP_GO}","declarations":[{{"kind":"const","name":"K"}}]}}]}}"#
    );
    let err = GoSourceModel::decode_str(&json).expect_err("version/payload lie must refuse");
    assert!(matches!(err, SnapshotError::VersionPayloadMismatch { .. }));
}

#[test]
fn refuses_declaration_kind_outside_the_closed_vocabulary() {
    let json = with_declarations(r#"{"kind":"goroutine","name":"g"}"#);
    let err = GoSourceModel::decode_str(&json).expect_err("unknown kind must refuse");
    assert!(matches!(err, SnapshotError::UnknownDeclarationKind { .. }));
}

/// A member kind at package scope, or a package-scope kind nested inside a declaration, is a
/// structural error and not merely an unusual shape — `param` is not a thing a package
/// declares, and `struct` is not a thing a parameter list contains.
#[test]
fn refuses_member_kind_at_package_scope() {
    let json =
        with_declarations(r#"{"kind":"param","name":"a","type":{"kind":"basic","name":"int"}}"#);
    let err = GoSourceModel::decode_str(&json).expect_err("member kind at top level refuses");
    assert!(matches!(err, SnapshotError::UnknownDeclarationKind { .. }));
}

#[test]
fn refuses_package_scope_kind_nested_as_a_member() {
    let json =
        with_declarations(r#"{"kind":"func","name":"f","children":[{"kind":"const","name":"K"}]}"#);
    let err = GoSourceModel::decode_str(&json).expect_err("nested package kind refuses");
    assert!(matches!(err, SnapshotError::UnknownDeclarationKind { .. }));
}

#[test]
fn refuses_flag_outside_the_closed_vocabulary() {
    let json = with_declarations(r#"{"kind":"const","name":"K","flags":["exportd"]}"#);
    let err = GoSourceModel::decode_str(&json).expect_err("misspelled flag must refuse");
    assert_eq!(
        err,
        SnapshotError::UnknownFlag {
            unit_id: "x".into(),
            actual: "exportd".into(),
        },
        "a silently dropped `exported` would unexport a declaration with no diagnostic"
    );
}

/// Go gives every package-scope identifier one namespace, so a repeat is proof the extractor
/// lost information rather than a naming choice.
#[test]
fn refuses_duplicate_declaration_name_in_one_namespace() {
    let json = with_declarations(
        r#"{"kind":"const","name":"K","type":{"kind":"basic","name":"int"}},{"kind":"func","name":"K"}"#,
    );
    let err = GoSourceModel::decode_str(&json).expect_err("duplicate name must refuse");
    assert!(matches!(err, SnapshotError::DuplicateDeclaration { .. }));
}

/// The exception that keeps the rule usable: `func(int, int) int` really does declare two
/// nameless parameters, so blank and empty names may repeat.
#[test]
fn admits_repeated_blank_member_names() {
    let json = with_declarations(
        r#"{"kind":"func","name":"f","children":[{"kind":"param","name":"","type":{"kind":"basic","name":"int"}},{"kind":"param","name":"","type":{"kind":"basic","name":"int"}},{"kind":"param","name":"_","type":{"kind":"basic","name":"int"}},{"kind":"param","name":"_","type":{"kind":"basic","name":"int"}}]}"#,
    );
    let model = GoSourceModel::decode_str(&json).expect("unnamed parameters are legal Go");
    let decls = model
        .declarations_for(&UnitId("x".into()))
        .expect("unit present");
    assert_eq!(decls[0].children.len(), 4);
}
