//! Identifier generation traits and the [`Fingerprint`] primitive.

use crate::error::{Error, Result};
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// A short content fingerprint for detecting that a resource changed.
///
/// FNV-1a is NOT cryptographically secure: a fingerprint match must never stand
/// in for authentication, only for change detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fingerprint(u64);

impl Fingerprint {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0100_0000_01b3;

    /// Width of the hex rendering: a `u64` zero-padded to its full extent, so
    /// every fingerprint string sorts and compares as a fixed-width token.
    const HEX_LEN: usize = 16;

    pub fn of(bytes: &[u8]) -> Self {
        let mut hash = Self::FNV_OFFSET;
        for &b in bytes {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(Self::FNV_PRIME);
        }
        Fingerprint(hash)
    }

    pub fn of_str(s: &str) -> Self {
        Self::of(s.as_bytes())
    }

    pub fn value(&self) -> u64 {
        self.0
    }

    pub fn to_hex(&self) -> String {
        alloc::format!("{self}")
    }

    pub fn from_hex(s: &str) -> Result<Self> {
        if s.len() != Self::HEX_LEN {
            return Err(Error::parse(alloc::format!(
                "fingerprint hex must be {} characters",
                Self::HEX_LEN
            )));
        }
        let v = u64::from_str_radix(s, 16).map_err(|_| Error::parse("invalid fingerprint hex"))?;
        Ok(Fingerprint(v))
    }
}

impl fmt::Display for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:0width$x}", self.0, width = Fingerprint::HEX_LEN)
    }
}

/// Abstract so a subsystem can inject a deterministic generator under test.
pub trait IdGenerator {
    fn next_id(&mut self) -> String;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequentialIdGenerator {
    prefix: String,
    counter: u64,
}

impl SequentialIdGenerator {
    pub fn new(prefix: impl Into<String>) -> Self {
        SequentialIdGenerator {
            prefix: prefix.into(),
            counter: 0,
        }
    }

    pub fn count(&self) -> u64 {
        self.counter
    }

    /// Advances the counter by `n`.
    pub fn take(&mut self, n: usize) -> Vec<String> {
        (0..n).map(|_| self.next_id()).collect()
    }
}

impl IdGenerator for SequentialIdGenerator {
    fn next_id(&mut self) -> String {
        let id = alloc::format!("{}-{}", self.prefix, self.counter);
        self.counter += 1;
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_is_deterministic_and_distinct() {
        let a = Fingerprint::of_str("machine-config-v1");
        let b = Fingerprint::of_str("machine-config-v1");
        let c = Fingerprint::of_str("machine-config-v2");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn fingerprint_hex_roundtrip() {
        let f = Fingerprint::of_str("hello world");
        let hex = f.to_hex();
        assert_eq!(hex.len(), Fingerprint::HEX_LEN);
        assert_eq!(Fingerprint::from_hex(&hex).unwrap(), f);
        assert!(Fingerprint::from_hex("xyz").is_err());
        assert!(Fingerprint::from_hex("zzzzzzzzzzzzzzzz").is_err());
    }

    #[test]
    fn sequential_generator_is_monotonic() {
        let mut generator = SequentialIdGenerator::new("svc");
        assert_eq!(generator.next_id(), "svc-0");
        assert_eq!(generator.next_id(), "svc-1");
        assert_eq!(generator.count(), 2);
        let batch = generator.take(3);
        assert_eq!(batch, ["svc-2", "svc-3", "svc-4"]);
    }

    #[test]
    fn hashing_no_bytes_leaves_the_fnv_offset_basis() {
        // The literal, not FNV_OFFSET: comparing the constant to itself would
        // still pass if the constant were wrong.
        assert_eq!(Fingerprint::of(b"").value(), 0xcbf2_9ce4_8422_2325);
    }
}
