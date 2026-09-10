#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

//! Telemetry severity classification: `as_otel_int` follows the OpenTelemetry
//! SeverityNumber specification, so exporters serialize without re-mapping.

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Severity {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl Severity {
    pub const fn wire_label(self) -> &'static str {
        match self {
            Severity::Trace => "TRACE",
            Severity::Debug => "DEBUG",
            Severity::Info => "INFO",
            Severity::Warn => "WARN",
            Severity::Error => "ERROR",
            Severity::Fatal => "FATAL",
        }
    }

    pub const fn as_otel_int(self) -> u8 {
        match self {
            Severity::Trace => 1,
            Severity::Debug => 5,
            Severity::Info => 9,
            Severity::Warn => 13,
            Severity::Error => 17,
            Severity::Fatal => 21,
        }
    }

    pub const fn all() -> [Self; 6] {
        [
            Severity::Trace,
            Severity::Debug,
            Severity::Info,
            Severity::Warn,
            Severity::Error,
            Severity::Fatal,
        ]
    }

    /// Map an OTel SeverityNumber integer to its `Severity` bucket.
    ///
    /// The OTel spec defines six named levels, each spanning a range of four
    /// integers (the "severity number ranges"). This function maps any integer
    /// in a range to the canonical bucket regardless of position within the
    /// range. Returns `None` for 0 and any value > 24.
    ///
    /// | Range | Bucket |
    /// |-------|--------|
    /// | 1–4   | Trace  |
    /// | 5–8   | Debug  |
    /// | 9–12  | Info   |
    /// | 13–16 | Warn   |
    /// | 17–20 | Error  |
    /// | 21–24 | Fatal  |
    pub fn from_otel_int(n: u8) -> Option<Self> {
        match n {
            1..=4 => Some(Severity::Trace),
            5..=8 => Some(Severity::Debug),
            9..=12 => Some(Severity::Info),
            13..=16 => Some(Severity::Warn),
            17..=20 => Some(Severity::Error),
            21..=24 => Some(Severity::Fatal),
            _ => None,
        }
    }

    pub fn from_wire_label(label: &str) -> Option<Self> {
        match label {
            "TRACE" => Some(Severity::Trace),
            "DEBUG" => Some(Severity::Debug),
            "INFO" => Some(Severity::Info),
            "WARN" => Some(Severity::Warn),
            "ERROR" => Some(Severity::Error),
            "FATAL" => Some(Severity::Fatal),
            _ => None,
        }
    }
}

/// Return `true` when `record_severity` meets or exceeds `min_threshold`.
///
/// Emitters call this before forwarding a log record to any exporter, using
/// the configured minimum severity as the threshold. The implementation
/// delegates to `Ord` which is already derived on `Severity`
/// (Trace < Debug < Info < Warn < Error < Fatal).
pub fn should_emit(record_severity: Severity, min_threshold: Severity) -> bool {
    record_severity >= min_threshold
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.wire_label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownSeverityLabel(pub String);

impl std::fmt::Display for UnknownSeverityLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown severity label: {}", self.0)
    }
}

impl std::error::Error for UnknownSeverityLabel {}

impl TryFrom<&str> for Severity {
    type Error = UnknownSeverityLabel;
    fn try_from(label: &str) -> Result<Self, <Self as TryFrom<&str>>::Error> {
        Self::from_wire_label(label).ok_or_else(|| UnknownSeverityLabel(label.to_string()))
    }
}

impl TryFrom<String> for Severity {
    type Error = UnknownSeverityLabel;
    fn try_from(label: String) -> Result<Self, <Self as TryFrom<String>>::Error> {
        if let Some(sev) = Self::from_wire_label(&label) {
            Ok(sev)
        } else {
            Err(UnknownSeverityLabel(label))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_labels_round_trip() {
        for sev in Severity::all() {
            assert_eq!(Severity::from_wire_label(sev.wire_label()), Some(sev));
        }
    }

    #[test]
    fn unknown_label_returns_none() {
        assert_eq!(Severity::from_wire_label(""), None);
        assert_eq!(Severity::from_wire_label("info"), None);
        assert_eq!(Severity::from_wire_label("CRITICAL"), None);
    }

    #[test]
    fn severity_is_totally_ordered_low_to_high() {
        assert!(Severity::Trace < Severity::Debug);
        assert!(Severity::Debug < Severity::Info);
        assert!(Severity::Info < Severity::Warn);
        assert!(Severity::Warn < Severity::Error);
        assert!(Severity::Error < Severity::Fatal);
    }

    #[test]
    fn otel_int_mapping_matches_spec() {
        assert_eq!(Severity::Trace.as_otel_int(), 1);
        assert_eq!(Severity::Debug.as_otel_int(), 5);
        assert_eq!(Severity::Info.as_otel_int(), 9);
        assert_eq!(Severity::Warn.as_otel_int(), 13);
        assert_eq!(Severity::Error.as_otel_int(), 17);
        assert_eq!(Severity::Fatal.as_otel_int(), 21);
    }

    #[test]
    fn display_renders_wire_label() {
        assert_eq!(format!("{}", Severity::Info), "INFO");
        assert_eq!(format!("{}", Severity::Fatal), "FATAL");
    }

    #[test]
    fn try_from_str_accepts_canonical_label() {
        let sev: Severity = "WARN".try_into().unwrap();
        assert_eq!(sev, Severity::Warn);
    }

    #[test]
    fn try_from_str_rejects_unknown_label() {
        let err: UnknownSeverityLabel = Severity::try_from("warn").unwrap_err();
        assert_eq!(err.0, "warn");
        assert_eq!(err.to_string(), "unknown severity label: warn");
    }

    #[test]
    fn try_from_string_accepts_canonical_label() {
        let sev: Severity = String::from("ERROR").try_into().unwrap();
        assert_eq!(sev, Severity::Error);
    }

    #[test]
    fn all_returns_six_distinct_variants() {
        let all = Severity::all();
        assert_eq!(all.len(), 6);
        let set: std::collections::BTreeSet<_> = all.iter().copied().collect();
        assert_eq!(set.len(), 6);
    }

    #[test]
    fn from_otel_int_bucket_boundaries() {
        assert_eq!(Severity::from_otel_int(4), Some(Severity::Trace));
        assert_eq!(Severity::from_otel_int(5), Some(Severity::Debug));
        assert_eq!(Severity::from_otel_int(20), Some(Severity::Error));
        assert_eq!(Severity::from_otel_int(21), Some(Severity::Fatal));
        assert_eq!(Severity::from_otel_int(0), None);
        assert_eq!(Severity::from_otel_int(25), None);
    }

    #[test]
    fn from_otel_int_round_trips_canonical_ints() {
        for s in Severity::all() {
            assert_eq!(Severity::from_otel_int(s.as_otel_int()), Some(s));
        }
    }

    #[test]
    fn should_emit_threshold_matrix() {
        let levels = Severity::all();
        for &record in &levels {
            for &threshold in &levels {
                assert_eq!(
                    should_emit(record, threshold),
                    record >= threshold,
                    "should_emit({record:?}, {threshold:?}) mismatch",
                );
            }
        }
        // Spot-check the three acceptance criteria.
        assert!(!should_emit(Severity::Info, Severity::Warn));
        assert!(should_emit(Severity::Error, Severity::Warn));
        assert!(should_emit(Severity::Warn, Severity::Warn));
    }
}
