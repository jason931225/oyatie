use std::collections::BTreeSet;

use crate::identity::Digest;

/// The receipt axes ADR-0637 (archived; live via apex ADR-0704) fixes. Every emitted-byte change
/// must be attributable to at least one of them; an unattributable change is RED. See
/// `port-engine-kernel::verify`.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ReceiptAxis {
    /// The upstream pin the source snapshot was taken at.
    Pin,
    Snapshot,
    Engine,
    RulePack,
    Toolchain,
    Formatter,
}

/// Every axis, in declaration order.
pub const RECEIPT_AXES: [ReceiptAxis; 6] = [
    ReceiptAxis::Pin,
    ReceiptAxis::Snapshot,
    ReceiptAxis::Engine,
    ReceiptAxis::RulePack,
    ReceiptAxis::Toolchain,
    ReceiptAxis::Formatter,
];

const _: () = assert!(
    RECEIPT_AXES.len() == 6,
    "ADR-0637 fixes the receipt at six axes; widening it is a decision amendment, not a refactor"
);

/// The provenance of one emission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Receipt {
    /// The upstream pin (an opaque revision identifier).
    pub pin: String, // data_class: INTERNAL_ONLY
    pub snapshot_digest: Digest,  // data_class: INTERNAL_ONLY
    pub engine_digest: Digest,    // data_class: INTERNAL_ONLY
    pub rulepack_digest: Digest,  // data_class: INTERNAL_ONLY
    pub toolchain_digest: Digest, // data_class: INTERNAL_ONLY
    pub formatter_digest: Digest, // data_class: INTERNAL_ONLY
}

impl Receipt {
    #[must_use]
    pub fn differing_axes(&self, other: &Self) -> BTreeSet<ReceiptAxis> {
        let mut differing = BTreeSet::new();
        for axis in RECEIPT_AXES {
            let differs = match axis {
                ReceiptAxis::Pin => self.pin != other.pin,
                ReceiptAxis::Snapshot => self.snapshot_digest != other.snapshot_digest,
                ReceiptAxis::Engine => self.engine_digest != other.engine_digest,
                ReceiptAxis::RulePack => self.rulepack_digest != other.rulepack_digest,
                ReceiptAxis::Toolchain => self.toolchain_digest != other.toolchain_digest,
                ReceiptAxis::Formatter => self.formatter_digest != other.formatter_digest,
            };
            if differs {
                differing.insert(axis);
            }
        }
        differing
    }

    /// The axes that say NOTHING — an empty pin or an empty digest.
    ///
    /// [`Receipt::differing_axes`] answers "did this axis move", which is only a usable answer when
    /// the axis carries a value on both sides. An unfilled axis makes an apparent difference
    /// absence of information rather than evidence of a cause, and `port-engine-kernel::verify`
    /// must not spend it as an explanation.
    #[must_use]
    pub fn incomplete_axes(&self) -> BTreeSet<ReceiptAxis> {
        let mut incomplete = BTreeSet::new();
        for axis in RECEIPT_AXES {
            let empty = match axis {
                ReceiptAxis::Pin => self.pin.is_empty(),
                ReceiptAxis::Snapshot => self.snapshot_digest.0.is_empty(),
                ReceiptAxis::Engine => self.engine_digest.0.is_empty(),
                ReceiptAxis::RulePack => self.rulepack_digest.0.is_empty(),
                ReceiptAxis::Toolchain => self.toolchain_digest.0.is_empty(),
                ReceiptAxis::Formatter => self.formatter_digest.0.is_empty(),
            };
            if empty {
                incomplete.insert(axis);
            }
        }
        incomplete
    }
}
