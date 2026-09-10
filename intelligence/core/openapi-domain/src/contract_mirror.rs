use crate::document::{OPENAPI_PREFIX, validate_document_path};
use crate::error::OpenApiSourceError;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenApiContractMirrorLocation {
    pub contract_id: String, // data_class: INTERNAL_ONLY
    pub location: String,    // data_class: INTERNAL_ONLY
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OpenApiContractMirrorReport {
    pub contracts_checked: usize,         // data_class: INTERNAL_ONLY
    pub spec_references_checked: usize,   // data_class: INTERNAL_ONLY
    pub mirror_references_checked: usize, // data_class: INTERNAL_ONLY
}

pub fn validate_openapi_contract_mirror<I, L>(
    openapi_paths: I,
    spec_contents: &str,
    mirror_locations: L,
) -> Result<OpenApiContractMirrorReport, OpenApiSourceError>
where
    I: IntoIterator,
    I::Item: AsRef<str>,
    L: IntoIterator<Item = OpenApiContractMirrorLocation>,
{
    let openapi_paths = openapi_paths
        .into_iter()
        .map(|path| {
            let path = path.as_ref().to_string();
            validate_document_path(&path)?;
            Ok(path)
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    if openapi_paths.is_empty() {
        return Err(OpenApiSourceError::NoDocuments);
    }

    let spec_paths = exact_openapi_paths_in_text(spec_contents);
    for path in &openapi_paths {
        if !spec_paths.contains(path) {
            return Err(OpenApiSourceError::MissingSpecMirror { path: path.clone() });
        }
    }
    if let Some(path) = spec_paths
        .iter()
        .find(|path| !openapi_paths.contains(*path))
    {
        return Err(OpenApiSourceError::StaleSpecMirror { path: path.clone() });
    }

    let locations = mirror_locations.into_iter().collect::<Vec<_>>();
    let mut exact_mirror_paths = BTreeSet::new();
    let mut mirror_references_checked = 0usize;
    for location in &locations {
        for reference in openapi_location_references(&location.location) {
            if is_openapi_glob_reference(&reference) {
                continue;
            }
            mirror_references_checked += 1;
            if !openapi_paths.contains(&reference) {
                return Err(OpenApiSourceError::StaleMachineMirror {
                    contract_id: location.contract_id.clone(),
                    path: reference,
                });
            }
            exact_mirror_paths.insert(reference);
        }
    }
    for path in &openapi_paths {
        if !exact_mirror_paths.contains(path) {
            return Err(OpenApiSourceError::MissingMachineMirror { path: path.clone() });
        }
    }

    Ok(OpenApiContractMirrorReport {
        contracts_checked: openapi_paths.len(),
        spec_references_checked: spec_paths.len(),
        mirror_references_checked,
    })
}

fn exact_openapi_paths_in_text(contents: &str) -> BTreeSet<String> {
    contents
        .split(|character: char| {
            character.is_whitespace()
                || matches!(
                    character,
                    '`' | '(' | ')' | '[' | ']' | ',' | ';' | '"' | '\''
                )
        })
        .filter_map(|token| {
            let token =
                token.trim_matches(|character: char| matches!(character, '.' | ':' | '!' | '?'));
            if is_exact_openapi_path(token) {
                Some(token.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn openapi_location_references(location: &str) -> Vec<String> {
    location
        .split(|character: char| character.is_whitespace() || matches!(character, '+' | ','))
        .filter_map(|token| {
            let token = token.trim_matches(|character: char| {
                matches!(character, '`' | '(' | ')' | '[' | ']' | '"' | '\'')
            });
            if token.starts_with("contracts/")
                && (token.contains(".yaml") || token.contains(".yml"))
            {
                Some(token.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn is_openapi_glob_reference(path: &str) -> bool {
    path.contains('*') || path.contains('<') || path.contains('>')
}

fn is_exact_openapi_path(path: &str) -> bool {
    path.starts_with(OPENAPI_PREFIX)
        && (path.ends_with(".yaml") || path.ends_with(".yml"))
        && !is_openapi_glob_reference(path)
}
