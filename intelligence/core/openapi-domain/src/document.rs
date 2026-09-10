use crate::data_class::validate_data_class_annotations;
use crate::error::{OpenApiSourceError, invalid_path};
use crate::operation::collect_document_operations;
use crate::path_validation::validate_paths;
use crate::security::validate_bearer_security_scheme;
use crate::yaml::{block_scalar_value, logical_lines, top_level_block, top_level_value};
use std::collections::BTreeMap;

pub(crate) const OPENAPI_PREFIX: &str = "contracts/openapi/";
const SUPPORTED_OPENAPI_MAJOR_MINOR: &str = "3.2.";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenApiDocument {
    pub path: String,     // data_class: INTERNAL_ONLY
    pub contents: String, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OpenApiSourceReport {
    pub documents_checked: usize,              // data_class: INTERNAL_ONLY
    pub operations_checked: usize,             // data_class: INTERNAL_ONLY
    pub data_class_annotations_checked: usize, // data_class: INTERNAL_ONLY
}

pub fn validate_openapi_documents<I>(
    documents: I,
) -> Result<OpenApiSourceReport, OpenApiSourceError>
where
    I: IntoIterator<Item = OpenApiDocument>,
{
    let documents = document_map(documents)?;
    let mut operations_checked = 0usize;
    let mut data_class_annotations_checked = 0usize;
    let mut operation_ids = BTreeMap::<String, String>::new();
    for (path, contents) in &documents {
        let report = validate_document(path, contents)?;
        for operation in collect_document_operations(path, contents)? {
            if let Some(first_path) = operation_ids.insert(
                operation.operation_id.clone(),
                operation.contract_path.clone(),
            ) {
                return Err(OpenApiSourceError::DuplicateOperationId {
                    operation_id: operation.operation_id,
                    first_path,
                    second_path: operation.contract_path,
                });
            }
        }
        operations_checked += report.operations_checked;
        data_class_annotations_checked += report.data_class_annotations_checked;
    }
    Ok(OpenApiSourceReport {
        documents_checked: documents.len(),
        operations_checked,
        data_class_annotations_checked,
    })
}

pub(crate) fn document_map<I>(documents: I) -> Result<BTreeMap<String, String>, OpenApiSourceError>
where
    I: IntoIterator<Item = OpenApiDocument>,
{
    let mut map = BTreeMap::new();
    for document in documents {
        validate_document_path(&document.path)?;
        if document.contents.trim().is_empty() {
            return Err(OpenApiSourceError::EmptyDocument {
                path: document.path,
            });
        }
        if map
            .insert(document.path.clone(), document.contents)
            .is_some()
        {
            return Err(OpenApiSourceError::DuplicateDocument {
                path: document.path,
            });
        }
    }
    if map.is_empty() {
        return Err(OpenApiSourceError::NoDocuments);
    }
    Ok(map)
}

pub(crate) fn validate_document_path(path: &str) -> Result<(), OpenApiSourceError> {
    if path.trim().is_empty() {
        return invalid_path(path, "path must be non-empty");
    }
    if path.starts_with('/') || path.contains('\\') || path.contains('\0') {
        return invalid_path(path, "path must be a relative slash path");
    }
    if path
        .split('/')
        .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return invalid_path(
            path,
            "path must not contain empty, dot, or parent components",
        );
    }
    if !path.starts_with(OPENAPI_PREFIX) {
        return invalid_path(path, "OpenAPI documents must live under contracts/openapi/");
    }
    if !(path.ends_with(".yaml") || path.ends_with(".yml")) {
        return invalid_path(path, "OpenAPI documents must be .yaml or .yml files");
    }
    if path.ends_with(".meta.yaml") || path.ends_with(".meta.yml") {
        return invalid_path(path, "OpenAPI metadata files are not source documents");
    }
    if version_suffix_major(path).is_none() {
        return invalid_path(path, "OpenAPI document filenames must end in -v<major>");
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DocumentValidationReport {
    pub(crate) operations_checked: usize,
    pub(crate) data_class_annotations_checked: usize,
}

pub(crate) fn validate_document(
    path: &str,
    contents: &str,
) -> Result<DocumentValidationReport, OpenApiSourceError> {
    let lines = logical_lines(contents);
    let openapi = top_level_value(&lines, "openapi").ok_or_else(|| {
        OpenApiSourceError::MissingTopLevelField {
            path: path.into(),
            field: "openapi",
        }
    })?;
    if !openapi.starts_with(SUPPORTED_OPENAPI_MAJOR_MINOR) {
        return Err(OpenApiSourceError::UnsupportedOpenApiVersion {
            path: path.into(),
            version: openapi,
        });
    }

    let info_range = top_level_block(&lines, "info").ok_or_else(|| {
        OpenApiSourceError::MissingTopLevelField {
            path: path.into(),
            field: "info",
        }
    })?;
    let title = block_scalar_value(&lines, info_range.clone(), 2, "title").ok_or_else(|| {
        OpenApiSourceError::MissingInfoField {
            path: path.into(),
            field: "title",
        }
    })?;
    if title.trim().is_empty() {
        return Err(OpenApiSourceError::MissingInfoField {
            path: path.into(),
            field: "title",
        });
    }
    let info_version = block_scalar_value(&lines, info_range, 2, "version").ok_or_else(|| {
        OpenApiSourceError::MissingInfoField {
            path: path.into(),
            field: "version",
        }
    })?;
    let info_major =
        semver_major(&info_version).ok_or_else(|| OpenApiSourceError::InvalidInfoVersion {
            path: path.into(),
            version: info_version.clone(),
        })?;
    let path_major = version_suffix_major(path).ok_or_else(|| OpenApiSourceError::InvalidPath {
        path: path.into(),
        reason: "OpenAPI document filenames must end in -v<major>".into(),
    })?;
    if info_major != path_major {
        return Err(OpenApiSourceError::VersionSuffixMismatch {
            path: path.into(),
            path_major,
            info_major,
        });
    }

    let paths_range = top_level_block(&lines, "paths").ok_or_else(|| {
        OpenApiSourceError::MissingTopLevelField {
            path: path.into(),
            field: "paths",
        }
    })?;
    validate_bearer_security_scheme(path, &lines)?;
    let operations_checked = validate_paths(path, &lines, paths_range)?;
    let data_class_annotations_checked = validate_data_class_annotations(path, &lines)?;
    Ok(DocumentValidationReport {
        operations_checked,
        data_class_annotations_checked,
    })
}

fn version_suffix_major(path: &str) -> Option<u64> {
    let file_name = path.rsplit('/').next()?;
    let stem = file_name.split('.').next()?;
    let (_, suffix) = stem.rsplit_once("-v")?;
    suffix.parse::<u64>().ok()
}

fn semver_major(version: &str) -> Option<u64> {
    let mut parts = version.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next()?.parse::<u64>().ok()?;
    let patch = parts.next()?.parse::<u64>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    let _ = (minor, patch);
    Some(major)
}
