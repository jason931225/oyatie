//! The migration value vocabulary: total conversions, backfill constants
//! with exact canonical digest bytes per variant, and the FNV-1a-64 digest
//! the plan identity rides on.

use data_ontology_kernel::ScalarType;
use foundry_edits::{WireDouble, WireValue};

/// Total value conversions — each defined for every value of its input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueConversion {
    IntegerToString,
    BooleanToInteger,
}

/// A backfill value with exact canonical digest bytes per variant.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DefaultValue {
    String(String),                  // data_class: PROPERTY_VALUE_PRIVACY_CLASS
    Integer(i64),                    // data_class: PROPERTY_VALUE_PRIVACY_CLASS
    Boolean(bool),                   // data_class: PROPERTY_VALUE_PRIVACY_CLASS
    Double(WireDouble),              // data_class: PROPERTY_VALUE_PRIVACY_CLASS
    Timestamp { epoch_millis: i64 }, // data_class: PROPERTY_VALUE_PRIVACY_CLASS
}

impl DefaultValue {
    /// The wire carrier the runner writes.
    pub fn to_wire(&self) -> WireValue {
        match self {
            Self::String(value) => WireValue::String(value.clone()),
            Self::Integer(value) => WireValue::Integer(*value),
            Self::Boolean(value) => WireValue::Boolean(*value),
            Self::Double(value) => WireValue::Double(*value),
            Self::Timestamp { epoch_millis } => WireValue::Timestamp {
                epoch_millis: *epoch_millis,
            },
        }
    }

    pub(super) fn scalar_type(&self) -> ScalarType {
        match self {
            Self::String(_) => ScalarType::String,
            Self::Integer(_) => ScalarType::Integer,
            Self::Boolean(_) => ScalarType::Boolean,
            Self::Double(_) => ScalarType::Double,
            Self::Timestamp { .. } => ScalarType::Timestamp,
        }
    }

    pub(super) fn digest_into(&self, digest: &mut Fnv1a64) {
        match self {
            Self::String(value) => {
                digest.write(&[1]);
                digest.write(value.as_bytes());
            }
            Self::Integer(value) => {
                digest.write(&[2]);
                digest.write(&value.to_be_bytes());
            }
            Self::Boolean(value) => {
                digest.write(&[3, u8::from(*value)]);
            }
            Self::Double(value) => {
                digest.write(&[4]);
                digest.write(&value.get().to_bits().to_be_bytes());
            }
            Self::Timestamp { epoch_millis } => {
                digest.write(&[5]);
                digest.write(&epoch_millis.to_be_bytes());
            }
        }
    }
}

pub(super) struct Fnv1a64(u64);

impl Fnv1a64 {
    pub(super) fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    pub(super) fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    pub(super) fn finish(&self) -> u64 {
        self.0
    }
}
