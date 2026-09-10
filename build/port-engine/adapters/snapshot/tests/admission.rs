//! Snapshot admission: the pin binding, the cross-language digest, and every refusal.

use std::collections::{BTreeMap, BTreeSet};

use port_engine_api::{Declaration, Digest, SourceModel, TypeRef, UnitId};
use port_engine_frontend_go::{PRODUCER_BOOTSTRAP_GO, PRODUCER_OWNED_RUST};
use port_engine_hash::digest_bytes;
use port_engine_snapshot::*;

#[test]
fn slice8_claims_snapshot_readiness() {
    assert!(w0_ready());
}

#[test]
fn semantic_preimage_is_injective_across_field_boundaries() {
    let producer = PRODUCER_BOOTSTRAP_GO;
    let embedded_delimiters = format!("x\0{producer}\0y");
    let one_unit = snapshot_preimage("go", &[(embedded_delimiters.as_str(), producer)]);
    let two_units = snapshot_preimage("go", &[("x", producer), ("y", producer)]);
    assert_ne!(one_unit, two_units);
}

#[test]
fn embedded_fixture_admits_and_binds_pin() {
    let admitted = admit_embedded_fixture().expect("fixture must admit");
    assert!(!admitted.pin().is_empty());
    assert_eq!(
        admitted.model_digest().0,
        "sha256:5a3bca44537be2cc8d1cb909616b741e8e4e1d1b879dc231e40dfc56d75e3f7a"
    );
    assert_eq!(
        admitted.artifact_digest(),
        &digest_bytes(include_str!("../src/fixture-snapshot-v0.json").as_bytes())
    );
    assert_eq!(
        admitted.as_model().snapshot_digest(),
        admitted.artifact_digest().clone()
    );
    assert_eq!(admitted.as_model().units().len(), 2);
    assert_eq!(
        admitted.producer_for(&UnitId("example.com/a".into())),
        Some(PRODUCER_BOOTSTRAP_GO)
    );
}

/// The cross-language check. This fixture's `snapshot_digest` was computed by the Go
/// extractor over ITS encoder; admission recomputes it here over the Rust one. The test
/// passing means the two implementations agree byte-for-byte over a real declaration tree —
/// which is the whole reason mirroring the encoder is acceptable rather than reckless.
#[test]
fn v1_fixture_admits_and_carries_declarations() {
    let admitted = admit_embedded_fixture_v1().expect("v1 fixture must admit");

    let units = admitted.as_model().units();
    // The package SET, not its size. A count has to be edited every time a corpus package lands,
    // which makes the edit routine and the check ceremonial; a set says which package went missing.
    let names: BTreeSet<&str> = units
        .iter()
        .filter_map(|unit| unit.0.rsplit('/').next())
        .collect();
    assert_eq!(
        names,
        BTreeSet::from([
            "basic",
            "composite",
            "fallible",
            "geometry",
            "naming",
            "pointers",
            "shapes"
        ]),
        "every corpus package must be admitted"
    );

    let basic = units
        .iter()
        .find(|u| u.0.ends_with("basic"))
        .expect("basic package");
    let declarations = admitted
        .as_model()
        .declarations(basic)
        .expect("a unit in the model answers Some");
    assert!(
        declarations.len() >= 8,
        "basic declares consts, vars, an alias, a named type and functions, got {}",
        declarations.len()
    );

    let add = declarations
        .iter()
        .find(|d| d.name == "Add")
        .expect("`Add` is declared");
    assert_eq!(add.kind, "func");
    assert!(add.has_flag("exported"));
    assert_eq!(add.children_of_kind("param").len(), 2);
    assert_eq!(add.children_of_kind("result").len(), 1);

    let shapes = units
        .iter()
        .find(|u| u.0.ends_with("shapes"))
        .expect("shapes package");
    let shape_decls = admitted
        .as_model()
        .declarations(shapes)
        .expect("a unit in the model answers Some");
    let point = shape_decls
        .iter()
        .find(|d| d.name == "Point")
        .expect("`Point` is declared");
    assert_eq!(point.kind, "struct");
    assert_eq!(point.children_of_kind("field").len(), 3);
    assert_eq!(point.children_of_kind("method").len(), 2);
}

#[test]
fn unknown_unit_answers_none_not_an_empty_model() {
    let admitted = admit_embedded_fixture_v1().expect("v1 fixture must admit");
    assert!(
        admitted
            .as_model()
            .declarations(&UnitId("nothing/here".into()))
            .is_none(),
        "an unknown unit must be distinguishable from one that declares nothing"
    );
}

/// The v0 preimage covers language and the package→producer map only. If a v1 artifact were
/// digested with it, every declaration would sit outside the snapshot identity: a renamed
/// field or a changed parameter type would leave `snapshot_digest` untouched, and the receipt
/// would then see emitted bytes move with all six axes unchanged — the exact `Unexplained`
/// verdict the axes exist to prevent, arriving for a change that is fully explainable.
#[test]
fn v1_preimage_moves_when_a_declaration_moves() {
    let producer = PRODUCER_BOOTSTRAP_GO;
    let base = Declaration {
        kind: "const".into(),
        name: "MaxRetries".into(),
        type_ref: TypeRef::basic("int"),
        flags: ["exported".to_owned()].into_iter().collect(),
        attrs: [("value".to_owned(), "3".to_owned())].into_iter().collect(),
        children: Vec::new(),
    };

    let mut retyped = base.clone();
    retyped.type_ref = TypeRef::basic("int64");
    let mut unexported = base.clone();
    unexported.flags.clear();

    let original = snapshot_preimage_v1("go", &[("u", producer, vec![base.clone()])]);
    let after_type = snapshot_preimage_v1("go", &[("u", producer, vec![retyped])]);
    let after_flag = snapshot_preimage_v1("go", &[("u", producer, vec![unexported])]);

    assert_ne!(original, after_type, "a changed type must move the digest");
    assert_ne!(original, after_flag, "a changed flag must move the digest");
}

/// Nesting must be unambiguous, not merely encoded. Without the explicit child arity, a node
/// with one child would flatten into the same byte string as two sibling nodes, and the whole
/// declaration tree could be reshaped without moving the digest.
#[test]
fn v1_preimage_distinguishes_nesting_from_sibling_order() {
    let producer = PRODUCER_BOOTSTRAP_GO;
    let leaf = |name: &str| Declaration {
        kind: "param".into(),
        name: name.into(),
        type_ref: TypeRef::basic("int"),
        flags: std::collections::BTreeSet::new(),
        attrs: std::collections::BTreeMap::new(),
        children: Vec::new(),
    };

    let nested = Declaration {
        kind: "func".into(),
        name: "f".into(),
        type_ref: TypeRef::default(),
        flags: std::collections::BTreeSet::new(),
        attrs: std::collections::BTreeMap::new(),
        children: vec![leaf("a")],
    };
    let mut flat = nested.clone();
    flat.children.clear();

    assert_ne!(
        snapshot_preimage_v1("go", &[("u", producer, vec![nested])]),
        snapshot_preimage_v1("go", &[("u", producer, vec![flat, leaf("a")])]),
    );
}

#[test]
fn refuses_digest_mismatch() {
    let json = r#"{
  "language": "go",
  "snapshot_digest": "sha256:deadbeef",
  "packages": [
{"unit_id": "example.com/a", "producer": "bootstrap-go-packages-go-types"}
  ]
}"#;
    let bytes = json.as_bytes();
    let err = admit_reproducible_pair(bytes, bytes).expect_err("bad digest must refuse");
    assert!(matches!(err, AdmitError::DigestMismatch { .. }));
}

#[test]
fn refuses_byte_drift_between_extractor_passes() {
    let first = include_str!("../src/fixture-snapshot-v0.json").as_bytes();
    let mut second = first.to_vec();
    second.push(b'\n');

    let err = admit_reproducible_pair(first, &second)
        .expect_err("semantically equivalent snapshots with byte drift must refuse");
    assert_eq!(
        err,
        AdmitError::SnapshotMismatch {
            first: digest_bytes(first),
            second: digest_bytes(&second),
        }
    );
}

#[test]
fn refuses_non_go_bootstrap_language() {
    let json = r#"{
  "language": "rust",
  "snapshot_digest": "sha256:unused",
  "packages": []
}"#;
    let bytes = json.as_bytes();
    let err = admit_reproducible_pair(bytes, bytes)
        .expect_err("bootstrap admission must refuse a non-Go language");
    assert_eq!(
        err,
        AdmitError::Language {
            actual: "rust".to_owned(),
        }
    );
}

#[test]
fn refuses_owned_frontend_before_equivalence() {
    let unit = "example.com/a";
    let digest = digest_bytes(&snapshot_preimage(
        "go",
        &[(unit, port_engine_frontend_go::PRODUCER_OWNED_RUST)],
    ));
    let json = format!(
        r#"{{
  "language": "go",
  "snapshot_digest": "{}",
  "packages": [
{{"unit_id": "{unit}", "producer": "owned-rust-go-front-end"}}
  ]
}}"#,
        digest.0
    );
    let bytes = json.as_bytes();
    let err = admit_reproducible_pair(bytes, bytes)
        .expect_err("owned front end needs the later equivalence authorization");
    assert_eq!(
        err,
        AdmitError::ProducerNotAuthorized {
            unit: unit.to_owned(),
            actual: port_engine_frontend_go::PRODUCER_OWNED_RUST.to_owned(),
        }
    );
}
