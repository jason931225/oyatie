struct ValidatedRequest {
    provider: PathBuf,
    cargo: PathBuf,
    rustc: PathBuf,
    first_root: PathBuf,
    second_root: PathBuf,
    first_cargo_home: PathBuf,
    second_cargo_home: PathBuf,
    first_target: PathBuf,
    second_target: PathBuf,
}

impl ValidatedRequest {
    fn new(
        request: &CandidateHeadQualificationRequest,
    ) -> Result<Self, CandidateHeadQualificationFailure> {
        let provider = canonical_file(
            QualificationPath::ProviderExecutable,
            &request.provider_executable,
            true,
        )?;
        let cargo = canonical_file(
            QualificationPath::CargoExecutable,
            &request.cargo_executable,
            true,
        )?;
        let rustc = canonical_file(
            QualificationPath::RustcExecutable,
            &request.rustc_executable,
            true,
        )?;
        let first_root = canonical_directory(
            QualificationPath::FirstCandidateRoot,
            &request.first_candidate_root,
        )?;
        let second_root = canonical_directory(
            QualificationPath::SecondCandidateRoot,
            &request.second_candidate_root,
        )?;
        let first_target = canonical_nonexistent_path(
            QualificationPath::FirstTargetDirectory,
            &request.first_target_dir,
        )?;
        let second_target = canonical_nonexistent_path(
            QualificationPath::SecondTargetDirectory,
            &request.second_target_dir,
        )?;

        let paths = [
            (QualificationPath::ProviderExecutable, provider.as_path()),
            (QualificationPath::CargoExecutable, cargo.as_path()),
            (QualificationPath::RustcExecutable, rustc.as_path()),
            (QualificationPath::FirstCandidateRoot, first_root.as_path()),
            (
                QualificationPath::SecondCandidateRoot,
                second_root.as_path(),
            ),
            (
                QualificationPath::FirstTargetDirectory,
                first_target.as_path(),
            ),
            (
                QualificationPath::SecondTargetDirectory,
                second_target.as_path(),
            ),
        ];
        for (index, (field, path)) in paths.iter().enumerate() {
            for (_, other) in paths.iter().skip(index + 1) {
                if paths_overlap(path, other) {
                    return Err(CandidateHeadQualificationFailure::InvalidPath {
                        field: *field,
                        path: (*path).to_path_buf(),
                        reason: PathRefusal::OverlapsAnotherInput,
                    });
                }
            }
        }
        reject_executable_aliases([
            (QualificationPath::ProviderExecutable, provider.as_path()),
            (QualificationPath::CargoExecutable, cargo.as_path()),
            (QualificationPath::RustcExecutable, rustc.as_path()),
        ])?;

        Ok(Self {
            first_cargo_home: first_root.join("third-party/.cargo"),
            second_cargo_home: second_root.join("third-party/.cargo"),
            provider,
            cargo,
            rustc,
            first_root,
            second_root,
            first_target,
            second_target,
        })
    }
}

fn canonical_file(
    field: QualificationPath,
    path: &Path,
    executable: bool,
) -> Result<PathBuf, CandidateHeadQualificationFailure> {
    ensure_absolute(field, path)?;
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| invalid_path_from_io(field, path, error))?;
    if metadata.file_type().is_symlink() {
        return Err(invalid_path(field, path, PathRefusal::Symlink));
    }
    if !metadata.is_file() {
        return Err(invalid_path(field, path, PathRefusal::NotAFile));
    }
    if executable && !is_executable(&metadata) {
        return Err(invalid_path(field, path, PathRefusal::NotExecutable));
    }
    let canonical = fs::canonicalize(path)
        .map_err(|error| invalid_path_from_io(field, path, error))?;
    if canonical != path {
        return Err(invalid_path(field, path, PathRefusal::NonCanonical));
    }
    Ok(canonical)
}

fn canonical_directory(
    field: QualificationPath,
    path: &Path,
) -> Result<PathBuf, CandidateHeadQualificationFailure> {
    ensure_absolute(field, path)?;
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| invalid_path_from_io(field, path, error))?;
    if metadata.file_type().is_symlink() {
        return Err(invalid_path(field, path, PathRefusal::Symlink));
    }
    if !metadata.is_dir() {
        return Err(invalid_path(field, path, PathRefusal::NotADirectory));
    }
    let canonical = fs::canonicalize(path)
        .map_err(|error| invalid_path_from_io(field, path, error))?;
    if canonical != path {
        return Err(invalid_path(field, path, PathRefusal::NonCanonical));
    }
    Ok(canonical)
}

fn canonical_nonexistent_path(
    field: QualificationPath,
    path: &Path,
) -> Result<PathBuf, CandidateHeadQualificationFailure> {
    ensure_absolute(field, path)?;
    match fs::symlink_metadata(path) {
        Ok(_) => return Err(invalid_path(field, path, PathRefusal::AlreadyExists)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(invalid_path_from_io(field, path, error)),
    }
    let parent = path
        .parent()
        .ok_or_else(|| invalid_path(field, path, PathRefusal::ParentMissing))?;
    let name = path
        .file_name()
        .ok_or_else(|| invalid_path(field, path, PathRefusal::InvalidName))?;
    let parent_metadata = fs::symlink_metadata(parent)
        .map_err(|error| invalid_path_from_io(field, parent, error))?;
    if parent_metadata.file_type().is_symlink() || !parent_metadata.is_dir() {
        return Err(invalid_path(field, parent, PathRefusal::ParentMissing));
    }
    let canonical_parent = fs::canonicalize(parent)
        .map_err(|error| invalid_path_from_io(field, parent, error))?;
    let canonical = canonical_parent.join(name);
    if canonical != path {
        return Err(invalid_path(field, path, PathRefusal::NonCanonical));
    }
    Ok(canonical)
}

fn ensure_absolute(
    field: QualificationPath,
    path: &Path,
) -> Result<(), CandidateHeadQualificationFailure> {
    if path.is_absolute() {
        Ok(())
    } else {
        Err(invalid_path(field, path, PathRefusal::NotAbsolute))
    }
}

fn paths_overlap(first: &Path, second: &Path) -> bool {
    first.starts_with(second) || second.starts_with(first)
}

#[cfg(unix)]
fn is_executable(metadata: &Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_metadata: &Metadata) -> bool {
    false
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FileIdentity {
    device: u64,
    inode: u64,
}

#[cfg(unix)]
fn file_identity(metadata: &Metadata) -> FileIdentity {
    use std::os::unix::fs::MetadataExt;
    FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    }
}

#[cfg(not(unix))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FileIdentity;

#[cfg(not(unix))]
fn file_identity(_metadata: &Metadata) -> FileIdentity {
    FileIdentity
}

fn reject_executable_aliases(
    tools: [(QualificationPath, &Path); 3],
) -> Result<(), CandidateHeadQualificationFailure> {
    let mut identities = Vec::with_capacity(tools.len());
    for (field, path) in tools {
        let metadata = fs::metadata(path)
            .map_err(|error| invalid_path_from_io(field, path, error))?;
        identities.push((field, path, file_identity(&metadata)));
    }
    for first in 0..identities.len() {
        for second in (first + 1)..identities.len() {
            if identities[first].2 == identities[second].2 {
                return Err(invalid_path(
                    identities[first].0,
                    identities[first].1,
                    PathRefusal::AliasesAnotherExecutable,
                ));
            }
        }
    }
    Ok(())
}

fn reject_ancestor_cargo_config(
    root: CandidateRoot,
    candidate_root: &Path,
) -> Result<(), CandidateHeadQualificationFailure> {
    let mut ancestor = candidate_root.parent();
    while let Some(path) = ancestor {
        for relative in [".cargo/config", ".cargo/config.toml"] {
            let config = path.join(relative);
            match fs::symlink_metadata(&config) {
                Ok(_) => {
                    return Err(CandidateHeadQualificationFailure::AncestorCargoConfig {
                        root,
                        path: config,
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(CandidateHeadQualificationFailure::CandidateTreeRead {
                        root,
                        scope: CandidateTreeScope::Semantic,
                        path: config,
                        kind: error.kind(),
                    });
                }
            }
        }
        ancestor = path.parent();
    }
    Ok(())
}
