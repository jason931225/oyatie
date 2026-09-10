//! Protocol-agnostic shapes for the shared platform contracts, modelled ahead of
//! any handler that serves them: `serde` types plus their invariants, with no
//! IO and no transport coupling.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use std::fmt;

pub mod identity;
pub mod pdp;
pub mod shell_bff;
pub mod tenancy;

/// A single contract-invariant violation. Every `validate()` in this crate
/// surfaces ALL of them, never just the first, so a caller can report a whole
/// bad request at once.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContractViolation {
    MissingValue {
        field: &'static str,
    },
    TooLong {
        field: &'static str,
        max: usize,
        actual: usize,
    },
    InvalidCharset {
        field: &'static str,
        value: String,
    },
    /// e.g. an expiry that does not follow its issuance.
    InvalidTemporalOrder {
        field: &'static str,
        detail: String,
    },
    InvalidTransition {
        from: &'static str,
        operation: &'static str,
    },
    /// A shape invariant not covered by the variants above.
    InvalidShape {
        field: &'static str,
        detail: String,
    },
    /// Duplicate id, dangling reference, ambiguous route, cross-tenant mismatch.
    BrokenReference {
        field: &'static str,
        detail: String,
    },
}

impl fmt::Display for ContractViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingValue { field } => write!(f, "{field}: required value is missing"),
            Self::TooLong { field, max, actual } => {
                write!(f, "{field}: length {actual} exceeds maximum {max}")
            }
            Self::InvalidCharset { field, value } => {
                write!(f, "{field}: value {value:?} contains disallowed characters")
            }
            Self::InvalidTemporalOrder { field, detail } => {
                write!(f, "{field}: temporal order violated ({detail})")
            }
            Self::InvalidTransition { from, operation } => {
                write!(
                    f,
                    "lifecycle: operation {operation} is not allowed from state {from}"
                )
            }
            Self::InvalidShape { field, detail } => write!(f, "{field}: {detail}"),
            Self::BrokenReference { field, detail } => write!(f, "{field}: {detail}"),
        }
    }
}

impl std::error::Error for ContractViolation {}

pub const MAX_ID_LEN: usize = 128;
pub const MAX_DISPLAY_NAME_LEN: usize = 256;
pub const MAX_OPAQUE_TOKEN_LEN: usize = 512;

fn is_slug_char(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '.' | '_')
}

pub(crate) fn check_slug(
    field: &'static str,
    value: &str,
    max: usize,
    out: &mut Vec<ContractViolation>,
) {
    if value.is_empty() {
        out.push(ContractViolation::MissingValue { field });
        return;
    }
    if value.len() > max {
        out.push(ContractViolation::TooLong {
            field,
            max,
            actual: value.len(),
        });
    }
    let first_ok = value
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    if !first_ok || !value.chars().all(is_slug_char) {
        out.push(ContractViolation::InvalidCharset {
            field,
            value: value.to_owned(),
        });
    }
}

pub(crate) fn check_text(
    field: &'static str,
    value: &str,
    max: usize,
    out: &mut Vec<ContractViolation>,
) {
    if value.trim().is_empty() {
        out.push(ContractViolation::MissingValue { field });
        return;
    }
    if value.len() > max {
        out.push(ContractViolation::TooLong {
            field,
            max,
            actual: value.len(),
        });
    }
}

pub(crate) fn check_opaque_token(
    field: &'static str,
    value: &str,
    out: &mut Vec<ContractViolation>,
) {
    if value.is_empty() {
        out.push(ContractViolation::MissingValue { field });
        return;
    }
    if value.len() > MAX_OPAQUE_TOKEN_LEN {
        out.push(ContractViolation::TooLong {
            field,
            max: MAX_OPAQUE_TOKEN_LEN,
            actual: value.len(),
        });
    }
    if value.chars().any(char::is_whitespace) {
        out.push(ContractViolation::InvalidCharset {
            field,
            value: value.to_owned(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_accepts_canonical_ids() {
        for ok in ["acme", "tenant-1", "a.b_c-d", "0abc"] {
            let mut out = Vec::new();
            check_slug("field", ok, MAX_ID_LEN, &mut out);
            assert!(out.is_empty(), "{ok}: {out:?}");
        }
    }

    #[test]
    fn slug_rejects_empty_uppercase_and_bad_first_char() {
        for bad in ["", "Acme", "-leading-dash", "has space", "uni\u{e9}"] {
            let mut out = Vec::new();
            check_slug("field", bad, MAX_ID_LEN, &mut out);
            assert!(!out.is_empty(), "{bad}: expected violation");
        }
    }

    #[test]
    fn slug_surfaces_length_and_charset_together() {
        let long_upper = "A".repeat(MAX_ID_LEN + 1);
        let mut out = Vec::new();
        check_slug("field", &long_upper, MAX_ID_LEN, &mut out);
        assert_eq!(
            out.len(),
            2,
            "surface-all must report both violations: {out:?}"
        );
    }

    #[test]
    fn opaque_token_rejects_whitespace_and_empty() {
        let mut out = Vec::new();
        check_opaque_token("token", "", &mut out);
        assert_eq!(
            out,
            vec![ContractViolation::MissingValue { field: "token" }]
        );
        out.clear();
        check_opaque_token("token", "a b", &mut out);
        assert!(matches!(
            out.as_slice(),
            [ContractViolation::InvalidCharset { .. }]
        ));
    }

    #[test]
    fn violations_render_human_legible_messages() {
        let v = ContractViolation::TooLong {
            field: "tenant_id",
            max: 8,
            actual: 9,
        };
        assert_eq!(v.to_string(), "tenant_id: length 9 exceeds maximum 8");
    }
}
