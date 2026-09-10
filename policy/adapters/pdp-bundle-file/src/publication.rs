use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use shared_audit_event_kernel::{ChainSigner, DigestChainError, encode_hex};
use shared_pdp_kernel::PolicyBundle;

use super::{
    BundleSignature, BundleStoreError, FilePolicyBundleStore, SignedPolicyBundleDoc,
    load_trust_anchor, parse_signed_bundle,
};

/// Publication failure, including whether replacement has already committed.
#[derive(Debug)]
pub enum BundlePublishError {
    Serialization(serde_json::Error),
    Signing(DigestChainError),
    Verification(BundleStoreError),
    BeforeCommit {
        operation: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    StagingCleanupFailed {
        path: PathBuf,
        source: io::Error,
        publication_error: Box<BundlePublishError>,
    },
    CommittedButDurabilityUnknown {
        path: PathBuf,
        source: io::Error,
    },
}

impl fmt::Display for BundlePublishError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Serialization(error) => write!(formatter, "bundle serialization failed: {error}"),
            Self::Signing(error) => write!(formatter, "bundle signing failed: {error}"),
            Self::Verification(error) => write!(formatter, "bundle verification failed: {error}"),
            Self::BeforeCommit {
                operation,
                path,
                source,
            } => write!(
                formatter,
                "bundle publication before commit: {operation} {}: {source}",
                path.display()
            ),
            Self::StagingCleanupFailed {
                path,
                source,
                publication_error,
            } => write!(
                formatter,
                "{publication_error}; staging cleanup failed at {}: {source}",
                path.display()
            ),
            Self::CommittedButDurabilityUnknown { path, source } => write!(
                formatter,
                "bundle replacement committed at {}; directory durability unknown: {source}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for BundlePublishError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            Self::Serialization(error) => error,
            Self::Signing(error) => error,
            Self::Verification(error) => error,
            Self::BeforeCommit { source, .. }
            | Self::StagingCleanupFailed { source, .. }
            | Self::CommittedButDurabilityUnknown { source, .. } => source,
        })
    }
}

impl FilePolicyBundleStore {
    /// The public key is envelope metadata; it never registers trust. The
    /// destination directory must exist.
    ///
    /// # Errors
    /// A pre-commit error leaves the destination unchanged. A successful rename
    /// followed by directory-sync failure returns
    /// [`BundlePublishError::CommittedButDurabilityUnknown`]; the new file is
    /// already visible and the caller cannot assume rollback.
    pub fn write_signed_bundle(
        &self,
        bundle: &PolicyBundle,
        signer: &dyn ChainSigner,
        public_key: &[u8],
    ) -> Result<(), BundlePublishError> {
        let verifier =
            load_trust_anchor(self.trust_dir()).map_err(BundlePublishError::Verification)?;
        let inner = serde_json::to_string(bundle).map_err(BundlePublishError::Serialization)?;
        let signature_hex = signer
            .sign_hex(inner.as_bytes())
            .map_err(BundlePublishError::Signing)?;
        let document = SignedPolicyBundleDoc {
            bundle: inner,
            signatures: vec![BundleSignature {
                key_id: signer.key_id().to_owned(),
                public_key_hex: encode_hex(public_key),
                signature_hex,
            }],
        };
        let serialized =
            serde_json::to_string(&document).map_err(BundlePublishError::Serialization)?;
        parse_signed_bundle(&serialized, &verifier).map_err(BundlePublishError::Verification)?;
        replace_atomically(self.path(), serialized.as_bytes(), File::sync_all)
    }
}

fn before_commit(operation: &'static str, path: &Path, source: io::Error) -> BundlePublishError {
    BundlePublishError::BeforeCommit {
        operation,
        path: path.to_owned(),
        source,
    }
}

fn create_staging(directory: &Path) -> Result<(PathBuf, File), BundlePublishError> {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    for _ in 0..64 {
        let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = directory.join(format!(
            ".policy-bundle-stage-{}-{sequence}.tmp",
            std::process::id()
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(before_commit("create staging file", &path, error)),
        }
    }
    Err(before_commit(
        "create staging file",
        directory,
        io::Error::new(io::ErrorKind::AlreadyExists, "staging names exhausted"),
    ))
}

pub(super) fn replace_atomically(
    destination: &Path,
    bytes: &[u8],
    sync: impl Fn(&File) -> io::Result<()>,
) -> Result<(), BundlePublishError> {
    if destination.file_name().is_none() {
        return Err(before_commit(
            "resolve destination",
            destination,
            io::Error::new(io::ErrorKind::InvalidInput, "destination needs a file name"),
        ));
    }
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let directory =
        File::open(parent).map_err(|error| before_commit("open directory", parent, error))?;
    let (staging_path, mut staging) = create_staging(parent)?;
    let written = staging
        .write_all(bytes)
        .map_err(|error| before_commit("write staging file", &staging_path, error))
        .and_then(|()| {
            sync(&staging).map_err(|error| before_commit("sync staging file", &staging_path, error))
        });
    drop(staging);
    let committed = written.and_then(|()| {
        fs::rename(&staging_path, destination)
            .map_err(|error| before_commit("replace destination", destination, error))
    });
    if let Err(error) = committed {
        return match fs::remove_file(&staging_path) {
            Ok(()) => Err(error),
            Err(cleanup) if cleanup.kind() == io::ErrorKind::NotFound => Err(error),
            Err(source) => Err(BundlePublishError::StagingCleanupFailed {
                path: staging_path,
                source,
                publication_error: Box::new(error),
            }),
        };
    }
    sync(&directory).map_err(|source| BundlePublishError::CommittedButDurabilityUnknown {
        path: destination.to_owned(),
        source,
    })
}
