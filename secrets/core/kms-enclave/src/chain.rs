//! Decrypt-only key-version rotation, on the AWS KMS version-rotation
//! precedent.
//!
//! Rotation creates a NEW key version that encrypts forward; every prior
//! version is demoted to [`DecryptOnlyKek`], a type with no wrap API, so
//! "encrypt with a retired version" is unrepresentable. Existing ciphertext
//! is never re-encrypted — the rejected anti-pattern is re-encrypt-on-rotate.

use std::collections::BTreeMap;

use secrets_kms_domain::envelope_keys::{DekId, KekId};

use crate::EnclaveError;
use crate::material::{DekMaterial, KekMaterial, KekVersion};
use crate::token::WrappedDek;

/// A retired KEK version. Exposes unwrapping only — the absence of any wrap
/// method is the enforcement, not a runtime check.
pub struct DecryptOnlyKek {
    inner: KekMaterial,
}

impl DecryptOnlyKek {
    pub fn version(&self) -> KekVersion {
        self.inner.version()
    }

    pub fn unwrap_dek(&self, wrapped: &WrappedDek) -> Result<DekMaterial, EnclaveError> {
        self.inner.unwrap_dek(wrapped)
    }
}

impl std::fmt::Debug for DecryptOnlyKek {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DecryptOnlyKek {{ version: {}, key: [REDACTED] }}",
            self.version()
        )
    }
}

/// All live versions of one tenant KEK: a single current (encrypt-capable)
/// version plus retired decrypt-only versions.
pub struct KekVersionChain {
    current: KekMaterial,
    retired: BTreeMap<u32, DecryptOnlyKek>,
}

impl KekVersionChain {
    pub fn new(initial: KekMaterial) -> Self {
        Self {
            current: initial,
            retired: BTreeMap::new(),
        }
    }

    pub fn kek_id(&self) -> &KekId {
        self.current.kek_id()
    }

    pub fn current_version(&self) -> KekVersion {
        self.current.version()
    }

    pub fn retired_versions(&self) -> impl Iterator<Item = KekVersion> + '_ {
        self.retired.values().map(DecryptOnlyKek::version)
    }

    /// Borrow the current encrypt-capable KEK (e.g. for re-sealing under a
    /// root after rotation).
    pub fn current(&self) -> &KekMaterial {
        &self.current
    }

    /// Rotate: generate fresh material at the next version; the previous
    /// current version is demoted to decrypt-only. Returns the new version.
    pub fn rotate(&mut self) -> Result<KekVersion, EnclaveError> {
        let next_version = self.current.version().next()?;
        let next = KekMaterial::generate(self.current.kek_id().clone(), next_version)?;
        let previous = std::mem::replace(&mut self.current, next);
        self.retired.insert(
            previous.version().value(),
            DecryptOnlyKek { inner: previous },
        );
        Ok(next_version)
    }

    pub fn generate_dek(&self, dek_id: DekId) -> Result<(DekMaterial, WrappedDek), EnclaveError> {
        self.current.generate_dek(dek_id)
    }

    /// Unwrap a DEK wrapped by any version this chain holds; routing is by
    /// the version the wrapped DEK's authenticated header carries.
    pub fn unwrap_dek(&self, wrapped: &WrappedDek) -> Result<DekMaterial, EnclaveError> {
        if wrapped.kek_id() != self.kek_id() {
            return Err(EnclaveError::KeyBindingMismatch {
                expected: self.kek_id().value().to_owned(),
                found: wrapped.kek_id().value().to_owned(),
            });
        }
        if wrapped.kek_version() == self.current.version().value() {
            return self.current.unwrap_dek(wrapped);
        }
        match self.retired.get(&wrapped.kek_version()) {
            Some(retired) => retired.unwrap_dek(wrapped),
            None => Err(EnclaveError::UnknownKekVersion {
                version: wrapped.kek_version(),
            }),
        }
    }
}

impl std::fmt::Debug for KekVersionChain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "KekVersionChain {{ kek_id: {}, current: {}, retired: {} versions, keys: [REDACTED] }}",
            self.kek_id(),
            self.current_version(),
            self.retired.len()
        )
    }
}
