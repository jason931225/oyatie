//! The declaration plane of the typed value model: [`ValueTypeDeclaration`]
//! describes what a property or parameter value must be, and
//! [`ValueTypeDeclaration::admits_value`] walks a value in lockstep with its
//! declaration — zero coercion, value depth bounded by declaration depth.

use crate::property::PropertyTier;
use crate::value::{PropertyValue, ValueTypeError};

/// Maximum nesting depth a declaration may reach, enforced at
/// [`ValueTypeDeclaration::validate`]. Conformance recursion is driven by
/// the declaration, so admitted values can never nest deeper.
pub const MAX_VALUE_TYPE_DEPTH: usize = 8;

/// The scalar base types of V1.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScalarType {
    String,
    Integer,
    Double,
    Boolean,
    Date,
    Timestamp,
}

impl ScalarType {
    const fn type_label(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Integer => "integer",
            Self::Double => "double",
            Self::Boolean => "boolean",
            Self::Date => "date",
            Self::Timestamp => "timestamp",
        }
    }
}

/// One declared field of a struct schema.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructFieldDeclaration {
    pub name: String,
    pub value_type: ValueTypeDeclaration,
    pub required: bool,
}

/// A named-field schema for struct values. Non-empty, with unique,
/// non-blank field names — enforced by [`ValueTypeDeclaration::validate`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructSchema {
    pub fields: Vec<StructFieldDeclaration>,
}

/// What a value must be. V1 covers exactly the Scalar/Array/Struct shapes;
/// typed Timeseries/Geo/Ciphertext are reserved loosen-only widenings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValueTypeDeclaration {
    Scalar(ScalarType),
    Array { element: Box<ValueTypeDeclaration> },
    Struct(StructSchema),
}

/// A fail-closed conformance diagnostic: the path to the offending value
/// and static type labels only — never classified values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueTypeViolation {
    /// Dotted/indexed path from the declaration root, `""` at the root.
    pub path: String,
    /// The declared expectation at that path (`"absent"` for a field the
    /// schema does not declare).
    pub expected: &'static str,
    /// What the value actually carried (`"absent"` for a missing required
    /// field).
    pub found: &'static str,
}

impl ValueTypeDeclaration {
    /// The total projection onto the tier taxonomy: a declaration's tier is
    /// derived, never stated, so tier/type incoherence is unrepresentable.
    pub const fn tier(&self) -> PropertyTier {
        match self {
            Self::Scalar(_) => PropertyTier::Scalar,
            Self::Array { .. } => PropertyTier::Vector,
            Self::Struct(_) => PropertyTier::Struct,
        }
    }

    /// Static label of this declaration, the diagnostic vocabulary shared
    /// with [`PropertyValue::type_label`].
    pub const fn type_label(&self) -> &'static str {
        match self {
            Self::Scalar(scalar) => scalar.type_label(),
            Self::Array { .. } => "array",
            Self::Struct(_) => "struct",
        }
    }

    /// Structural well-formedness: depth at most [`MAX_VALUE_TYPE_DEPTH`],
    /// struct schemas non-empty with unique non-blank field names.
    pub fn validate(&self) -> Result<(), ValueTypeError> {
        self.validate_at_depth(1)
    }

    fn validate_at_depth(&self, depth: usize) -> Result<(), ValueTypeError> {
        if depth > MAX_VALUE_TYPE_DEPTH {
            return Err(ValueTypeError::DepthExceeded);
        }
        match self {
            Self::Scalar(_) => Ok(()),
            Self::Array { element } => element.validate_at_depth(depth + 1),
            Self::Struct(schema) => {
                if schema.fields.is_empty() {
                    return Err(ValueTypeError::EmptyStructSchema);
                }
                let mut seen = std::collections::BTreeSet::new();
                for field in &schema.fields {
                    if field.name.trim().is_empty() {
                        return Err(ValueTypeError::BlankStructFieldName);
                    }
                    if !seen.insert(field.name.as_str()) {
                        return Err(ValueTypeError::DuplicateStructField {
                            name: field.name.clone(),
                        });
                    }
                    field.value_type.validate_at_depth(depth + 1)?;
                }
                Ok(())
            }
        }
    }

    /// Walk `value` in lockstep with this declaration. Zero coercion: every
    /// mismatch is a [`ValueTypeViolation`] carrying the path and the two
    /// static labels.
    pub fn admits_value(&self, value: &PropertyValue) -> Result<(), ValueTypeViolation> {
        self.admits_at_path(value, String::new())
    }

    fn admits_at_path(
        &self,
        value: &PropertyValue,
        path: String,
    ) -> Result<(), ValueTypeViolation> {
        let mismatch = |expected: &'static str| ValueTypeViolation {
            path: path.clone(),
            expected,
            found: value.type_label(),
        };
        match self {
            Self::Scalar(scalar) => {
                let admitted = matches!(
                    (scalar, value),
                    (ScalarType::String, PropertyValue::String(_))
                        | (ScalarType::Integer, PropertyValue::Integer(_))
                        | (ScalarType::Double, PropertyValue::Double(_))
                        | (ScalarType::Boolean, PropertyValue::Boolean(_))
                        | (ScalarType::Date, PropertyValue::Date(_))
                        | (ScalarType::Timestamp, PropertyValue::Timestamp { .. })
                );
                if admitted {
                    Ok(())
                } else {
                    Err(mismatch(scalar.type_label()))
                }
            }
            Self::Array { element } => match value {
                PropertyValue::Array(items) => {
                    for (index, item) in items.iter().enumerate() {
                        element.admits_at_path(item, format!("{path}[{index}]"))?;
                    }
                    Ok(())
                }
                _ => Err(mismatch("array")),
            },
            Self::Struct(schema) => match value {
                PropertyValue::Struct(entries) => {
                    for field in &schema.fields {
                        match entries.get(&field.name) {
                            Some(entry) => {
                                field
                                    .value_type
                                    .admits_at_path(entry, join_path(&path, &field.name))?;
                            }
                            None if field.required => {
                                return Err(ValueTypeViolation {
                                    path: join_path(&path, &field.name),
                                    expected: field.value_type.type_label(),
                                    found: "absent",
                                });
                            }
                            None => {}
                        }
                    }
                    for name in entries.keys() {
                        if !schema.fields.iter().any(|f| &f.name == name) {
                            return Err(ValueTypeViolation {
                                path: join_path(&path, name),
                                expected: "absent",
                                found: entries[name].type_label(),
                            });
                        }
                    }
                    Ok(())
                }
                _ => Err(mismatch("struct")),
            },
        }
    }
}

impl crate::definitions::EntityTypePropertyDefinition {
    /// A typed property definition: the tier is DERIVED from the
    /// declaration's projection, so tier/type incoherence is
    /// unrepresentable here.
    pub fn typed(
        name: impl Into<String>,
        value_type: ValueTypeDeclaration,
        data_class: data_boundary_kernel::PrivacyDataClass,
        required: bool,
    ) -> Result<Self, crate::error::OntologyEngineError> {
        let tier = value_type.tier();
        let mut property = Self::new(name, tier, data_class, required)?;
        property.value_type = Some(value_type);
        Ok(property)
    }
}

fn join_path(path: &str, name: &str) -> String {
    if path.is_empty() {
        name.to_string()
    } else {
        format!("{path}.{name}")
    }
}
