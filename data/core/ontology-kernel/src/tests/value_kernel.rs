//! Pins for the typed value plane: finite-double ordering, calendar
//! validation, storage classes, and the legacy-bridge read path.

use std::collections::BTreeMap;

use crate::*;

#[test]
fn non_finite_doubles_rejected() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(FiniteDouble::new(bad), Err(ValueTypeError::NonFiniteDouble));
    }
    assert!(FiniteDouble::new(f64::MAX).is_ok());
    assert!(FiniteDouble::new(f64::MIN).is_ok());
}

#[test]
fn negative_zero_folds() {
    let neg = FiniteDouble::new(-0.0).unwrap();
    let pos = FiniteDouble::new(0.0).unwrap();
    assert_eq!(neg, pos);
    assert_eq!(neg.get().to_bits(), 0.0f64.to_bits());
}

#[test]
fn derived_ord_equals_numeric_order() {
    let ordered = [
        f64::MIN,
        -1.0e10,
        -2.5,
        -1.0,
        -f64::MIN_POSITIVE,
        0.0,
        f64::MIN_POSITIVE,
        1.0,
        2.5,
        1.0e10,
        f64::MAX,
    ];
    let keys: Vec<FiniteDouble> = ordered
        .iter()
        .map(|v| FiniteDouble::new(*v).unwrap())
        .collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted, "derived Ord must equal numeric order");
    assert!(keys.windows(2).all(|w| w[0] < w[1]));
}

#[test]
fn finite_double_round_trips() {
    for v in [f64::MIN, -3.25, 0.0, 1.5, 6.02e23, f64::MAX] {
        assert_eq!(FiniteDouble::new(v).unwrap().get().to_bits(), v.to_bits());
    }
}

#[test]
fn calendar_dates_validated() {
    assert!(
        CalendarDate::new(2024, 2, 29).is_ok(),
        "2024 is a leap year"
    );
    assert_eq!(
        CalendarDate::new(2026, 2, 29),
        Err(ValueTypeError::InvalidDate)
    );
    assert_eq!(
        CalendarDate::new(1900, 2, 29),
        Err(ValueTypeError::InvalidDate),
        "1900 is not a leap year (century rule)"
    );
    assert!(CalendarDate::new(2000, 2, 29).is_ok(), "400-year rule");
    assert_eq!(
        CalendarDate::new(2026, 4, 31),
        Err(ValueTypeError::InvalidDate)
    );
    assert_eq!(
        CalendarDate::new(2026, 13, 1),
        Err(ValueTypeError::InvalidDate)
    );
    assert_eq!(
        CalendarDate::new(2026, 1, 0),
        Err(ValueTypeError::InvalidDate)
    );
}

#[test]
fn calendar_dates_order_chronologically() {
    let a = CalendarDate::new(2025, 12, 31).unwrap();
    let b = CalendarDate::new(2026, 1, 1).unwrap();
    let c = CalendarDate::new(2026, 8, 30).unwrap();
    assert!(a < b && b < c);
}

#[test]
fn storage_classes_are_the_ruled_mapping() {
    let cases: Vec<(PropertyValue, StorageClass)> = vec![
        (PropertyValue::Integer(7), StorageClass::Integer),
        (PropertyValue::Boolean(true), StorageClass::Integer),
        (
            PropertyValue::Date(CalendarDate::new(2026, 8, 30).unwrap()),
            StorageClass::Integer,
        ),
        (
            PropertyValue::Timestamp { epoch_millis: 0 },
            StorageClass::Integer,
        ),
        (
            PropertyValue::Double(FiniteDouble::new(1.5).unwrap()),
            StorageClass::Real,
        ),
        (PropertyValue::String("x".into()), StorageClass::Text),
        (PropertyValue::Array(vec![]), StorageClass::Bytes),
        (PropertyValue::Struct(BTreeMap::new()), StorageClass::Bytes),
    ];
    for (value, class) in cases {
        assert_eq!(value.storage_class(), class, "{}", value.type_label());
    }
}

#[test]
fn as_str_answers_only_for_string() {
    assert_eq!(PropertyValue::String("v".into()).as_str(), Some("v"));
    assert_eq!(PropertyValue::Integer(1).as_str(), None);
    assert_eq!(PropertyValue::Array(vec![]).as_str(), None);
}

#[test]
fn type_labels_stable() {
    assert_eq!(PropertyValue::Integer(1).type_label(), "integer");
    assert_eq!(
        PropertyValue::Struct(BTreeMap::new()).type_label(),
        "struct"
    );
}
