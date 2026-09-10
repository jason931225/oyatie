//! Crate-wide error type shared across every operating-system subsystem.

use alloc::string::String;
use core::fmt;

pub type Result<T> = core::result::Result<T, Error>;

/// A subsystem either uses these variants directly or converts its own richer
/// error into one at the crate boundary. The `Display` arms carry the message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Invalid(String),
    NotFound(String),
    PermissionDenied(String),
    InvalidState(String),
    Parse(String),
    Timeout,
    Unsupported(String),
    /// Uncategorized. Prefer a variant above wherever one fits.
    Other(String),
}

impl Error {
    pub fn invalid(msg: impl Into<String>) -> Self {
        Error::Invalid(msg.into())
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Error::NotFound(msg.into())
    }

    pub fn permission_denied(msg: impl Into<String>) -> Self {
        Error::PermissionDenied(msg.into())
    }

    pub fn invalid_state(msg: impl Into<String>) -> Self {
        Error::InvalidState(msg.into())
    }

    pub fn parse(msg: impl Into<String>) -> Self {
        Error::Parse(msg.into())
    }

    pub fn unsupported(msg: impl Into<String>) -> Self {
        Error::Unsupported(msg.into())
    }

    /// Stable across releases: callers and log queries match on it.
    pub fn kind(&self) -> &'static str {
        match self {
            Error::Invalid(_) => "invalid",
            Error::NotFound(_) => "not_found",
            Error::PermissionDenied(_) => "permission_denied",
            Error::InvalidState(_) => "invalid_state",
            Error::Parse(_) => "parse",
            Error::Timeout => "timeout",
            Error::Unsupported(_) => "unsupported",
            Error::Other(_) => "other",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Invalid(m) => write!(f, "invalid: {m}"),
            Error::NotFound(m) => write!(f, "not found: {m}"),
            Error::PermissionDenied(m) => write!(f, "permission denied: {m}"),
            Error::InvalidState(m) => write!(f, "invalid state: {m}"),
            Error::Parse(m) => write!(f, "parse error: {m}"),
            Error::Timeout => write!(f, "operation timed out"),
            Error::Unsupported(m) => write!(f, "unsupported: {m}"),
            Error::Other(m) => write!(f, "error: {m}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn kind_strings_are_stable() {
        assert_eq!(Error::invalid("x").kind(), "invalid");
        assert_eq!(Error::not_found("x").kind(), "not_found");
        assert_eq!(Error::permission_denied("x").kind(), "permission_denied");
        assert_eq!(Error::Timeout.kind(), "timeout");
    }

    #[test]
    fn display_includes_message() {
        assert_eq!(
            Error::parse("bad version").to_string(),
            "parse error: bad version"
        );
        assert_eq!(Error::Timeout.to_string(), "operation timed out");
    }

    #[test]
    fn constructors_accept_str_and_string() {
        let a = Error::invalid("literal");
        let b = Error::invalid("owned".to_string());
        assert_ne!(a, b);
        assert_eq!(a, Error::Invalid("literal".to_string()));
    }
}
