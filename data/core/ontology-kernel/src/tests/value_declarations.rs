//! Pins: the tier projection is total, validation rejects malformed
//! schemas and over-deep nesting, and admits_value is a zero-coercion
//! lockstep walk with honest paths.

use std::collections::BTreeMap;

use crate::*;

fn scalar(t: ScalarType) -> ValueTypeDeclaration {
    ValueTypeDeclaration::Scalar(t)
}

fn array_of(element: ValueTypeDeclaration) -> ValueTypeDeclaration {
    ValueTypeDeclaration::Array {
        element: Box::new(element),
    }
}

fn field(name: &str, value_type: ValueTypeDeclaration, required: bool) -> StructFieldDeclaration {
    StructFieldDeclaration {
        name: name.into(),
        value_type,
        required,
    }
}

fn struct_of(fields: Vec<StructFieldDeclaration>) -> ValueTypeDeclaration {
    ValueTypeDeclaration::Struct(StructSchema { fields })
}

#[test]
fn tier_projection_total() {
    assert_eq!(scalar(ScalarType::Integer).tier(), PropertyTier::Scalar);
    assert_eq!(
        array_of(scalar(ScalarType::String)).tier(),
        PropertyTier::Vector
    );
    assert_eq!(
        struct_of(vec![field("a", scalar(ScalarType::Boolean), true)]).tier(),
        PropertyTier::Struct
    );
}

#[test]
fn depth_ceiling_enforced() {
    let mut decl = scalar(ScalarType::Integer); // depth 1
    for _ in 0..7 {
        decl = array_of(decl); // depths 2..=8
    }
    assert_eq!(decl.validate(), Ok(()), "depth 8 must validate");
    assert_eq!(
        array_of(decl).validate(),
        Err(ValueTypeError::DepthExceeded),
        "depth 9 must be rejected"
    );
}

#[test]
fn struct_schema_well_formedness() {
    assert_eq!(
        struct_of(vec![]).validate(),
        Err(ValueTypeError::EmptyStructSchema)
    );
    assert_eq!(
        struct_of(vec![field("  ", scalar(ScalarType::String), true)]).validate(),
        Err(ValueTypeError::BlankStructFieldName)
    );
    assert_eq!(
        struct_of(vec![
            field("x", scalar(ScalarType::String), true),
            field("x", scalar(ScalarType::Integer), false),
        ])
        .validate(),
        Err(ValueTypeError::DuplicateStructField { name: "x".into() })
    );
    // Nested malformation is found through the walk.
    assert_eq!(
        struct_of(vec![field("inner", struct_of(vec![]), true)]).validate(),
        Err(ValueTypeError::EmptyStructSchema)
    );
}

#[test]
fn scalar_admission_is_exact() {
    let date = PropertyValue::Date(CalendarDate::new(2026, 8, 30).unwrap());
    let cases: Vec<(ScalarType, PropertyValue)> = vec![
        (ScalarType::String, PropertyValue::String("s".into())),
        (ScalarType::Integer, PropertyValue::Integer(1)),
        (
            ScalarType::Double,
            PropertyValue::Double(FiniteDouble::new(1.5).unwrap()),
        ),
        (ScalarType::Boolean, PropertyValue::Boolean(true)),
        (ScalarType::Date, date.clone()),
        (
            ScalarType::Timestamp,
            PropertyValue::Timestamp { epoch_millis: 0 },
        ),
    ];
    for (declared, value) in &cases {
        assert_eq!(scalar(*declared).admits_value(value), Ok(()));
    }
    // Integer does not coerce to double, nor string to anything.
    let violation = scalar(ScalarType::Double)
        .admits_value(&PropertyValue::Integer(1))
        .unwrap_err();
    assert_eq!(violation.expected, "double");
    assert_eq!(violation.found, "integer");
    assert_eq!(violation.path, "");
}

#[test]
fn array_elements_checked_with_paths() {
    let decl = array_of(scalar(ScalarType::Integer));
    assert_eq!(
        decl.admits_value(&PropertyValue::Array(vec![
            PropertyValue::Integer(1),
            PropertyValue::Integer(2),
        ])),
        Ok(())
    );
    let violation = decl
        .admits_value(&PropertyValue::Array(vec![
            PropertyValue::Integer(1),
            PropertyValue::String("two".into()),
        ]))
        .unwrap_err();
    assert_eq!(violation.path, "[1]");
    assert_eq!(violation.expected, "integer");
    assert_eq!(violation.found, "string");
}

#[test]
fn struct_admission_fail_closed() {
    let decl = struct_of(vec![
        field("id", scalar(ScalarType::Integer), true),
        field("tags", array_of(scalar(ScalarType::String)), false),
    ]);

    let mut ok = BTreeMap::new();
    ok.insert("id".to_string(), PropertyValue::Integer(7));
    assert_eq!(
        decl.admits_value(&PropertyValue::Struct(ok.clone())),
        Ok(())
    );

    // Missing required.
    let missing = decl
        .admits_value(&PropertyValue::Struct(BTreeMap::new()))
        .unwrap_err();
    assert_eq!(missing.path, "id");
    assert_eq!(missing.found, "absent");

    // Undeclared field.
    let mut extra = ok.clone();
    extra.insert("ghost".to_string(), PropertyValue::Boolean(true));
    let undeclared = decl
        .admits_value(&PropertyValue::Struct(extra))
        .unwrap_err();
    assert_eq!(undeclared.path, "ghost");
    assert_eq!(undeclared.expected, "absent");
    assert_eq!(undeclared.found, "boolean");

    // Nested path through struct-in-array.
    let mut nested = ok;
    nested.insert(
        "tags".to_string(),
        PropertyValue::Array(vec![
            PropertyValue::String("fine".into()),
            PropertyValue::Integer(3),
        ]),
    );
    let deep = decl
        .admits_value(&PropertyValue::Struct(nested))
        .unwrap_err();
    assert_eq!(deep.path, "tags[1]");
    assert_eq!(deep.expected, "string");
    assert_eq!(deep.found, "integer");
}

#[test]
fn value_depth_bounded_by_declaration() {
    let decl = array_of(scalar(ScalarType::Integer)); // depth 2
    let too_deep =
        PropertyValue::Array(vec![PropertyValue::Array(vec![PropertyValue::Integer(1)])]);
    let violation = decl.admits_value(&too_deep).unwrap_err();
    assert_eq!(violation.path, "[0]");
    assert_eq!(violation.expected, "integer");
    assert_eq!(violation.found, "array");
}
