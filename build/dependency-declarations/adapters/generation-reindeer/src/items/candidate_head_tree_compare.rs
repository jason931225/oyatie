fn first_difference(first: &[u8], second: &[u8]) -> usize {
    first
        .iter()
        .zip(second)
        .position(|(first, second)| first != second)
        .unwrap_or_else(|| cmp::min(first.len(), second.len()))
}

#[derive(Debug, Eq, PartialEq)]
struct TreeSnapshot {
    entries: Vec<SnapshotEntry>,
}

#[derive(Debug, Eq, PartialEq)]
struct SnapshotEntry {
    path: PathBuf,
    mode: u32,
    contents: Option<Vec<u8>>,
}

fn external_tool_identities(
    request: &ValidatedRequest,
) -> Result<[FileIdentity; 3], CandidateHeadQualificationFailure> {
    Ok([
        external_tool_identity(QualificationTool::Provider, &request.provider)?,
        external_tool_identity(QualificationTool::Cargo, &request.cargo)?,
        external_tool_identity(QualificationTool::Rustc, &request.rustc)?,
    ])
}

fn external_tool_identity(
    tool: QualificationTool,
    path: &Path,
) -> Result<FileIdentity, CandidateHeadQualificationFailure> {
    fs::metadata(path)
        .map(|metadata| file_identity(&metadata))
        .map_err(|error| CandidateHeadQualificationFailure::ToolRead {
            tool,
            path: path.to_path_buf(),
            kind: error.kind(),
        })
}

struct TreeBudget {
    root: CandidateRoot,
    scope: CandidateTreeScope,
    entries: u64,
    bytes: u64,
    limits: QualificationLimits,
}

impl TreeBudget {
    fn new(root: CandidateRoot, scope: CandidateTreeScope, limits: QualificationLimits) -> Self {
        Self {
            root,
            scope,
            entries: 0,
            bytes: 0,
            limits,
        }
    }

    fn enter(
        &mut self,
        depth: u64,
        file_bytes: Option<u64>,
    ) -> Result<(), CandidateHeadQualificationFailure> {
        if depth > self.limits.tree_depth {
            return Err(self.exceeded(
                QualificationLimit::TreeDepth,
                depth,
                self.limits.tree_depth,
            ));
        }
        self.entries = self.entries.saturating_add(1);
        if self.entries > self.limits.tree_entries {
            return Err(self.exceeded(
                QualificationLimit::TreeEntries,
                self.entries,
                self.limits.tree_entries,
            ));
        }
        if let Some(file_bytes) = file_bytes {
            if file_bytes > self.limits.tree_file_bytes {
                return Err(self.exceeded(
                    QualificationLimit::TreeFileBytes,
                    file_bytes,
                    self.limits.tree_file_bytes,
                ));
            }
            self.bytes = self.bytes.saturating_add(file_bytes);
            if self.bytes > self.limits.tree_total_bytes {
                return Err(self.exceeded(
                    QualificationLimit::TreeTotalBytes,
                    self.bytes,
                    self.limits.tree_total_bytes,
                ));
            }
        }
        Ok(())
    }

    fn exceeded(
        &self,
        limit: QualificationLimit,
        value: u64,
        maximum: u64,
    ) -> CandidateHeadQualificationFailure {
        CandidateHeadQualificationFailure::CandidateTreeLimitExceeded {
            root: self.root,
            scope: self.scope,
            limit,
            value,
            maximum,
        }
    }
}

struct TreeEntry {
    name: String,
    path: PathBuf,
    metadata: Metadata,
    is_directory: bool,
}

fn compare_trees(
    first: &Path,
    second: &Path,
    scope: CandidateTreeScope,
    limits: QualificationLimits,
    tool_identities: &[FileIdentity],
    storage: &mut IndependentStorage,
) -> Result<TreeSnapshot, CandidateHeadQualificationFailure> {
    let mut first_budget = TreeBudget::new(CandidateRoot::First, scope, limits);
    let mut second_budget = TreeBudget::new(CandidateRoot::Second, scope, limits);
    let first_metadata =
        tree_directory_metadata(CandidateRoot::First, scope, first, Path::new(""))?;
    let second_metadata =
        tree_directory_metadata(CandidateRoot::Second, scope, second, Path::new(""))?;
    if metadata_mode(&first_metadata) != metadata_mode(&second_metadata) {
        return Err(CandidateHeadQualificationFailure::CandidateTreesDiffer {
            scope,
            path: PathBuf::new(),
        });
    }
    let mut snapshot = TreeSnapshot {
        entries: vec![SnapshotEntry {
            path: PathBuf::new(),
            mode: metadata_mode(&first_metadata),
            contents: None,
        }],
    };
    compare_directories(
        first,
        second,
        Path::new(""),
        1,
        scope,
        &mut first_budget,
        &mut second_budget,
        &mut snapshot,
        tool_identities,
        storage,
    )?;
    Ok(snapshot)
}

#[allow(clippy::too_many_arguments)]
fn compare_directories(
    first_directory: &Path,
    second_directory: &Path,
    relative_directory: &Path,
    depth: u64,
    scope: CandidateTreeScope,
    first_budget: &mut TreeBudget,
    second_budget: &mut TreeBudget,
    snapshot: &mut TreeSnapshot,
    tool_identities: &[FileIdentity],
    storage: &mut IndependentStorage,
) -> Result<(), CandidateHeadQualificationFailure> {
    let first_entries = read_tree_entries(
        CandidateRoot::First,
        first_directory,
        relative_directory,
        scope,
        first_budget.limits.tree_entries,
    )?;
    let second_entries = read_tree_entries(
        CandidateRoot::Second,
        second_directory,
        relative_directory,
        scope,
        second_budget.limits.tree_entries,
    )?;
    if first_entries.len() != second_entries.len() {
        return Err(CandidateHeadQualificationFailure::CandidateTreesDiffer {
            scope,
            path: relative_directory.to_path_buf(),
        });
    }

    for (first, second) in first_entries.iter().zip(&second_entries) {
        let relative_path = relative_directory.join(&first.name);
        if first.name != second.name
            || first.is_directory != second.is_directory
            || metadata_mode(&first.metadata) != metadata_mode(&second.metadata)
        {
            return Err(CandidateHeadQualificationFailure::CandidateTreesDiffer {
                scope,
                path: relative_path,
            });
        }
        first_budget.enter(depth, (!first.is_directory).then_some(first.metadata.len()))?;
        second_budget.enter(
            depth,
            (!second.is_directory).then_some(second.metadata.len()),
        )?;
        if first.is_directory {
            snapshot.entries.push(SnapshotEntry {
                path: relative_path.clone(),
                mode: metadata_mode(&first.metadata),
                contents: None,
            });
            compare_directories(
                &first.path,
                &second.path,
                &relative_path,
                depth.saturating_add(1),
                scope,
                first_budget,
                second_budget,
                snapshot,
                tool_identities,
                storage,
            )?;
        } else {
            reject_tool_alias(
                CandidateRoot::First,
                scope,
                &relative_path,
                &first.metadata,
                tool_identities,
            )?;
            reject_tool_alias(
                CandidateRoot::Second,
                scope,
                &relative_path,
                &second.metadata,
                tool_identities,
            )?;
            storage.observe(&first.metadata, &second.metadata, scope, &relative_path)?;
            if first.metadata.len() != second.metadata.len() {
                return Err(CandidateHeadQualificationFailure::CandidateTreesDiffer {
                    scope,
                    path: relative_path,
                });
            }
            let contents = compare_regular_files(
                &first.path,
                &second.path,
                &relative_path,
                scope,
                first_budget.limits.tree_file_bytes,
            )?;
            snapshot.entries.push(SnapshotEntry {
                path: relative_path,
                mode: metadata_mode(&first.metadata),
                contents: Some(contents),
            });
        }
    }
    Ok(())
}
