#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use observability_domain::{Severity, should_emit};

#[test]
fn from_otel_int_returns_none_for_zero() {
    assert_eq!(Severity::from_otel_int(0), None);
}

#[test]
fn from_otel_int_returns_none_for_value_above_24() {
    assert_eq!(Severity::from_otel_int(25), None);
    assert_eq!(Severity::from_otel_int(255), None);
}

#[test]
fn from_otel_int_maps_range_1_to_4_to_trace() {
    for n in 1u8..=4 {
        assert_eq!(
            Severity::from_otel_int(n),
            Some(Severity::Trace),
            "expected Trace for OTel SeverityNumber {n}"
        );
    }
}

#[test]
fn from_otel_int_maps_range_5_to_8_to_debug() {
    for n in 5u8..=8 {
        assert_eq!(
            Severity::from_otel_int(n),
            Some(Severity::Debug),
            "expected Debug for OTel SeverityNumber {n}"
        );
    }
}

#[test]
fn from_otel_int_maps_range_9_to_12_to_info() {
    for n in 9u8..=12 {
        assert_eq!(
            Severity::from_otel_int(n),
            Some(Severity::Info),
            "expected Info for OTel SeverityNumber {n}"
        );
    }
}

#[test]
fn from_otel_int_maps_range_13_to_16_to_warn() {
    for n in 13u8..=16 {
        assert_eq!(
            Severity::from_otel_int(n),
            Some(Severity::Warn),
            "expected Warn for OTel SeverityNumber {n}"
        );
    }
}

#[test]
fn from_otel_int_maps_range_17_to_20_to_error() {
    for n in 17u8..=20 {
        assert_eq!(
            Severity::from_otel_int(n),
            Some(Severity::Error),
            "expected Error for OTel SeverityNumber {n}"
        );
    }
}

#[test]
fn from_otel_int_maps_range_21_to_24_to_fatal() {
    for n in 21u8..=24 {
        assert_eq!(
            Severity::from_otel_int(n),
            Some(Severity::Fatal),
            "expected Fatal for OTel SeverityNumber {n}"
        );
    }
}

#[test]
fn from_otel_int_round_trips_all_canonical_ints_via_as_otel_int() {
    for s in Severity::all() {
        let canonical = s.as_otel_int();
        assert_eq!(
            Severity::from_otel_int(canonical),
            Some(s),
            "round-trip failed for {s:?} (canonical int {canonical})"
        );
    }
}

#[test]
fn from_otel_int_acceptance_criteria_spot_checks() {
    assert_eq!(Severity::from_otel_int(4), Some(Severity::Trace));
    assert_eq!(Severity::from_otel_int(5), Some(Severity::Debug));
    assert_eq!(Severity::from_otel_int(20), Some(Severity::Error));
    assert_eq!(Severity::from_otel_int(21), Some(Severity::Fatal));
    assert_eq!(Severity::from_otel_int(0), None);
    assert_eq!(Severity::from_otel_int(25), None);
}

#[test]
fn should_emit_is_reexported_from_crate_root() {
    // Compilation of this test proves the re-export exists.
    let _result = should_emit(Severity::Info, Severity::Info);
}

#[test]
fn should_emit_returns_false_when_record_below_threshold() {
    assert!(!should_emit(Severity::Info, Severity::Warn));
}

#[test]
fn should_emit_returns_true_when_record_above_threshold() {
    assert!(should_emit(Severity::Error, Severity::Warn));
}

#[test]
fn should_emit_returns_true_when_record_equals_threshold() {
    assert!(should_emit(Severity::Warn, Severity::Warn));
}

#[test]
fn should_emit_trace_threshold_passes_all_levels() {
    for record in Severity::all() {
        assert!(
            should_emit(record, Severity::Trace),
            "{record:?} should be emitted when threshold is Trace"
        );
    }
}

#[test]
fn should_emit_fatal_threshold_passes_only_fatal() {
    for record in Severity::all() {
        let expected = record == Severity::Fatal;
        assert_eq!(
            should_emit(record, Severity::Fatal),
            expected,
            "{record:?} vs Fatal threshold: expected {expected}"
        );
    }
}

#[test]
fn should_emit_full_6x6_threshold_matrix_matches_ord() {
    let levels = Severity::all();
    for &record in &levels {
        for &threshold in &levels {
            let expected = record >= threshold;
            assert_eq!(
                should_emit(record, threshold),
                expected,
                "should_emit({record:?}, {threshold:?}) expected {expected}"
            );
        }
    }
}

#[test]
fn should_emit_lower_triangle_never_emits() {
    let levels = Severity::all();
    // levels is ordered Trace < Debug < Info < Warn < Error < Fatal (index 0..5)
    for (i, &threshold) in levels.iter().enumerate() {
        for &record in &levels[..i] {
            assert!(
                !should_emit(record, threshold),
                "{record:?} (below {threshold:?}) must not emit"
            );
        }
    }
}

#[test]
fn should_emit_upper_triangle_always_emits() {
    let levels = Severity::all();
    for (i, &threshold) in levels.iter().enumerate() {
        for &record in &levels[i..] {
            assert!(
                should_emit(record, threshold),
                "{record:?} (>= {threshold:?}) must emit"
            );
        }
    }
}
