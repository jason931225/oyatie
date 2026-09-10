use std::collections::BTreeSet;

use port_engine_api::{
    Digest, LanguagePair, RECEIPT_AXES, Receipt, ReceiptAxis, TypeRef, w0_ready,
};

#[test]
fn seam_types_are_ready() {
    assert!(w0_ready());
    let pair = LanguagePair {
        source: "go".into(),
        target: "rust".into(),
    };
    assert_eq!(pair.slug().unwrap(), "go-rust");
}

fn filled() -> Receipt {
    Receipt {
        pin: "pin".into(),
        snapshot_digest: Digest("snapshot".into()),
        engine_digest: Digest("engine".into()),
        rulepack_digest: Digest("rulepack".into()),
        toolchain_digest: Digest("toolchain".into()),
        formatter_digest: Digest("formatter".into()),
    }
}

/// Every registered axis is wired to a field of its own.
///
/// A variant can be added, its match arms written, and never appended to [`RECEIPT_AXES`]; that
/// compiles and silently un-covers the axis. Moving one field at a time and demanding exactly the
/// matching axis back is what notices, and it also catches an arm wired to the wrong field.
#[test]
fn each_registered_axis_is_bound_to_its_own_field() {
    let moved: Vec<(ReceiptAxis, Receipt)> = vec![
        (
            ReceiptAxis::Pin,
            Receipt {
                pin: "other".into(),
                ..filled()
            },
        ),
        (
            ReceiptAxis::Snapshot,
            Receipt {
                snapshot_digest: Digest("other".into()),
                ..filled()
            },
        ),
        (
            ReceiptAxis::Engine,
            Receipt {
                engine_digest: Digest("other".into()),
                ..filled()
            },
        ),
        (
            ReceiptAxis::RulePack,
            Receipt {
                rulepack_digest: Digest("other".into()),
                ..filled()
            },
        ),
        (
            ReceiptAxis::Toolchain,
            Receipt {
                toolchain_digest: Digest("other".into()),
                ..filled()
            },
        ),
        (
            ReceiptAxis::Formatter,
            Receipt {
                formatter_digest: Digest("other".into()),
                ..filled()
            },
        ),
    ];

    let covered: BTreeSet<ReceiptAxis> = moved.iter().map(|(axis, _)| *axis).collect();
    assert_eq!(
        covered,
        RECEIPT_AXES.into_iter().collect::<BTreeSet<_>>(),
        "an axis is registered that no field moves, so nothing would ever explain it"
    );

    for (axis, other) in moved {
        assert_eq!(
            filled().differing_axes(&other),
            BTreeSet::from([axis]),
            "moving one field must report exactly its own axis"
        );
    }
}

#[test]
fn a_filled_receipt_has_no_incomplete_axes() {
    assert!(filled().incomplete_axes().is_empty());
}

/// `is_empty` ignores `package`, which reads as an oversight and is not one.
///
/// It decides the present/absent marker in the snapshot digest preimage, so the current answer is
/// baked into every digest already recorded. Pinned here so a "fix" has to be a deliberate one.
#[test]
fn a_type_carrying_only_a_package_is_still_empty() {
    let only_package = TypeRef {
        package: "example.com/pkg".into(),
        ..TypeRef::default()
    };
    assert!(only_package.is_empty());

    assert!(TypeRef::default().is_empty());
    assert!(!TypeRef::basic("int").is_empty());
    assert!(!TypeRef::of("slice").is_empty());
}
