use crate::document::{OpenApiDocument, document_map, validate_document, validate_document_path};
use crate::error::OpenApiSourceError;
use crate::operation::collect_document_operations;
use crate::path_validation::valid_numeric_response_status;
use crate::runtime_binding_source::validate_runtime_binding_source;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenApiRuntimeBinding {
    pub operation_id: String,                       // data_class: INTERNAL_ONLY
    pub contract_path: String,                      // data_class: INTERNAL_ONLY
    pub runtime_crate: String,                      // data_class: INTERNAL_ONLY
    pub source_path: String,                        // data_class: INTERNAL_ONLY
    pub symbol: String,                             // data_class: INTERNAL_ONLY
    pub status_type: String,                        // data_class: INTERNAL_ONLY
    pub evidence_surface: String,                   // data_class: INTERNAL_ONLY
    pub test_path: String,                          // data_class: INTERNAL_ONLY
    pub response_schemas: BTreeMap<String, String>, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenApiRuntimeSource {
    pub path: String,     // data_class: INTERNAL_ONLY
    pub contents: String, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OpenApiRuntimeParityReport {
    pub operations_checked: usize,        // data_class: INTERNAL_ONLY
    pub bindings_checked: usize,          // data_class: INTERNAL_ONLY
    pub sources_checked: usize,           // data_class: INTERNAL_ONLY
    pub tests_checked: usize,             // data_class: INTERNAL_ONLY
    pub response_statuses_checked: usize, // data_class: INTERNAL_ONLY
    pub response_schemas_checked: usize,  // data_class: INTERNAL_ONLY
}

pub fn validate_openapi_runtime_parity<D, B, S, T>(
    documents: D,
    bindings: B,
    runtime_sources: S,
    runtime_tests: T,
) -> Result<OpenApiRuntimeParityReport, OpenApiSourceError>
where
    D: IntoIterator<Item = OpenApiDocument>,
    B: IntoIterator<Item = OpenApiRuntimeBinding>,
    S: IntoIterator<Item = OpenApiRuntimeSource>,
    T: IntoIterator<Item = OpenApiRuntimeSource>,
{
    let documents = document_map(documents)?;
    let mut operations = BTreeMap::new();
    for (path, contents) in &documents {
        validate_document(path, contents)?;
        for operation in collect_document_operations(path, contents)? {
            if let Some(first) =
                operations.insert(operation.operation_id.clone(), operation.clone())
            {
                return Err(OpenApiSourceError::DuplicateOperationId {
                    operation_id: operation.operation_id,
                    first_path: first.contract_path,
                    second_path: operation.contract_path,
                });
            }
        }
    }

    let bindings = runtime_binding_map(bindings)?;
    let sources = runtime_source_map(runtime_sources, true)?;
    let tests = runtime_source_map(runtime_tests, false)?;

    for operation in operations.values() {
        if !bindings.contains_key(&operation.operation_id) {
            return Err(OpenApiSourceError::MissingRuntimeBinding {
                operation_id: operation.operation_id.clone(),
                contract_path: operation.contract_path.clone(),
            });
        }
    }

    let mut response_statuses_checked = 0usize;
    let mut response_schemas_checked = 0usize;
    for binding in bindings.values() {
        let Some(operation) = operations.get(&binding.operation_id) else {
            return Err(OpenApiSourceError::StaleRuntimeBinding {
                operation_id: binding.operation_id.clone(),
                contract_path: binding.contract_path.clone(),
            });
        };
        if operation.contract_path != binding.contract_path {
            return Err(OpenApiSourceError::StaleRuntimeBinding {
                operation_id: binding.operation_id.clone(),
                contract_path: binding.contract_path.clone(),
            });
        }
        let report = validate_runtime_binding_source(binding, operation, &sources, &tests)?;
        response_statuses_checked += report.response_statuses_checked;
        response_schemas_checked += report.response_schemas_checked;
    }

    Ok(OpenApiRuntimeParityReport {
        operations_checked: operations.len(),
        bindings_checked: bindings.len(),
        sources_checked: sources.len(),
        tests_checked: tests.len(),
        response_statuses_checked,
        response_schemas_checked,
    })
}

fn runtime_binding_map<I>(
    bindings: I,
) -> Result<BTreeMap<String, OpenApiRuntimeBinding>, OpenApiSourceError>
where
    I: IntoIterator<Item = OpenApiRuntimeBinding>,
{
    let mut map = BTreeMap::new();
    for binding in bindings {
        validate_runtime_binding_shape(&binding)?;
        let operation_id = binding.operation_id.clone();
        if map.insert(operation_id.clone(), binding).is_some() {
            return Err(OpenApiSourceError::DuplicateRuntimeBinding { operation_id });
        }
    }
    Ok(map)
}

pub(crate) fn runtime_source_map<I>(
    sources: I,
    source_file: bool,
) -> Result<BTreeMap<String, String>, OpenApiSourceError>
where
    I: IntoIterator<Item = OpenApiRuntimeSource>,
{
    let mut map = BTreeMap::new();
    for source in sources {
        validate_relative_runtime_path(&source.path, "runtime artifact path", "<runtime-source>")?;
        if map.insert(source.path.clone(), source.contents).is_some() {
            if source_file {
                return Err(OpenApiSourceError::DuplicateRuntimeSource { path: source.path });
            }
            return Err(OpenApiSourceError::DuplicateRuntimeTest { path: source.path });
        }
    }
    Ok(map)
}

/// Whether a binding `source_path`/`test_path` lives in the crate directory of
/// `runtime_crate`. Two layouts are accepted:
/// - the legacy flat layout `crates/<crate>/...`;
/// - the canonical capability-face layout `<capability>/<face>/<crate>/...`
///   (ADR-0562), where the package name is `<capability>-<crate>` — e.g.
///   `intelligence/core/api` hosts the `intelligence-api` package.
///
/// The canonical check is shape-only (path component arithmetic); it does not
/// read the crate manifest, keeping this kernel filesystem-free.
pub(crate) fn path_lives_in_crate(path: &str, runtime_crate: &str) -> bool {
    if let Some(rest) = path.strip_prefix("crates/") {
        return rest.starts_with(&format!("{runtime_crate}/"));
    }
    // Canonical: <capability>/<face>/<crate>/... where `<capability>-<crate>`
    // is the package name. The crate dir is the third path component.
    let mut parts = path.split('/');
    let (Some(capability), Some(_face), Some(crate_dir)) =
        (parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    format!("{capability}-{crate_dir}") == runtime_crate
}

fn validate_runtime_binding_shape(
    binding: &OpenApiRuntimeBinding,
) -> Result<(), OpenApiSourceError> {
    require_non_empty(&binding.operation_id, "operation_id", &binding.operation_id)?;
    validate_document_path(&binding.contract_path)?;
    require_non_empty(
        &binding.runtime_crate,
        "runtime_crate",
        &binding.operation_id,
    )?;
    validate_relative_runtime_path(&binding.source_path, "source_path", &binding.operation_id)?;
    validate_relative_runtime_path(&binding.test_path, "test_path", &binding.operation_id)?;
    require_non_empty(&binding.symbol, "symbol", &binding.operation_id)?;
    require_non_empty(&binding.status_type, "status_type", &binding.operation_id)?;
    require_non_empty(
        &binding.evidence_surface,
        "evidence_surface",
        &binding.operation_id,
    )?;
    if binding.response_schemas.is_empty() {
        return Err(OpenApiSourceError::InvalidRuntimeBinding {
            operation_id: binding.operation_id.clone(),
            field: "response_schemas",
            reason: "field must contain status=schema pairs".into(),
        });
    }
    for (status, schema_name) in &binding.response_schemas {
        if !valid_numeric_response_status(status) {
            return Err(OpenApiSourceError::InvalidRuntimeBinding {
                operation_id: binding.operation_id.clone(),
                field: "response_schemas",
                reason: format!("response status {status} must be an explicit numeric status"),
            });
        }
        require_non_empty(schema_name, "response_schemas", &binding.operation_id)?;
    }

    if !path_lives_in_crate(&binding.source_path, &binding.runtime_crate) {
        return Err(OpenApiSourceError::InvalidRuntimeBinding {
            operation_id: binding.operation_id.clone(),
            field: "source_path",
            reason: format!(
                "source path must live in the crate dir of `{}` (legacy `crates/<crate>/` or canonical `<capability>/<face>/<crate>/` layout)",
                binding.runtime_crate
            ),
        });
    }
    if !path_lives_in_crate(&binding.test_path, &binding.runtime_crate) {
        return Err(OpenApiSourceError::InvalidRuntimeBinding {
            operation_id: binding.operation_id.clone(),
            field: "test_path",
            reason: format!(
                "test path must live in the crate dir of `{}` (legacy `crates/<crate>/` or canonical `<capability>/<face>/<crate>/` layout)",
                binding.runtime_crate
            ),
        });
    }
    Ok(())
}

fn require_non_empty(
    value: &str,
    field: &'static str,
    operation_id: &str,
) -> Result<(), OpenApiSourceError> {
    if value.trim().is_empty() {
        return Err(OpenApiSourceError::InvalidRuntimeBinding {
            operation_id: operation_id.into(),
            field,
            reason: "field must be non-empty".into(),
        });
    }
    Ok(())
}

fn validate_relative_runtime_path(
    path: &str,
    field: &'static str,
    operation_id: &str,
) -> Result<(), OpenApiSourceError> {
    if path.trim().is_empty() {
        return Err(OpenApiSourceError::InvalidRuntimeBinding {
            operation_id: operation_id.into(),
            field,
            reason: "path must be non-empty".into(),
        });
    }
    if path.starts_with('/') || path.contains('\\') || path.contains('\0') {
        return Err(OpenApiSourceError::InvalidRuntimeBinding {
            operation_id: operation_id.into(),
            field,
            reason: "path must be a relative slash path".into(),
        });
    }
    if path
        .split('/')
        .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(OpenApiSourceError::InvalidRuntimeBinding {
            operation_id: operation_id.into(),
            field,
            reason: "path must not contain empty, dot, or parent components".into(),
        });
    }
    Ok(())
}
