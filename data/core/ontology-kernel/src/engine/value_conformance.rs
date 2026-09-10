//! The shared value-conformance station: the FINAL per-property and
//! per-parameter step of both conformance checks.
//!
//! `Some(declaration)` walks the carrier in declaration lockstep with zero
//! coercion; `None` requires the legacy [`PropertyValue::String`] carrier —
//! byte-identical to what every bridge constructor produces, closing the
//! typed-value-under-untyped-declaration loophole.

use crate::error::OntologyEngineError;
use crate::property::ObjectProperty;
use crate::value::PropertyValue;
use crate::value_type::{ValueTypeDeclaration, ValueTypeViolation};

/// Which conformance surface is asking — selects the error variant.
pub(crate) enum ValueCheckSubject {
    Property,
    Parameter,
}

pub(crate) fn check_declared_value(
    declared: Option<&ValueTypeDeclaration>,
    carrier: &ObjectProperty,
    subject: ValueCheckSubject,
) -> Result<(), OntologyEngineError> {
    let violation = match declared {
        Some(declaration) => declaration.admits_value(&carrier.value.value).err(),
        None => match &carrier.value.value {
            PropertyValue::String(_) => None,
            other => Some(ValueTypeViolation {
                path: String::new(),
                expected: "string",
                found: other.type_label(),
            }),
        },
    };
    let Some(violation) = violation else {
        return Ok(());
    };
    Err(match subject {
        ValueCheckSubject::Property => OntologyEngineError::PropertyValueTypeMismatch {
            name: carrier.name.clone(),
            path: violation.path,
            expected: violation.expected,
            found: violation.found,
        },
        ValueCheckSubject::Parameter => OntologyEngineError::ParameterValueTypeMismatch {
            name: carrier.name.clone(),
            path: violation.path,
            expected: violation.expected,
            found: violation.found,
        },
    })
}
