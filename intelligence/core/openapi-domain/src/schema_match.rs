use crate::error::OpenApiSourceError;
use crate::schema_shape::{
    ExpectedOpenApiScalar, OpenApiSchemaProperty, OpenApiSchemaShape, RustStructShape,
};
use std::collections::BTreeSet;

pub(crate) fn validate_schema_fields_match(
    schema: &OpenApiSchemaShape,
    runtime: &RustStructShape,
) -> Result<(), OpenApiSourceError> {
    let schema_fields = schema.properties.keys().cloned().collect::<BTreeSet<_>>();
    let runtime_fields = runtime.fields.keys().cloned().collect::<BTreeSet<_>>();
    let missing_properties = runtime
        .fields
        .keys()
        .filter(|field| !schema_fields.contains(*field))
        .cloned()
        .collect::<Vec<_>>();
    let extra_properties = schema
        .properties
        .keys()
        .filter(|field| !runtime_fields.contains(*field))
        .cloned()
        .collect::<Vec<_>>();
    if !missing_properties.is_empty() || !extra_properties.is_empty() {
        return Err(OpenApiSourceError::SchemaFieldMismatch {
            schema_name: schema.schema_name.clone(),
            contract_path: schema.contract_path.clone(),
            missing_properties,
            extra_properties,
        });
    }

    let missing_required = runtime
        .required_fields
        .difference(&schema.required)
        .cloned()
        .collect::<Vec<_>>();
    let extra_required = schema
        .required
        .difference(&runtime.required_fields)
        .cloned()
        .collect::<Vec<_>>();
    if !missing_required.is_empty() || !extra_required.is_empty() {
        return Err(OpenApiSourceError::SchemaRequiredMismatch {
            schema_name: schema.schema_name.clone(),
            contract_path: schema.contract_path.clone(),
            missing_required,
            extra_required,
        });
    }
    Ok(())
}

pub(crate) fn validate_schema_types_match(
    schema: &OpenApiSchemaShape,
    runtime: &RustStructShape,
) -> Result<(), OpenApiSourceError> {
    let mut mismatches = schema
        .unsupported_keywords
        .iter()
        .map(|keyword| {
            format!(
                "{}: unsupported OpenAPI schema keyword {keyword}",
                schema.schema_name
            )
        })
        .collect::<Vec<_>>();
    for (field_name, runtime_field) in &runtime.fields {
        let Some(property) = schema.properties.get(field_name) else {
            continue;
        };
        let expected_rust_type = &runtime_field.rust_type;
        match &property.rust_type {
            Some(actual) if actual == expected_rust_type => {}
            Some(actual) => mismatches.push(format!(
                "{field_name}: expected x-oyatie-rust-type {expected_rust_type}, found {actual}"
            )),
            None => mismatches.push(format!(
                "{field_name}: missing x-oyatie-rust-type, expected {expected_rust_type}"
            )),
        }

        validate_schema_property_type(field_name, expected_rust_type, property, &mut mismatches);
    }
    if !mismatches.is_empty() {
        return Err(OpenApiSourceError::SchemaTypeMismatch {
            schema_name: schema.schema_name.clone(),
            contract_path: schema.contract_path.clone(),
            mismatches,
        });
    }
    Ok(())
}

fn validate_schema_property_type(
    field_name: &str,
    expected_rust_type: &str,
    property: &OpenApiSchemaProperty,
    mismatches: &mut Vec<String>,
) {
    for keyword in &property.unsupported_keywords {
        mismatches.push(format!(
            "{field_name}: unsupported OpenAPI schema keyword {keyword}"
        ));
    }

    if property.nullable && strip_option_type(expected_rust_type).is_none() {
        mismatches.push(format!(
            "{field_name}: OpenAPI nullable true requires Rust Option<...>, found {expected_rust_type}"
        ));
    }

    let unwrapped_rust_type = strip_option_type(expected_rust_type).unwrap_or(expected_rust_type);
    if let Some(item_type) = strip_vec_type(unwrapped_rust_type) {
        let actual_type = property.type_name.as_deref().unwrap_or("<missing>");
        if actual_type != "array" {
            mismatches.push(format!(
                "{field_name}: expected type array for Rust {expected_rust_type}, found type {actual_type}"
            ));
        }
        let item_scalar = expected_openapi_scalar(item_type);
        if item_scalar.type_name != "<unsupported-rust-scalar>" {
            if let Some(actual) = property.items_ref_schema.as_deref() {
                mismatches.push(format!(
                    "{field_name}: scalar array items for Rust {expected_rust_type} must not declare items $ref {actual}"
                ));
                return;
            }
            let actual_item_type = property.items_type_name.as_deref().unwrap_or("<missing>");
            let actual_item_format = property.items_format.as_deref().unwrap_or("<none>");
            let expected_item_format = item_scalar.format.unwrap_or("<none>");
            if actual_item_type != item_scalar.type_name
                || actual_item_format != expected_item_format
            {
                mismatches.push(format!(
                    "{field_name}: expected items type {} format {} for Rust {expected_rust_type}, found items type {} format {}",
                    item_scalar.type_name, expected_item_format, actual_item_type, actual_item_format
                ));
            }
            return;
        }
        match property.items_ref_schema.as_deref() {
            Some(actual) if actual == item_type => {}
            Some(actual) => mismatches.push(format!(
                "{field_name}: expected items $ref {item_type} for Rust {expected_rust_type}, found {actual}"
            )),
            None => mismatches.push(format!(
                "{field_name}: missing items $ref, expected {item_type} for Rust {expected_rust_type}"
            )),
        }
        return;
    }

    let scalar = expected_openapi_scalar(unwrapped_rust_type);
    if scalar.type_name != "<unsupported-rust-scalar>" {
        let actual_type = property.type_name.as_deref().unwrap_or("<missing>");
        let actual_format = property.format.as_deref().unwrap_or("<none>");
        let expected_format = scalar.format.unwrap_or("<none>");
        if actual_type != scalar.type_name || actual_format != expected_format {
            mismatches.push(format!(
                "{field_name}: expected type {} format {} for Rust {}, found type {} format {}",
                scalar.type_name, expected_format, expected_rust_type, actual_type, actual_format
            ));
        }
        return;
    }

    match property.ref_schema.as_deref() {
        Some(actual) if actual == unwrapped_rust_type => {
            if let Some(actual_type) = property.type_name.as_deref() {
                mismatches.push(format!(
                    "{field_name}: $ref property for Rust {expected_rust_type} must not declare type {actual_type}"
                ));
            }
            if let Some(actual_format) = property.format.as_deref() {
                mismatches.push(format!(
                    "{field_name}: $ref property for Rust {expected_rust_type} must not declare format {actual_format}"
                ));
            }
        }
        Some(actual) => mismatches.push(format!(
            "{field_name}: expected $ref {unwrapped_rust_type} for Rust {expected_rust_type}, found {actual}"
        )),
        None => mismatches.push(format!(
            "{field_name}: missing $ref, expected {unwrapped_rust_type} for Rust {expected_rust_type}"
        )),
    }
}

fn expected_openapi_scalar(rust_type: &str) -> ExpectedOpenApiScalar {
    let scalar_type = strip_option_type(rust_type).unwrap_or(rust_type);
    match scalar_type {
        "String" | "Purpose" | "SubjectClass" | "BudgetWarning" => ExpectedOpenApiScalar {
            type_name: "string",
            format: None,
        },
        "bool" => ExpectedOpenApiScalar {
            type_name: "boolean",
            format: None,
        },
        "u8" => ExpectedOpenApiScalar {
            type_name: "integer",
            format: Some("uint8"),
        },
        "u16" => ExpectedOpenApiScalar {
            type_name: "integer",
            format: Some("uint16"),
        },
        "u32" => ExpectedOpenApiScalar {
            type_name: "integer",
            format: Some("uint32"),
        },
        "u64" => ExpectedOpenApiScalar {
            type_name: "integer",
            format: Some("uint64"),
        },
        "i8" => ExpectedOpenApiScalar {
            type_name: "integer",
            format: Some("int8"),
        },
        "i16" => ExpectedOpenApiScalar {
            type_name: "integer",
            format: Some("int16"),
        },
        "i32" => ExpectedOpenApiScalar {
            type_name: "integer",
            format: Some("int32"),
        },
        "i64" => ExpectedOpenApiScalar {
            type_name: "integer",
            format: Some("int64"),
        },
        "f32" => ExpectedOpenApiScalar {
            type_name: "number",
            format: Some("float"),
        },
        "f64" => ExpectedOpenApiScalar {
            type_name: "number",
            format: Some("double"),
        },
        _ => ExpectedOpenApiScalar {
            type_name: "<unsupported-rust-scalar>",
            format: None,
        },
    }
}

pub(crate) fn strip_option_type(rust_type: &str) -> Option<&str> {
    rust_type
        .strip_prefix("Option<")
        .and_then(|inner| inner.strip_suffix('>'))
}

fn strip_vec_type(rust_type: &str) -> Option<&str> {
    rust_type
        .strip_prefix("Vec<")
        .and_then(|inner| inner.strip_suffix('>'))
}

pub(crate) fn canonical_rust_type(rust_type: &str) -> String {
    rust_type
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}
