//! Agreed cross-owner surface for KMS reference and policy values.
//!
//! Consumers receive Secrets' established exact value identities, typed
//! validation errors, and validation behavior without importing its internal
//! aggregate. This port intentionally excludes key material, provider clients,
//! cryptographic operations, and the KMS directory.

#![forbid(unsafe_code)]

pub use secrets_kms_domain::{
    CiphertextRef, CloudKmsError, DestructionProofRef, KmsKeyId, KmsKeyOrigin, KmsPurpose,
    KmsUseEventId, MaterialRef,
};
