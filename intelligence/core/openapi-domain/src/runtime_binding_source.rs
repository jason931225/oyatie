use crate::error::OpenApiSourceError;
use crate::operation::OpenApiOperation;
use crate::runtime_parity::OpenApiRuntimeBinding;
use crate::runtime_test_scan::runtime_test_covers_status;
use crate::rust_source::{
    rust_code_contains_identifier, rust_code_contains_public_function,
    rust_code_contains_public_string_const, rust_string_literal_equals,
};
use crate::rust_status_mapping::{
    rust_enum_variants, rust_status_code_match_blocks, status_match_mappings,
};
use std::collections::BTreeMap;
use std::collections::BTreeSet;

pub(crate) fn validate_runtime_binding_source(
    binding: &OpenApiRuntimeBinding,
    operation: &OpenApiOperation,
    sources: &BTreeMap<String, String>,
    tests: &BTreeMap<String, String>,
) -> Result<RuntimeBindingValidationReport, OpenApiSourceError> {
    let source = sources.get(&binding.source_path).ok_or_else(|| {
        OpenApiSourceError::MissingRuntimeSource {
            operation_id: binding.operation_id.clone(),
            path: binding.source_path.clone(),
        }
    })?;
    if !rust_code_contains_public_function(source, &binding.symbol) {
        return Err(OpenApiSourceError::MissingRuntimeSymbol {
            operation_id: binding.operation_id.clone(),
            path: binding.source_path.clone(),
            symbol: binding.symbol.clone(),
        });
    }
    if !rust_code_contains_public_string_const(source, &binding.evidence_surface) {
        return Err(OpenApiSourceError::MissingRuntimeEvidenceSurface {
            operation_id: binding.operation_id.clone(),
            path: binding.source_path.clone(),
            evidence_surface: binding.evidence_surface.clone(),
        });
    }
    if let Some(response_key) = operation.non_explicit_response_keys.iter().next() {
        return Err(OpenApiSourceError::NonExplicitRuntimeResponseKey {
            operation_id: binding.operation_id.clone(),
            contract_path: operation.contract_path.clone(),
            response_key: response_key.clone(),
        });
    }
    for status in &operation.response_statuses {
        let Some(actual_schema) = operation.response_schema_refs.get(status) else {
            return Err(OpenApiSourceError::MissingRuntimeResponseSchema {
                operation_id: binding.operation_id.clone(),
                contract_path: operation.contract_path.clone(),
                status: status.clone(),
            });
        };
        let Some(expected_schema) = binding.response_schemas.get(status) else {
            return Err(OpenApiSourceError::RuntimeResponseSchemaMismatch {
                operation_id: binding.operation_id.clone(),
                contract_path: operation.contract_path.clone(),
                status: status.clone(),
                expected_schema: "<missing runtime binding response schema>".into(),
                actual_schema: actual_schema.clone(),
            });
        };
        if expected_schema != actual_schema {
            return Err(OpenApiSourceError::RuntimeResponseSchemaMismatch {
                operation_id: binding.operation_id.clone(),
                contract_path: operation.contract_path.clone(),
                status: status.clone(),
                expected_schema: expected_schema.clone(),
                actual_schema: actual_schema.clone(),
            });
        }
    }
    for status in binding.response_schemas.keys() {
        if !operation.response_statuses.contains(status) {
            return Err(OpenApiSourceError::RuntimeResponseSchemaMismatch {
                operation_id: binding.operation_id.clone(),
                contract_path: operation.contract_path.clone(),
                status: status.clone(),
                expected_schema: binding
                    .response_schemas
                    .get(status)
                    .cloned()
                    .unwrap_or_default(),
                actual_schema: "<undocumented response status>".into(),
            });
        }
    }
    let source_statuses = match runtime_status_codes(source, &binding.status_type) {
        Ok(statuses) => statuses,
        Err(RuntimeStatusParseError::MissingStatusType) => {
            return Err(OpenApiSourceError::MissingRuntimeStatusType {
                operation_id: binding.operation_id.clone(),
                path: binding.source_path.clone(),
                status_type: binding.status_type.clone(),
            });
        }
        Err(RuntimeStatusParseError::Invalid(reason)) => {
            return Err(OpenApiSourceError::InvalidRuntimeStatusType {
                operation_id: binding.operation_id.clone(),
                path: binding.source_path.clone(),
                status_type: binding.status_type.clone(),
                reason,
            });
        }
    };
    for status in &operation.response_statuses {
        if !source_statuses.contains(status) {
            return Err(OpenApiSourceError::MissingRuntimeResponseStatus {
                operation_id: binding.operation_id.clone(),
                path: binding.source_path.clone(),
                status_type: binding.status_type.clone(),
                status: status.clone(),
            });
        }
    }
    for status in &source_statuses {
        if !operation.response_statuses.contains(status) {
            return Err(OpenApiSourceError::UndocumentedRuntimeResponseStatus {
                operation_id: binding.operation_id.clone(),
                path: binding.source_path.clone(),
                status_type: binding.status_type.clone(),
                status: status.clone(),
            });
        }
    }

    let test =
        tests
            .get(&binding.test_path)
            .ok_or_else(|| OpenApiSourceError::MissingRuntimeTest {
                operation_id: binding.operation_id.clone(),
                path: binding.test_path.clone(),
            })?;
    if !(rust_code_contains_identifier(test, &binding.symbol)
        && rust_string_literal_equals(test, &binding.evidence_surface))
    {
        return Err(OpenApiSourceError::MissingRuntimeTestCoverage {
            operation_id: binding.operation_id.clone(),
            test_path: binding.test_path.clone(),
            symbol: binding.symbol.clone(),
            evidence_surface: binding.evidence_surface.clone(),
        });
    }
    for status in &operation.response_statuses {
        if !runtime_test_covers_status(test, &binding.status_type, status) {
            return Err(OpenApiSourceError::MissingRuntimeTestResponseStatus {
                operation_id: binding.operation_id.clone(),
                test_path: binding.test_path.clone(),
                status_type: binding.status_type.clone(),
                status: status.clone(),
            });
        }
    }
    Ok(RuntimeBindingValidationReport {
        response_statuses_checked: operation.response_statuses.len(),
        response_schemas_checked: operation.response_schema_refs.len(),
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeBindingValidationReport {
    pub(crate) response_statuses_checked: usize,
    pub(crate) response_schemas_checked: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RuntimeStatusParseError {
    MissingStatusType,
    Invalid(String),
}

fn runtime_status_codes(
    source: &str,
    status_type: &str,
) -> Result<BTreeSet<String>, RuntimeStatusParseError> {
    let variants = rust_enum_variants(source, status_type)?;
    if variants.is_empty() {
        return Err(RuntimeStatusParseError::Invalid(format!(
            "{status_type} must declare at least one fieldless variant"
        )));
    }

    let mut mappings = BTreeMap::<String, String>::new();
    let mut statuses = BTreeMap::<String, String>::new();
    for block in rust_status_code_match_blocks(source, status_type) {
        for (variant, status) in status_match_mappings(block)? {
            if !variants.contains(&variant) {
                return Err(RuntimeStatusParseError::Invalid(format!(
                    "{status_type} maps undeclared variant {variant}"
                )));
            }
            if let Some(previous_variant) = statuses.insert(status.clone(), variant.clone()) {
                return Err(RuntimeStatusParseError::Invalid(format!(
                    "{status_type} maps status {status} from both {previous_variant} and {variant}"
                )));
            }
            if mappings.insert(variant.clone(), status).is_some() {
                return Err(RuntimeStatusParseError::Invalid(format!(
                    "{status_type} maps variant {variant} more than once"
                )));
            }
        }
    }
    if mappings.is_empty() {
        return Err(RuntimeStatusParseError::Invalid(format!(
            "{status_type} must define code(self) with explicit Self::Variant => status mappings"
        )));
    }
    let mapped_variants = mappings.keys().cloned().collect::<BTreeSet<_>>();
    if mapped_variants != variants {
        let missing = variants
            .difference(&mapped_variants)
            .cloned()
            .collect::<Vec<_>>();
        let extra = mapped_variants
            .difference(&variants)
            .cloned()
            .collect::<Vec<_>>();
        return Err(RuntimeStatusParseError::Invalid(format!(
            "{status_type} code mappings do not cover enum variants: missing={missing:?}, extra={extra:?}"
        )));
    }
    Ok(mappings.into_values().collect())
}
