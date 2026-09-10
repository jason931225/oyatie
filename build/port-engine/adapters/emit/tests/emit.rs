//! Canary selection, golden compare, and every materialize destination refusal.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use port_engine_api::RegionId;
use port_engine_emit::*;

#[test]
fn slice13_claims_emit_readiness() {
    assert!(w0_ready());
}

#[test]
fn selects_exactly_one_canary_region() {
    let mut tree = BTreeMap::new();
    tree.insert(
        RegionId("example_com_a__identity".into()),
        b"pub fn example_com_a__identity() {}".to_vec(),
    );
    tree.insert(
        RegionId("example_com_b__canary_empty_unit".into()),
        golden_canary_bytes().to_vec(),
    );
    let art = select_canary(&tree).expect("one canary");
    assert_eq!(art.region.0, "example_com_b__canary_empty_unit");
    assert_matches_golden(&art).expect("golden");
}

#[test]
fn refuses_missing_and_ambiguous() {
    let empty = BTreeMap::new();
    assert!(matches!(
        select_canary(&empty),
        Err(EmitError::MissingCanary)
    ));
    let mut two = BTreeMap::new();
    two.insert(RegionId("a__canary_empty_unit".into()), b"a".to_vec());
    two.insert(RegionId("b__canary_empty_unit".into()), b"b".to_vec());
    assert!(matches!(
        select_canary(&two),
        Err(EmitError::AmbiguousCanary { count: 2 })
    ));
}

#[test]
fn refuses_k8s_destination() {
    let err = validate_canary_out_dir(Path::new("/tmp/k8s/port-engine-canary-out"))
        .expect_err("k8s path");
    assert!(matches!(err, EmitError::PathRefused { .. }));
}

#[test]
fn refuses_wrong_basename() {
    let err = validate_canary_out_dir(Path::new("/tmp/not-canary-out")).expect_err("basename");
    assert!(matches!(err, EmitError::PathRefused { .. }));
}

#[test]
fn materialize_writes_single_file_under_allowlisted_dir() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let root = std::env::temp_dir()
        .join(format!("pe-canary-{nanos}"))
        .join(CANARY_OUT_DIRNAME);
    let artifact = CanaryArtifact {
        region: RegionId("example_com_b__canary_empty_unit".into()),
        bytes: golden_canary_bytes().to_vec(),
        digest: golden_canary_digest(),
    };
    let dest = materialize_canary_roundtrip(&root, &artifact).expect("materialize+roundtrip");
    assert_eq!(
        dest.file_name().and_then(|s| s.to_str()),
        Some(CANARY_FILENAME)
    );
    let written = fs::read(&dest).expect("read back");
    assert_eq!(written, golden_canary_bytes());
    let _ = fs::remove_dir_all(root.parent().expect("parent"));
}

#[test]
fn tree_materialize_keeps_every_destination_refusal() {
    for path in [
        "/tmp/k8s/port-engine-emit-out",
        "/tmp/../port-engine-emit-out",
        "/tmp/port-engine-canary-out",
        "/tmp/somewhere-else",
    ] {
        let err =
            validate_emit_out_dir(Path::new(path)).expect_err("this destination must be refused");
        assert!(
            matches!(err, EmitError::PathRefused { .. }),
            "{path}: {err}"
        );
    }
}

#[test]
fn tree_materialize_writes_one_file_per_region() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let root = std::env::temp_dir()
        .join(format!("pe-tree-{nanos}"))
        .join(EMIT_OUT_DIRNAME);

    let mut emitted = BTreeMap::new();
    emitted.insert(
        RegionId("basic__go_const__max".into()),
        b"pub const M: i64 = 1;".to_vec(),
    );
    emitted.insert(
        RegionId("basic__go_func__add".into()),
        b"pub fn add() {}".to_vec(),
    );

    let written = materialize_tree(&root, &emitted).expect("tree materialize");
    assert_eq!(written.len(), 2);
    for (path, (_, bytes)) in written.iter().zip(emitted.iter()) {
        assert_eq!(&fs::read(path).expect("read back"), bytes);
    }
    let _ = fs::remove_dir_all(root.parent().expect("parent"));
}

#[test]
fn every_allowlisted_basename_is_accepted_by_exactly_one_validator() {
    for name in ALLOWED_OUT_DIRNAMES {
        let dir = Path::new("out").join(name);
        assert_ne!(
            validate_canary_out_dir(&dir).is_ok(),
            validate_emit_out_dir(&dir).is_ok(),
            "`{name}` is allowlisted but no validator claims it, or both do"
        );
    }
    assert!(validate_canary_out_dir(Path::new("out/port-engine-other-out")).is_err());
}

#[test]
fn tree_materialize_refuses_a_region_id_that_is_not_a_bare_identifier() {
    let root = std::env::temp_dir().join(EMIT_OUT_DIRNAME);
    for hostile in ["../escape", "a/b", "", "with space"] {
        let mut emitted = BTreeMap::new();
        emitted.insert(RegionId(hostile.into()), b"pub fn x() {}".to_vec());
        let err = materialize_tree(&root, &emitted)
            .expect_err("a region id that is not an identifier must refuse");
        assert!(
            matches!(err, EmitError::PathRefused { .. }),
            "{hostile}: {err}"
        );
    }
    let _ = fs::remove_dir_all(&root);
}
