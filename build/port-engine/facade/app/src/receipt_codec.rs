//! Stable receipt encoding + golden compare.
//!
//! Hand-rolled key=value lines in fixed axis order — no serde, and therefore no lock absorb.

use port_engine_api::Receipt;
use port_engine_hash::digest_bytes;

pub const GOLDEN_RECEIPT_V0: &str = include_str!("golden-receipt-v0.txt");

/// Encode a receipt as stable ordered lines (trailing newline).
#[must_use]
pub fn format_receipt(receipt: &Receipt) -> String {
    format!(
        "pin={}\nsnapshot_digest={}\nengine_digest={}\nrulepack_digest={}\ntoolchain_digest={}\nformatter_digest={}\n",
        receipt.pin,
        receipt.snapshot_digest.0,
        receipt.engine_digest.0,
        receipt.rulepack_digest.0,
        receipt.toolchain_digest.0,
        receipt.formatter_digest.0,
    )
}

/// The value the golden records for an axis it deliberately does not pin.
pub const GOLDEN_VARIES: &str = "<varies>";

/// True when `receipt` matches the embedded golden, axis by axis.
///
/// `engine_digest` is a content hash of the engine's own sources, so it moves on every commit that
/// touches the engine. The golden records [`GOLDEN_VARIES`] for it and this asserts its SHAPE
/// instead, which still catches an axis gone empty or malformed.
#[must_use]
pub fn matches_golden(receipt: &Receipt) -> bool {
    let actual = normalize(&format_receipt(receipt));
    let expected = normalize(GOLDEN_RECEIPT_V0);

    let mut lines = actual.lines().zip(expected.lines());
    let paired = lines
        .by_ref()
        .all(|(actual, expected)| match expected.split_once('=') {
            Some((axis, GOLDEN_VARIES)) => actual.starts_with(axis) && well_formed_digest(actual),
            _ => actual == expected,
        });
    // Arity is part of the claim: an axis added or dropped must not read as a match.
    paired && actual.lines().count() == expected.lines().count()
}

fn well_formed_digest(line: &str) -> bool {
    let Some((_, value)) = line.split_once('=') else {
        return false;
    };
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Per-region digests, sorted by region id.
///
/// The roll-up answers "did anything change"; this answers "how much, and where". A whole-program
/// decision can touch a handful of declarations and every call site that uses them, and under one
/// digest that is indistinguishable from an accidental one-line change.
#[must_use]
pub fn region_digests(
    emitted: &std::collections::BTreeMap<port_engine_api::RegionId, Vec<u8>>,
) -> Vec<(String, String)> {
    emitted
        .iter()
        .map(|(region, bytes)| (region.0.clone(), digest_bytes(bytes).0))
        .collect()
}

#[must_use]
pub fn emit_tree_digest(
    emitted: &std::collections::BTreeMap<port_engine_api::RegionId, Vec<u8>>,
) -> port_engine_api::Digest {
    let mut preimage = Vec::new();
    for (region, bytes) in emitted {
        push_length_prefixed(&mut preimage, region.0.as_bytes());
        push_length_prefixed(&mut preimage, bytes);
    }
    digest_bytes(&preimage)
}

/// Append `bytes` under its own length, the way the snapshot and engine preimages do.
///
/// A separator-delimited encoding is only unambiguous while the separator cannot appear in the
/// content, and emitted source is arbitrary; the length prefix makes the encoding injective without
/// relying on that.
fn push_length_prefixed(preimage: &mut Vec<u8>, bytes: &[u8]) {
    preimage.extend_from_slice(bytes.len().to_string().as_bytes());
    preimage.push(b':');
    preimage.extend_from_slice(bytes);
}

fn normalize(text: &str) -> String {
    // Accept either LF or CRLF golden checkouts; keep final newline.
    let mut body = text.replace("\r\n", "\n");
    if !body.ends_with('\n') {
        body.push('\n');
    }
    body
}

#[cfg(test)]
mod tests {
    use super::*;
    use port_engine_api::Digest;
    use std::collections::BTreeMap;

    #[test]
    fn format_receipt_is_six_ordered_lines() {
        let r = Receipt {
            pin: "p".into(),
            snapshot_digest: Digest("s".into()),
            engine_digest: Digest("e".into()),
            rulepack_digest: Digest("r".into()),
            toolchain_digest: Digest("t".into()),
            formatter_digest: Digest("f".into()),
        };
        let text = format_receipt(&r);
        let lines: Vec<_> = text.lines().collect();
        assert_eq!(lines.len(), 6);
        assert!(lines[0].starts_with("pin="));
        assert!(lines[5].starts_with("formatter_digest="));
        assert!(text.ends_with('\n'));
    }

    #[test]
    fn emit_tree_digest_is_order_stable() {
        let mut a = BTreeMap::new();
        a.insert(port_engine_api::RegionId("b".into()), b"1".to_vec());
        a.insert(port_engine_api::RegionId("a".into()), b"0".to_vec());
        let mut b = BTreeMap::new();
        b.insert(port_engine_api::RegionId("a".into()), b"0".to_vec());
        b.insert(port_engine_api::RegionId("b".into()), b"1".to_vec());
        assert_eq!(emit_tree_digest(&a), emit_tree_digest(&b));
    }

    /// Two different region trees cannot share one preimage.
    ///
    /// Without the length prefix, a region id and the bytes beside it run together: `ab` + `c` and
    /// `a` + `bc` would encode identically and one emit would hash as the other.
    #[test]
    fn a_region_boundary_cannot_be_moved_without_moving_the_digest() {
        let mut left = BTreeMap::new();
        left.insert(port_engine_api::RegionId("ab".into()), b"c".to_vec());
        let mut right = BTreeMap::new();
        right.insert(port_engine_api::RegionId("a".into()), b"bc".to_vec());
        assert_ne!(emit_tree_digest(&left), emit_tree_digest(&right));
    }
}
