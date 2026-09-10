//! Typed refusals from transform apply.
//!
//! Every one names WHAT it refused and, where an analysis exists, where that analysis lives. A
//! refusal is a finding; a guess is a defect the receipt would certify as reproducible.

use std::fmt;

use port_engine_api::PortError;

/// Typed refusal from transform apply.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransformError {
    MissingSemantics {
        rule: String,
        field: &'static str,
    },
    Precondition {
        rule: String,
        unit: String,
        precondition: String,
    },
    UnknownConstruction {
        rule: String,
        construction: String,
    },
    ConstructionKindMismatch {
        construction: String,
        kind: String,
        name: String,
    },
    MissingDatum {
        construction: String,
        name: String,
        datum: &'static str,
    },
    UnmappedType {
        unit: String,
        name: String,
        type_ref: String,
    },
    UncapturedDeclaration {
        unit: String,
        name: String,
        kind: String,
    },
    Unsupported {
        name: String,
        detail: String,
    },
    UnitNotInModel {
        unit: String,
    },
    Ownership {
        detail: String,
    },
    Ir(PortError),
}

impl fmt::Display for TransformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSemantics { rule, field } => {
                write!(f, "transform missing `{field}` for rule `{rule}`")
            }
            Self::Precondition {
                rule,
                unit,
                precondition,
            } => write!(
                f,
                "transform precondition `{precondition}` failed for rule `{rule}` unit `{unit}`"
            ),
            Self::UnknownConstruction { rule, construction } => write!(
                f,
                "transform unknown construction `{construction}` for rule `{rule}`"
            ),
            Self::ConstructionKindMismatch {
                construction,
                kind,
                name,
            } => write!(
                f,
                "transform construction `{construction}` cannot build from a `{kind}` declaration \
                 (`{name}`)"
            ),
            Self::MissingDatum {
                construction,
                name,
                datum,
            } => write!(
                f,
                "transform construction `{construction}` needs `{datum}` for declaration `{name}`"
            ),
            Self::UnmappedType {
                unit,
                name,
                type_ref,
            } => write!(
                f,
                "transform cannot resolve type `{type_ref}` for `{name}` in unit `{unit}`: it is \
                 declared nowhere in the unit and the pack's type map does not carry it"
            ),
            Self::UncapturedDeclaration { unit, name, kind } => write!(
                f,
                "transform refuses to drop `{name}`: unit `{unit}` declares it as `{kind}`, no \
                 rule captures that kind, and the pack does not defer it"
            ),
            Self::Unsupported { name, detail } => {
                write!(f, "transform refuses `{name}`: {detail}")
            }
            Self::UnitNotInModel { unit } => {
                write!(
                    f,
                    "transform planned unit `{unit}` is absent from the model"
                )
            }
            Self::Ownership { detail } => write!(f, "transform ownership: {detail}"),
            Self::Ir(err) => write!(f, "transform IR assembly failed: {err}"),
        }
    }
}

impl std::error::Error for TransformError {}
