fn snapshot_tree(
    root: CandidateRoot,
    directory: &Path,
    scope: CandidateTreeScope,
    limits: QualificationLimits,
    tool_identities: &[FileIdentity],
) -> Result<TreeSnapshot, CandidateHeadQualificationFailure> {
    let mut budget = TreeBudget::new(root, scope, limits);
    let metadata = tree_directory_metadata(root, scope, directory, Path::new(""))?;
    let mut snapshot = TreeSnapshot {
        entries: vec![SnapshotEntry {
            path: PathBuf::new(),
            mode: metadata_mode(&metadata),
            contents: None,
        }],
    };
    snapshot_directory(
        root,
        directory,
        Path::new(""),
        1,
        scope,
        &mut budget,
        &mut snapshot,
        tool_identities,
    )?;
    Ok(snapshot)
}

#[allow(clippy::too_many_arguments)]
fn snapshot_directory(
    root: CandidateRoot,
    directory: &Path,
    relative_directory: &Path,
    depth: u64,
    scope: CandidateTreeScope,
    budget: &mut TreeBudget,
    snapshot: &mut TreeSnapshot,
    tool_identities: &[FileIdentity],
) -> Result<(), CandidateHeadQualificationFailure> {
    let entries = read_tree_entries(
        root,
        directory,
        relative_directory,
        scope,
        budget.limits.tree_entries,
    )?;
    for entry in entries {
        let relative_path = relative_directory.join(&entry.name);
        budget.enter(
            depth,
            (!entry.is_directory).then_some(entry.metadata.len()),
        )?;
        if entry.is_directory {
            snapshot.entries.push(SnapshotEntry {
                path: relative_path.clone(),
                mode: metadata_mode(&entry.metadata),
                contents: None,
            });
            snapshot_directory(
                root,
                &entry.path,
                &relative_path,
                depth.saturating_add(1),
                scope,
                budget,
                snapshot,
                tool_identities,
            )?;
        } else {
            reject_tool_alias(
                root,
                scope,
                &relative_path,
                &entry.metadata,
                tool_identities,
            )?;
            snapshot.entries.push(SnapshotEntry {
                path: relative_path.clone(),
                mode: metadata_mode(&entry.metadata),
                contents: Some(read_regular_file(
                    root,
                    scope,
                    &entry.path,
                    &relative_path,
                    budget.limits.tree_file_bytes,
                )?),
            });
        }
    }
    Ok(())
}

fn verify_tree_unchanged(
    root: CandidateRoot,
    directory: &Path,
    scope: CandidateTreeScope,
    expected: &TreeSnapshot,
    limits: QualificationLimits,
    tool_identities: &[FileIdentity],
) -> Result<(), CandidateHeadQualificationFailure> {
    let actual = snapshot_tree(root, directory, scope, limits, tool_identities)?;
    if &actual == expected {
        Ok(())
    } else {
        Err(CandidateHeadQualificationFailure::CandidateTreeChanged { root, scope })
    }
}

fn read_tree_entries(
    root: CandidateRoot,
    directory: &Path,
    relative_directory: &Path,
    scope: CandidateTreeScope,
    entry_bound: u64,
) -> Result<Vec<TreeEntry>, CandidateHeadQualificationFailure> {
    let entries = fs::read_dir(directory)
        .map_err(|error| tree_read_error(root, scope, relative_directory, error))?;
    let initial_capacity = usize::try_from(cmp::min(entry_bound, 1024)).unwrap_or(1024);
    let mut result = Vec::with_capacity(initial_capacity);
    for entry in entries {
        if result.len() as u64 >= entry_bound {
            return Err(CandidateHeadQualificationFailure::CandidateTreeLimitExceeded {
                root,
                scope,
                limit: QualificationLimit::TreeEntries,
                value: (result.len() as u64).saturating_add(1),
                maximum: entry_bound,
            });
        }
        let entry =
            entry.map_err(|error| tree_read_error(root, scope, relative_directory, error))?;
        let name = entry.file_name().into_string().map_err(|_| {
            CandidateHeadQualificationFailure::UnsupportedCandidateEntry {
                root,
                scope,
                path: relative_directory.to_path_buf(),
                kind: UnsupportedCandidateEntryKind::NonUtf8Name,
            }
        })?;
        let relative_path = relative_directory.join(&name);
        let file_type = entry
            .file_type()
            .map_err(|error| tree_read_error(root, scope, &relative_path, error))?;
        if scope == CandidateTreeScope::Semantic
            && relative_directory == Path::new("third-party")
            && name == ".cargo"
            && file_type.is_dir()
        {
            continue;
        }
        if name == ".git" {
            return Err(CandidateHeadQualificationFailure::UnsupportedCandidateEntry {
                root,
                scope,
                path: relative_path,
                kind: UnsupportedCandidateEntryKind::ScmMetadata,
            });
        }
        if file_type.is_symlink() {
            return Err(CandidateHeadQualificationFailure::UnsupportedCandidateEntry {
                root,
                scope,
                path: relative_path,
                kind: UnsupportedCandidateEntryKind::Symlink,
            });
        }
        if !file_type.is_file() && !file_type.is_dir() {
            return Err(CandidateHeadQualificationFailure::UnsupportedCandidateEntry {
                root,
                scope,
                path: relative_path,
                kind: UnsupportedCandidateEntryKind::Special,
            });
        }
        let metadata = entry
            .metadata()
            .map_err(|error| tree_read_error(root, scope, &relative_path, error))?;
        result.push(TreeEntry {
            name,
            path: entry.path(),
            metadata,
            is_directory: file_type.is_dir(),
        });
    }
    result.sort_by(|first, second| first.name.as_bytes().cmp(second.name.as_bytes()));
    Ok(result)
}

fn tree_read_error(
    root: CandidateRoot,
    scope: CandidateTreeScope,
    path: &Path,
    error: io::Error,
) -> CandidateHeadQualificationFailure {
    CandidateHeadQualificationFailure::CandidateTreeRead {
        root,
        scope,
        path: path.to_path_buf(),
        kind: error.kind(),
    }
}

fn tree_directory_metadata(
    root: CandidateRoot,
    scope: CandidateTreeScope,
    path: &Path,
    relative_path: &Path,
) -> Result<Metadata, CandidateHeadQualificationFailure> {
    fs::metadata(path).map_err(|error| tree_read_error(root, scope, relative_path, error))
}

fn reject_tool_alias(
    root: CandidateRoot,
    scope: CandidateTreeScope,
    relative_path: &Path,
    metadata: &Metadata,
    tool_identities: &[FileIdentity],
) -> Result<(), CandidateHeadQualificationFailure> {
    if tool_identities.contains(&file_identity(metadata)) {
        Err(CandidateHeadQualificationFailure::UnsupportedCandidateEntry {
            root,
            scope,
            path: relative_path.to_path_buf(),
            kind: UnsupportedCandidateEntryKind::AliasesExternalTool,
        })
    } else {
        Ok(())
    }
}

#[cfg(unix)]
fn metadata_mode(metadata: &Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o777
}

#[cfg(not(unix))]
fn metadata_mode(_metadata: &Metadata) -> u32 {
    0
}

