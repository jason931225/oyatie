use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudIacReleaseIndexSeed {
    pub(super) modules: Vec<CloudIacReleaseIndexModuleSeed>,
}

impl CloudIacReleaseIndexSeed {
    pub fn modules(&self) -> &[CloudIacReleaseIndexModuleSeed] {
        &self.modules
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudIacReleaseIndexModuleSeed {
    pub(super) namespace: String,
    pub(super) name: String,
    pub(super) system: String,
    pub(super) version: String,
    pub(super) source_path: String,
    pub(super) archive_file: String,
    pub(super) archive_sha256: String,
    pub(super) archive_media_type: String,
    pub(super) archive_source_location: Option<String>,
    pub(super) archive_source_integrity_sha256: Option<String>,
    pub(super) archive_source_version_id: Option<String>,
    pub(super) archive_source_generation: Option<String>,
    pub(super) evidence_ref: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CloudIacAppArchiveArtifact {
    pub(super) archive_file: PathBuf,
    pub(super) archive_sha256: String,
    pub(super) media_type: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CloudIacReleaseIndexError {
    EmptyDocument,
    MissingField { field: &'static str },
    MalformedJson { reason: String },
    EmptyModules,
    InvalidField { field: &'static str, reason: String },
    Io { path: String, reason: String },
    Domain(CloudIacError),
}

pub fn load_release_index_seed_from_path(
    path: impl AsRef<Path>,
) -> Result<CloudIacReleaseIndexSeed, CloudIacReleaseIndexError> {
    let path_ref = path.as_ref();
    let body = fs::read_to_string(path_ref).map_err(|error| CloudIacReleaseIndexError::Io {
        path: path_ref.display().to_string(),
        reason: error.to_string(),
    })?;
    load_release_index_seed_from_str(&body)
}

pub fn load_release_index_seed_from_str(
    input: &str,
) -> Result<CloudIacReleaseIndexSeed, CloudIacReleaseIndexError> {
    if input.trim().is_empty() {
        return Err(CloudIacReleaseIndexError::EmptyDocument);
    }
    let module_array = array_field_contents(input, "modules")?;
    let objects = top_level_object_slices(module_array)?;
    if objects.is_empty() {
        return Err(CloudIacReleaseIndexError::EmptyModules);
    }

    let mut modules = Vec::with_capacity(objects.len());
    for object in objects {
        let module = CloudIacReleaseIndexModuleSeed {
            namespace: required_string_field(object, "namespace")?,
            name: required_string_field(object, "name")?,
            system: required_string_field(object, "system")?,
            version: required_string_field(object, "version")?,
            source_path: required_string_field(object, "source_path")?,
            archive_file: required_string_field(object, "archive_file")?,
            archive_sha256: required_string_field(object, "archive_sha256")?,
            archive_media_type: required_string_field(object, "archive_media_type")?,
            archive_source_location: optional_string_field(object, "archive_source_location")?,
            archive_source_integrity_sha256: optional_string_field(
                object,
                "archive_source_integrity_sha256",
            )?,
            archive_source_version_id: optional_string_field(object, "archive_source_version_id")?,
            archive_source_generation: optional_string_field(object, "archive_source_generation")?,
            evidence_ref: required_string_field(object, "evidence_ref")?,
        };
        validate_release_index_module_seed(&module)?;
        modules.push(module);
    }

    Ok(CloudIacReleaseIndexSeed { modules })
}

pub fn build_module_registry_from_release_index_seed(
    seed: &CloudIacReleaseIndexSeed,
) -> Result<ModuleRegistry, CloudIacReleaseIndexError> {
    let mut registry = ModuleRegistry::default();
    for module in seed.modules() {
        let source = release_source_for_module(module)?;
        let digest = format!("sha256:{}", module.archive_sha256);
        let release = OpenTofuModuleRelease::new(
            module.namespace.clone(),
            module.name.clone(),
            module.system.clone(),
            module.version.clone(),
            source,
            digest,
            module.evidence_ref.clone(),
        )
        .map_err(CloudIacReleaseIndexError::Domain)?;
        registry
            .publish(release)
            .map_err(CloudIacReleaseIndexError::Domain)?;
    }
    Ok(registry)
}

pub(super) fn validate_release_index_module_seed(
    module: &CloudIacReleaseIndexModuleSeed,
) -> Result<(), CloudIacReleaseIndexError> {
    validate_non_empty("namespace", &module.namespace)?;
    validate_non_empty("name", &module.name)?;
    validate_non_empty("system", &module.system)?;
    validate_non_empty("version", &module.version)?;
    validate_release_source_path(&module.source_path)?;
    validate_archive_file(&module.archive_file, &module.version)?;
    validate_archive_sha256(&module.archive_sha256)?;
    validate_archive_media_type(&module.archive_media_type)?;
    validate_archive_source_location(module)?;
    validate_archive_source_pin(module)?;
    validate_evidence_ref_seed(&module.evidence_ref)?;
    Ok(())
}

pub(super) fn validate_non_empty(
    field: &'static str,
    value: &str,
) -> Result<(), CloudIacReleaseIndexError> {
    if value.trim().is_empty() {
        Err(CloudIacReleaseIndexError::InvalidField {
            field,
            reason: "value must be non-empty".to_string(),
        })
    } else {
        Ok(())
    }
}

pub(super) fn validate_release_source_path(path: &str) -> Result<(), CloudIacReleaseIndexError> {
    validate_non_empty("source_path", path)?;
    if contains_secret_like_marker(path) {
        return Err(CloudIacReleaseIndexError::InvalidField {
            field: "source_path",
            reason: "path contains a credential-like marker".to_string(),
        });
    }
    if !path.starts_with(CLOUD_IAC_APP_RELEASE_SOURCE_ROOT) {
        return Err(CloudIacReleaseIndexError::InvalidField {
            field: "source_path",
            reason: format!("path must start with {CLOUD_IAC_APP_RELEASE_SOURCE_ROOT}"),
        });
    }
    if path.starts_with('/')
        || path.contains('\\')
        || path.contains('?')
        || path.contains('#')
        || path.contains("//")
    {
        return Err(CloudIacReleaseIndexError::InvalidField {
            field: "source_path",
            reason:
                "path must be repo-relative without query, fragment, backslash, or empty segment"
                    .to_string(),
        });
    }
    for segment in path.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(CloudIacReleaseIndexError::InvalidField {
                field: "source_path",
                reason: "path contains an empty, current-directory, or parent-directory segment"
                    .to_string(),
            });
        }
        if !segment
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
        {
            return Err(CloudIacReleaseIndexError::InvalidField {
                field: "source_path",
                reason: "path segment contains unsupported characters".to_string(),
            });
        }
    }
    Ok(())
}

pub(super) fn validate_archive_sha256(value: &str) -> Result<(), CloudIacReleaseIndexError> {
    if value.len() == 64 && value.chars().all(|ch| ch.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(CloudIacReleaseIndexError::InvalidField {
            field: "archive_sha256",
            reason: "archive digest must be exactly 64 hexadecimal characters without prefix"
                .to_string(),
        })
    }
}

pub(super) fn validate_archive_file(
    path: &str,
    version: &str,
) -> Result<(), CloudIacReleaseIndexError> {
    validate_non_empty("archive_file", path)?;
    if contains_secret_like_marker(path) {
        return Err(CloudIacReleaseIndexError::InvalidField {
            field: "archive_file",
            reason: "path contains a credential-like marker".to_string(),
        });
    }
    if !path.starts_with(CLOUD_IAC_APP_ARCHIVE_FILE_ROOT) {
        return Err(CloudIacReleaseIndexError::InvalidField {
            field: "archive_file",
            reason: format!("path must start with {CLOUD_IAC_APP_ARCHIVE_FILE_ROOT}"),
        });
    }
    if path.contains('\\') || path.contains('?') || path.contains('#') || path.contains("//") {
        return Err(CloudIacReleaseIndexError::InvalidField {
            field: "archive_file",
            reason:
                "path must be repo-relative without query, fragment, backslash, or empty segment"
                    .to_string(),
        });
    }
    let file_name = archive_file_name(path)?;
    let Some(relative_file_name) = path.strip_prefix(CLOUD_IAC_APP_ARCHIVE_FILE_ROOT) else {
        return Err(CloudIacReleaseIndexError::InvalidField {
            field: "archive_file",
            reason: format!("path must start with {CLOUD_IAC_APP_ARCHIVE_FILE_ROOT}"),
        });
    };
    if relative_file_name != file_name {
        return Err(CloudIacReleaseIndexError::InvalidField {
            field: "archive_file",
            reason: "archive file must live directly under the local module archive root"
                .to_string(),
        });
    }
    if !is_valid_archive_file_name(&file_name, version) {
        return Err(CloudIacReleaseIndexError::InvalidField {
            field: "archive_file",
            reason:
                "archive filename must be a safe lowercase .zip name pinned to the module version"
                    .to_string(),
        });
    }
    for segment in path.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(CloudIacReleaseIndexError::InvalidField {
                field: "archive_file",
                reason: "path contains an empty, current-directory, or parent-directory segment"
                    .to_string(),
            });
        }
    }
    Ok(())
}

pub(super) fn validate_archive_media_type(value: &str) -> Result<(), CloudIacReleaseIndexError> {
    if value == CLOUD_IAC_APP_ARCHIVE_MEDIA_TYPE {
        Ok(())
    } else {
        Err(CloudIacReleaseIndexError::InvalidField {
            field: "archive_media_type",
            reason: format!("archive media type must be {CLOUD_IAC_APP_ARCHIVE_MEDIA_TYPE}"),
        })
    }
}
