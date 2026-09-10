//! In-memory `SecretStorePort`.
//!
//! NOT FOR PRODUCTION: an in-process map whose lifetime is the process, with
//! no encryption at rest and no persistence across a restart.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use std::collections::HashMap;

use intelligence_account_domain::{
    SecretMaterial, SecretReference, SecretStoreError, SecretStorePort,
};

#[derive(Default, Clone)]
pub struct InMemorySecretStoreAdapter {
    store: HashMap<SecretReference, SecretMaterial>,
}

impl InMemorySecretStoreAdapter {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SecretStorePort for InMemorySecretStoreAdapter {
    fn put(
        &mut self,
        sref: &SecretReference,
        material: SecretMaterial,
    ) -> Result<(), SecretStoreError> {
        if material.is_empty() {
            return Err(SecretStoreError::Backend("empty material rejected".into()));
        }
        self.store.insert(sref.clone(), material);
        Ok(())
    }

    fn get(&self, sref: &SecretReference) -> Result<SecretMaterial, SecretStoreError> {
        self.store
            .get(sref)
            .cloned()
            .ok_or(SecretStoreError::NotFound)
    }

    fn rotate(
        &mut self,
        sref: &SecretReference,
        new_material: SecretMaterial,
    ) -> Result<(), SecretStoreError> {
        if !self.store.contains_key(sref) {
            return Err(SecretStoreError::NotFound);
        }
        self.store.insert(sref.clone(), new_material);
        Ok(())
    }

    fn delete(&mut self, sref: &SecretReference) -> Result<(), SecretStoreError> {
        self.store
            .remove(sref)
            .map(|_| ())
            .ok_or(SecretStoreError::NotFound)
    }
}

pub fn store_secret_reference(
    adapter: &mut InMemorySecretStoreAdapter,
    sref: &SecretReference,
    material: SecretMaterial,
) -> Result<(), SecretStoreError> {
    adapter.put(sref, material)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sref(s: &str) -> SecretReference {
        SecretReference::new(s.to_owned()).unwrap()
    }
    fn mat(bytes: &[u8]) -> SecretMaterial {
        SecretMaterial::new(bytes.to_vec())
    }

    #[test]
    fn put_then_get_roundtrip() {
        let mut a = InMemorySecretStoreAdapter::new();
        let r = sref("sref://aws-prod-key");
        a.put(&r, mat(b"top-secret-key-material")).unwrap();
        let got = a.get(&r).unwrap();
        assert_eq!(got.expose_for_provider_call(), b"top-secret-key-material");
    }

    #[test]
    fn get_missing_returns_not_found() {
        let a = InMemorySecretStoreAdapter::new();
        assert_eq!(
            a.get(&sref("sref://missing")),
            Err(SecretStoreError::NotFound)
        );
    }

    #[test]
    fn put_empty_material_rejected() {
        let mut a = InMemorySecretStoreAdapter::new();
        assert!(a.put(&sref("sref://k1"), mat(b"")).is_err());
    }

    #[test]
    fn rotate_replaces_material() {
        let mut a = InMemorySecretStoreAdapter::new();
        let r = sref("sref://gemini-key");
        a.put(&r, mat(b"old-key")).unwrap();
        a.rotate(&r, mat(b"new-key")).unwrap();
        assert_eq!(a.get(&r).unwrap().expose_for_provider_call(), b"new-key");
    }

    #[test]
    fn rotate_missing_returns_not_found() {
        let mut a = InMemorySecretStoreAdapter::new();
        assert_eq!(
            a.rotate(&sref("sref://gone"), mat(b"x")),
            Err(SecretStoreError::NotFound)
        );
    }

    #[test]
    fn delete_removes_entry() {
        let mut a = InMemorySecretStoreAdapter::new();
        let r = sref("sref://disposable");
        a.put(&r, mat(b"secret-value")).unwrap();
        a.delete(&r).unwrap();
        assert_eq!(a.get(&r), Err(SecretStoreError::NotFound));
    }

    #[test]
    fn delete_missing_returns_not_found() {
        let mut a = InMemorySecretStoreAdapter::new();
        assert_eq!(
            a.delete(&sref("sref://gone")),
            Err(SecretStoreError::NotFound)
        );
    }

    #[test]
    fn material_debug_is_redacted() {
        let m = mat(b"my-actual-secret-bytes");
        let dbg = format!("{m:?}");
        assert!(dbg.contains("[REDACTED"));
        assert!(!dbg.contains("my-actual-secret"));
    }

    #[test]
    fn sref_debug_is_redacted_in_store_too() {
        let r = sref("sref://very-secret-key-id");
        let dbg = format!("{r:?}");
        assert!(dbg.contains("[REDACTED]"));
        assert!(!dbg.contains("very-secret"));
    }

    #[test]
    fn store_secret_reference_fn() {
        let mut a = InMemorySecretStoreAdapter::new();
        let r = sref("sref://test-key");
        store_secret_reference(&mut a, &r, mat(b"value")).unwrap();
        assert_eq!(a.get(&r).unwrap().expose_for_provider_call(), b"value");
    }

    #[test]
    fn two_distinct_secrets_do_not_collide() {
        // Two secrets at once: `SecretReference::Debug` redacts to the same
        // string for every reference, so any map keyed on it collapses them.
        let mut a = InMemorySecretStoreAdapter::new();
        let aws = sref("sref://openbao/aws-prod-key");
        let gemini = sref("sref://openbao/gemini-prod-key");
        a.put(&aws, mat(b"aws-secret-bytes")).unwrap();
        a.put(&gemini, mat(b"gemini-secret-bytes")).unwrap();
        assert_eq!(
            a.get(&aws).unwrap().expose_for_provider_call(),
            b"aws-secret-bytes",
            "lookup for aws sref must return aws material, not the most recent put"
        );
        assert_eq!(
            a.get(&gemini).unwrap().expose_for_provider_call(),
            b"gemini-secret-bytes",
            "lookup for gemini sref must return gemini material"
        );
    }

    #[test]
    fn delete_one_does_not_affect_another() {
        let mut a = InMemorySecretStoreAdapter::new();
        let kept = sref("sref://openbao/kept");
        let dropped = sref("sref://openbao/dropped");
        a.put(&kept, mat(b"kept-material")).unwrap();
        a.put(&dropped, mat(b"dropped-material")).unwrap();
        a.delete(&dropped).unwrap();
        assert_eq!(
            a.get(&kept).unwrap().expose_for_provider_call(),
            b"kept-material"
        );
        assert_eq!(a.get(&dropped), Err(SecretStoreError::NotFound));
    }
}
