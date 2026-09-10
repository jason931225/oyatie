//! The canonical ULID surface: lexicographically sortable, 128-bit,
//! Crockford-base32 identifiers with no central allocator.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use std::fmt;

/// Canonical ULID — 26-character Crockford-base32 string (always uppercase).
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Ulid(String);

impl Ulid {
    /// Construct after full ULID spec validation.
    ///
    /// Input is normalised to uppercase before storage, so lowercase Crockford
    /// symbols are accepted (Crockford-base32 is case-insensitive by spec).
    ///
    /// # Errors
    /// `IdGeneratorError::MalformedUlid` when the input is not 26 symbols
    /// after uppercasing, carries a byte outside the Crockford-base32
    /// alphabet, or names a timestamp beyond year ~10889.
    pub fn try_new(raw: impl Into<String>) -> Result<Self, IdGeneratorError> {
        let raw = raw.into().to_ascii_uppercase();
        if raw.len() != 26 {
            return Err(IdGeneratorError::MalformedUlid(raw));
        }
        if !has_representable_timestamp(&raw) {
            return Err(IdGeneratorError::MalformedUlid(raw));
        }
        for byte in raw.as_bytes() {
            if !is_crockford_base32(*byte) {
                return Err(IdGeneratorError::MalformedUlid(raw));
            }
        }
        Ok(Ulid(raw))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A ULID packs its 48-bit timestamp into the first 10 base32 symbols (50
/// bits), so the leading symbol carries only 3 of its 5 bits. Anything above
/// this names a timestamp beyond year ~10889.
const MAX_TIMESTAMP_LEAD_SYMBOL: u8 = b'7';

fn has_representable_timestamp(raw: &str) -> bool {
    raw.as_bytes()
        .first()
        .is_some_and(|lead| *lead <= MAX_TIMESTAMP_LEAD_SYMBOL && lead.is_ascii_digit())
}

/// `I`, `L`, `O` and `U` are excluded from the Crockford alphabet: they read
/// as `1`, `1`, `0` and `V`. The gaps below are deliberate.
const fn is_crockford_base32(byte: u8) -> bool {
    matches!(
        byte,
        b'0'..=b'9'
            | b'A'..=b'H'
            | b'J'..=b'K'
            | b'M'..=b'N'
            | b'P'..=b'T'
            | b'V'..=b'Z'
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IdGeneratorError {
    MalformedUlid(String),
    SkeletonNotYetImplemented(&'static str),
}

impl fmt::Display for IdGeneratorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdGeneratorError::MalformedUlid(value) => {
                write!(f, "shared-ulid-id-kernel: malformed ULID {value:?}")
            }
            IdGeneratorError::SkeletonNotYetImplemented(method) => write!(
                f,
                "shared-ulid-id-kernel: {method} is skeleton-only \
                 (tracked under registry/placeholder-debt/adr-follow-ups.yaml#adr-0156-ulid-impl)"
            ),
        }
    }
}

impl std::error::Error for IdGeneratorError {}

/// The trait every µservice integrates to mint canonical ULIDs.
pub trait IdGenerator: Send + Sync {
    /// Mint a fresh ULID.
    ///
    /// # Errors
    /// - `SkeletonNotYetImplemented` for the skeleton impl.
    fn new_ulid(&self) -> Result<Ulid, IdGeneratorError>;
}

/// Deterministic generator used in tests — produces ULIDs in sequence
/// from a base prefix.
#[derive(Default)]
pub struct SeededIdGenerator {
    counter: std::sync::Mutex<u64>,
}

#[allow(clippy::expect_used)]
impl IdGenerator for SeededIdGenerator {
    fn new_ulid(&self) -> Result<Ulid, IdGeneratorError> {
        let mut c = self.counter.lock().expect("mutex poisoned");
        *c += 1;
        // Hand-rolled 26-char Crockford-base32 prefix; tests don't
        // depend on temporal monotonicity.
        let raw = format!("01HMZ{:021}", *c);
        Ulid::try_new(raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ulid_accepts_26_char_crockford() {
        let id = Ulid::try_new("01HMZ1234567890ABCDEFGHJKM").expect("ok");
        assert_eq!(id.as_str().len(), 26);
    }

    #[test]
    fn ulid_rejects_wrong_length() {
        assert!(matches!(
            Ulid::try_new("too-short"),
            Err(IdGeneratorError::MalformedUlid(_))
        ));
    }

    #[test]
    fn ulid_rejects_invalid_crockford_byte() {
        assert!(matches!(
            Ulid::try_new("01HMZ1234567890ABCDEFGHJKI"),
            Err(IdGeneratorError::MalformedUlid(_))
        ));
        // Lowercase is now accepted (normalised to uppercase at the boundary).
        let lc = Ulid::try_new("01hmz1234567890abcdefghjkm").expect("lowercase accepted");
        assert_eq!(lc.as_str(), "01HMZ1234567890ABCDEFGHJKM");
    }

    #[test]
    fn seeded_generator_emits_distinct_ulids() {
        let id_gen = SeededIdGenerator::default();
        let a = id_gen.new_ulid().expect("a");
        let b = id_gen.new_ulid().expect("b");
        assert_ne!(a, b);
        assert_eq!(a.as_str().len(), 26);
    }

    #[test]
    fn error_display_carries_follow_up_pointer() {
        let err = IdGeneratorError::SkeletonNotYetImplemented("new_ulid");
        let msg = format!("{err}");
        assert!(msg.contains("adr-0156-ulid-impl"));
    }

    #[test]
    fn ulid_rejects_timestamp_overflow() {
        // First char '8' — exactly one above the maximum (7).
        assert!(matches!(
            Ulid::try_new("81HMZ1234567890ABCDEFGHJKM"),
            Err(IdGeneratorError::MalformedUlid(_))
        ));
        // First char '9'.
        assert!(matches!(
            Ulid::try_new("91HMZ1234567890ABCDEFGHJKM"),
            Err(IdGeneratorError::MalformedUlid(_))
        ));
        // First char at the high end of the Crockford alphabet — 'Z'.
        assert!(matches!(
            Ulid::try_new("Z1HMZ1234567890ABCDEFGHJKM"),
            Err(IdGeneratorError::MalformedUlid(_))
        ));
    }

    #[test]
    fn ulid_accepts_lowercase_normalised() {
        // Valid ULID in all-lowercase should be accepted and uppercased.
        let lc = Ulid::try_new("01hmz1234567890abcdefghjkm").expect("lowercase accepted");
        assert_eq!(lc.as_str(), "01HMZ1234567890ABCDEFGHJKM");

        // Mixed case.
        let mixed = Ulid::try_new("01HmZ1234567890aBcDeFgHjKm").expect("mixed case accepted");
        assert_eq!(mixed.as_str(), "01HMZ1234567890ABCDEFGHJKM");
    }

    #[test]
    fn ulid_monotonic_prefix_boundaries() {
        // '7' in position 0 is the maximum valid first character.
        let ok = Ulid::try_new("71HMZ1234567890ABCDEFGHJKM");
        assert!(ok.is_ok(), "first char '7' should be accepted");

        // '8' in position 0 is the first invalid timestamp character.
        let overflow = Ulid::try_new("81HMZ1234567890ABCDEFGHJKM");
        assert!(
            matches!(overflow, Err(IdGeneratorError::MalformedUlid(_))),
            "first char '8' should be rejected as timestamp overflow"
        );

        // '0' is the lowest valid first character.
        let zero_prefix = Ulid::try_new("01HMZ1234567890ABCDEFGHJKM");
        assert!(zero_prefix.is_ok(), "first char '0' should be accepted");
    }
}
