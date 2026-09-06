#[derive(Default)]
struct IndependentStorage {
    first: std::collections::BTreeSet<FileIdentity>,
    second: std::collections::BTreeSet<FileIdentity>,
}

impl IndependentStorage {
    fn observe(
        &mut self,
        first: &Metadata,
        second: &Metadata,
        scope: CandidateTreeScope,
        path: &Path,
    ) -> Result<(), CandidateHeadQualificationFailure> {
        let first = file_identity(first);
        let second = file_identity(second);
        if first == second || self.second.contains(&first) || self.first.contains(&second) {
            return Err(
                CandidateHeadQualificationFailure::CandidateTreesShareStorage {
                    scope,
                    path: path.to_path_buf(),
                },
            );
        }
        self.first.insert(first);
        self.second.insert(second);
        Ok(())
    }
}
