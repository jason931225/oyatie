use crate::document::{OpenApiDocument, document_map, validate_document, validate_document_path};
use crate::error::OpenApiSourceError;
use crate::runtime_parity::{OpenApiRuntimeSource, path_lives_in_crate, runtime_source_map};
use crate::rust_struct_shape::parse_rust_struct_fields;
use crate::schema_match::{validate_schema_fields_match, validate_schema_types_match};
use crate::schema_shape::collect_component_schemas;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenApiSchemaBinding {
    pub schema_name: String,   // data_class: INTERNAL_ONLY
    pub contract_path: String, // data_class: INTERNAL_ONLY
    pub runtime_crate: String, // data_class: INTERNAL_ONLY
    pub source_path: String,   // data_class: INTERNAL_ONLY
    pub rust_struct: String,   // data_class: INTERNAL_ONLY
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OpenApiSchemaParityReport {
    pub schemas_checked: usize,  // data_class: INTERNAL_ONLY
    pub bindings_checked: usize, // data_class: INTERNAL_ONLY
    pub sources_checked: usize,  // data_class: INTERNAL_ONLY
    pub fields_checked: usize,   // data_class: INTERNAL_ONLY
    pub types_checked: usize,    // data_class: INTERNAL_ONLY
}

pub fn validate_openapi_schema_parity<D, B, S>(
    documents: D,
    bindings: B,
    runtime_sources: S,
) -> Result<OpenApiSchemaParityReport, OpenApiSourceError>
where
    D: IntoIterator<Item = OpenApiDocument>,
    B: IntoIterator<Item = OpenApiSchemaBinding>,
    S: IntoIterator<Item = OpenApiRuntimeSource>,
{
    let documents = document_map(documents)?;
    let mut schemas = BTreeMap::new();
    for (path, contents) in &documents {
        validate_document(path, contents)?;
        for schema in collect_component_schemas(path, contents)? {
            schemas.insert(
                (schema.contract_path.clone(), schema.schema_name.clone()),
                schema,
            );
        }
    }

    let bindings = schema_binding_map(bindings)?;
    let sources = runtime_source_map(runtime_sources, true)?;
    for schema in schemas.values() {
        let key = (schema.contract_path.clone(), schema.schema_name.clone());
        if !bindings.contains_key(&key) {
            return Err(OpenApiSourceError::MissingSchemaBinding {
                schema_name: schema.schema_name.clone(),
                contract_path: schema.contract_path.clone(),
            });
        }
    }

    let mut fields_checked = 0usize;
    let mut types_checked = 0usize;
    for binding in bindings.values() {
        let key = (binding.contract_path.clone(), binding.schema_name.clone());
        let Some(schema) = schemas.get(&key) else {
            return Err(OpenApiSourceError::StaleSchemaBinding {
                schema_name: binding.schema_name.clone(),
                contract_path: binding.contract_path.clone(),
            });
        };
        let source = sources.get(&binding.source_path).ok_or_else(|| {
            OpenApiSourceError::MissingSchemaRuntimeSource {
                schema_name: binding.schema_name.clone(),
                path: binding.source_path.clone(),
            }
        })?;
        let runtime = parse_rust_struct_fields(source, &binding.rust_struct).map_err(|reason| {
            OpenApiSourceError::InvalidRuntimeStruct {
                schema_name: binding.schema_name.clone(),
                path: binding.source_path.clone(),
                rust_struct: binding.rust_struct.clone(),
                reason,
            }
        })?;
        let runtime = runtime.ok_or_else(|| OpenApiSourceError::MissingRuntimeStruct {
            schema_name: binding.schema_name.clone(),
            path: binding.source_path.clone(),
            rust_struct: binding.rust_struct.clone(),
        })?;
        validate_schema_fields_match(schema, &runtime)?;
        fields_checked += schema.properties.len();
        validate_schema_types_match(schema, &runtime)?;
        types_checked += schema.properties.len();
    }

    Ok(OpenApiSchemaParityReport {
        schemas_checked: schemas.len(),
        bindings_checked: bindings.len(),
        sources_checked: sources.len(),
        fields_checked,
        types_checked,
    })
}

fn schema_binding_map<I>(
    bindings: I,
) -> Result<BTreeMap<(String, String), OpenApiSchemaBinding>, OpenApiSourceError>
where
    I: IntoIterator<Item = OpenApiSchemaBinding>,
{
    let mut map = BTreeMap::new();
    for binding in bindings {
        validate_schema_binding_shape(&binding)?;
        let key = (binding.contract_path.clone(), binding.schema_name.clone());
        if map.insert(key.clone(), binding).is_some() {
            return Err(OpenApiSourceError::DuplicateSchemaBinding {
                contract_path: key.0,
                schema_name: key.1,
            });
        }
    }
    Ok(map)
}

fn validate_schema_binding_shape(binding: &OpenApiSchemaBinding) -> Result<(), OpenApiSourceError> {
    require_schema_non_empty(&binding.schema_name, "schema_name", &binding.schema_name)?;
    validate_document_path(&binding.contract_path)?;
    require_schema_non_empty(
        &binding.runtime_crate,
        "runtime_crate",
        &binding.schema_name,
    )?;
    validate_relative_schema_path(&binding.source_path, "source_path", &binding.schema_name)?;
    require_schema_non_empty(&binding.rust_struct, "rust_struct", &binding.schema_name)?;

    if !path_lives_in_crate(&binding.source_path, &binding.runtime_crate) {
        return Err(OpenApiSourceError::InvalidSchemaBinding {
            schema_name: binding.schema_name.clone(),
            field: "source_path",
            reason: format!(
                "source path must live in the crate dir of `{}` (legacy `crates/<crate>/` or canonical `<capability>/<face>/<crate>/` layout)",
                binding.runtime_crate
            ),
        });
    }
    Ok(())
}

fn require_schema_non_empty(
    value: &str,
    field: &'static str,
    schema_name: &str,
) -> Result<(), OpenApiSourceError> {
    if value.trim().is_empty() {
        return Err(OpenApiSourceError::InvalidSchemaBinding {
            schema_name: schema_name.into(),
            field,
            reason: "field must be non-empty".into(),
        });
    }
    Ok(())
}

fn validate_relative_schema_path(
    path: &str,
    field: &'static str,
    schema_name: &str,
) -> Result<(), OpenApiSourceError> {
    if path.trim().is_empty() {
        return Err(OpenApiSourceError::InvalidSchemaBinding {
            schema_name: schema_name.into(),
            field,
            reason: "path must be non-empty".into(),
        });
    }
    if path.starts_with('/') || path.contains('\\') || path.contains('\0') {
        return Err(OpenApiSourceError::InvalidSchemaBinding {
            schema_name: schema_name.into(),
            field,
            reason: "path must be a relative slash path".into(),
        });
    }
    if path
        .split('/')
        .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(OpenApiSourceError::InvalidSchemaBinding {
            schema_name: schema_name.into(),
            field,
            reason: "path must not contain empty, dot, or parent components".into(),
        });
    }
    Ok(())
}
