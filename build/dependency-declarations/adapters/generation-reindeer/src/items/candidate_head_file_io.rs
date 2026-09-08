const FILE_BUFFER_BYTES: usize = 64 * 1024;
fn compare_published_output(
    root: CandidateRoot,
    path: &Path,
    generated: &[u8],
) -> Result<(), CandidateHeadQualificationFailure> {
    let metadata = fs::metadata(path).map_err(|error| {
        CandidateHeadQualificationFailure::CandidateTreeRead {
            root,
            scope: CandidateTreeScope::Semantic,
            path: path.to_path_buf(),
            kind: error.kind(),
        }
    })?;
    let published_bytes = usize::try_from(metadata.len()).unwrap_or(usize::MAX);
    let mut published =
        File::open(path).map_err(
            |error| CandidateHeadQualificationFailure::CandidateTreeRead {
                root,
                scope: CandidateTreeScope::Semantic,
                path: path.to_path_buf(),
                kind: error.kind(),
            },
        )?;
    let mut offset = 0_usize;
    let mut buffer = [0_u8; FILE_BUFFER_BYTES];
    loop {
        let count = published.read(&mut buffer).map_err(|error| {
            CandidateHeadQualificationFailure::CandidateTreeRead {
                root,
                scope: CandidateTreeScope::Semantic,
                path: path.to_path_buf(),
                kind: error.kind(),
            }
        })?;
        if count == 0 {
            break;
        }
        let available = generated.len().saturating_sub(offset);
        let compared = cmp::min(count, available);
        if let Some(difference) = buffer[..compared]
            .iter()
            .zip(&generated[offset..offset + compared])
            .position(|(published, generated)| published != generated)
        {
            return Err(CandidateHeadQualificationFailure::PublishedOutputMismatch {
                root,
                generated_bytes: generated.len(),
                published_bytes,
                first_difference: offset + difference,
            });
        }
        offset = offset.saturating_add(count);
        if count > available {
            break;
        }
    }
    if offset == generated.len() && published_bytes == generated.len() {
        Ok(())
    } else {
        Err(CandidateHeadQualificationFailure::PublishedOutputMismatch {
            root,
            generated_bytes: generated.len(),
            published_bytes,
            first_difference: cmp::min(offset, generated.len()),
        })
    }
}

fn compare_regular_files(
    first: &Path,
    second: &Path,
    relative_path: &Path,
    scope: CandidateTreeScope,
    maximum: u64,
) -> Result<Vec<u8>, CandidateHeadQualificationFailure> {
    let mut first_file = File::open(first)
        .map_err(|error| tree_read_error(CandidateRoot::First, scope, relative_path, error))?;
    let mut second_file = File::open(second)
        .map_err(|error| tree_read_error(CandidateRoot::Second, scope, relative_path, error))?;
    let mut first_buffer = [0_u8; FILE_BUFFER_BYTES];
    let mut second_buffer = [0_u8; FILE_BUFFER_BYTES];
    let mut contents = Vec::new();
    loop {
        let first_count = first_file
            .read(&mut first_buffer)
            .map_err(|error| tree_read_error(CandidateRoot::First, scope, relative_path, error))?;
        let second_count = second_file
            .read(&mut second_buffer)
            .map_err(|error| tree_read_error(CandidateRoot::Second, scope, relative_path, error))?;
        if first_count != second_count
            || first_buffer[..first_count] != second_buffer[..second_count]
        {
            return Err(CandidateHeadQualificationFailure::CandidateTreesDiffer {
                scope,
                path: relative_path.to_path_buf(),
            });
        }
        if first_count == 0 {
            return Ok(contents);
        }
        if (contents.len() as u64).saturating_add(first_count as u64) > maximum {
            return Err(
                CandidateHeadQualificationFailure::CandidateTreeLimitExceeded {
                    root: CandidateRoot::First,
                    scope,
                    limit: QualificationLimit::TreeFileBytes,
                    value: (contents.len() as u64).saturating_add(first_count as u64),
                    maximum,
                },
            );
        }
        contents.extend_from_slice(&first_buffer[..first_count]);
    }
}

fn read_regular_file(
    root: CandidateRoot,
    scope: CandidateTreeScope,
    path: &Path,
    relative_path: &Path,
    maximum: u64,
) -> Result<Vec<u8>, CandidateHeadQualificationFailure> {
    let mut file =
        File::open(path).map_err(|error| tree_read_error(root, scope, relative_path, error))?;
    let mut buffer = [0_u8; FILE_BUFFER_BYTES];
    let mut contents = Vec::new();
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| tree_read_error(root, scope, relative_path, error))?;
        if count == 0 {
            return Ok(contents);
        }
        if (contents.len() as u64).saturating_add(count as u64) > maximum {
            return Err(
                CandidateHeadQualificationFailure::CandidateTreeLimitExceeded {
                    root,
                    scope,
                    limit: QualificationLimit::TreeFileBytes,
                    value: (contents.len() as u64).saturating_add(count as u64),
                    maximum,
                },
            );
        }
        contents.extend_from_slice(&buffer[..count]);
    }
}
