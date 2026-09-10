//! Transitional sealing-root custody via OpenBao.
//!
//! Custody design: OpenBao GENERATES and custodies the per-cell sealing root
//! as an exportable transit key — ceremony tooling never passes key material
//! into a request body. At boot the enclave fetches the export and ingests
//! it through [`EnclaveRoot::from_key_bytes`], the kernel's one-way ingress:
//! material flows OpenBao → locked enclave memory and never back out.
//!
//! Like the sibling transit adapter, this module builds provider request
//! SHAPES and parses provider material strictly — it performs no network
//! I/O.

use std::fmt;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use secrets_kms_enclave::{EnclaveError, EnclaveRoot, RootProvenance, SealingRootId};
use zeroize::Zeroize;

use crate::{
    OpenBaoKmsAdapterConfigError, canonical_body, origin_without_scheme, validate_endpoint,
    validate_path_segment,
};

fn import_root_key<E>(
    source: &mut [u8; 32],
    import: impl FnOnce([u8; 32]) -> Result<EnclaveRoot, E>,
) -> Result<EnclaveRoot, E> {
    // Arrays are Copy, so the by-value importer can only wipe its copy.
    // Scrub this caller-owned buffer before either outcome escapes.
    let result = import(*source);
    source.zeroize();
    result
}

#[derive(Debug)]
pub enum RootCustodyError {
    Config(OpenBaoKmsAdapterConfigError),
    MaterialNotBase64,
    MaterialWrongLength { got: usize },
    Enclave(EnclaveError),
}

impl fmt::Display for RootCustodyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(_) => f.write_str("root custody: invalid adapter configuration"),
            Self::MaterialNotBase64 => {
                f.write_str("root custody: exported material is not valid base64")
            }
            Self::MaterialWrongLength { got } => {
                write!(
                    f,
                    "root custody: exported material is {got} bytes, expected 32"
                )
            }
            Self::Enclave(err) => write!(f, "root custody: enclave refused material: {err}"),
        }
    }
}

impl std::error::Error for RootCustodyError {}

impl From<EnclaveError> for RootCustodyError {
    fn from(err: EnclaveError) -> Self {
        Self::Enclave(err)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenBaoCustodyCommand {
    pub method: &'static str,
    pub path: String,
    pub namespace: Option<String>,
    pub body_canonical: String,
    pub audit_evidence_ref: String,
}

/// Builder for sealing-root custody commands against one OpenBao transit
/// mount + key. One custodian per cell sealing root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenBaoRootCustody {
    endpoint_origin: String,
    transit_mount: String,
    key_name: String,
    namespace: Option<String>,
}

impl OpenBaoRootCustody {
    pub fn new(
        endpoint_origin: impl Into<String>,
        transit_mount: impl Into<String>,
        key_name: impl Into<String>,
    ) -> Result<Self, RootCustodyError> {
        let endpoint_origin = endpoint_origin.into();
        let transit_mount = transit_mount.into();
        let key_name = key_name.into();
        validate_endpoint(&endpoint_origin).map_err(RootCustodyError::Config)?;
        validate_path_segment(
            &transit_mount,
            OpenBaoKmsAdapterConfigError::InvalidTransitMount,
        )
        .map_err(RootCustodyError::Config)?;
        validate_path_segment(&key_name, OpenBaoKmsAdapterConfigError::InvalidKeyName)
            .map_err(RootCustodyError::Config)?;
        Ok(Self {
            endpoint_origin,
            transit_mount,
            key_name,
            namespace: None,
        })
    }

    pub fn with_namespace(
        mut self,
        namespace: impl Into<String>,
    ) -> Result<Self, RootCustodyError> {
        let namespace = namespace.into();
        validate_path_segment(
            &namespace,
            OpenBaoKmsAdapterConfigError::InvalidTransitMount,
        )
        .map_err(RootCustodyError::Config)?;
        self.namespace = Some(namespace);
        Ok(self)
    }

    /// Ceremony-time command: have OpenBao generate the sealing root as an
    /// exportable AES-256 transit key. No key material crosses the wire in
    /// this request; OpenBao is the generator and custodian (ADR-0510).
    pub fn provision_root_command(&self, ceremony_evidence_ref: &str) -> OpenBaoCustodyCommand {
        OpenBaoCustodyCommand {
            method: "POST",
            path: format!("/v1/{}/keys/{}", self.transit_mount, self.key_name),
            namespace: self.namespace.clone(),
            body_canonical: canonical_body(&[
                ("type", "aes256-gcm96"),
                ("exportable", "true"),
                ("allow_plaintext_backup", "false"),
                ("ceremony_evidence_ref", ceremony_evidence_ref),
            ]),
            audit_evidence_ref: self.audit_ref("provision"),
        }
    }

    /// Boot-time command: fetch version 1 of the exported root key. The
    /// response material must be handed to [`Self::ingest_exported_root`]
    /// immediately and never persisted by the caller.
    pub fn fetch_root_export_command(&self) -> OpenBaoCustodyCommand {
        OpenBaoCustodyCommand {
            method: "GET",
            path: format!(
                "/v1/{}/export/encryption-key/{}/1",
                self.transit_mount, self.key_name
            ),
            namespace: self.namespace.clone(),
            body_canonical: String::new(),
            audit_evidence_ref: self.audit_ref("export"),
        }
    }

    /// Ingest exported root material through the enclave one-way door. The
    /// base64 input and every intermediate buffer are zeroized; on success
    /// the only holder of the root is the returned [`EnclaveRoot`].
    pub fn ingest_exported_root(
        &self,
        root_id: SealingRootId,
        mut exported_key_base64: String,
        ceremony_evidence_ref: &str,
    ) -> Result<(EnclaveRoot, RootProvenance), RootCustodyError> {
        let decoded = BASE64_STANDARD.decode(exported_key_base64.trim());
        exported_key_base64.zeroize();
        let mut decoded = decoded.map_err(|_| RootCustodyError::MaterialNotBase64)?;
        if decoded.len() != 32 {
            let got = decoded.len();
            decoded.zeroize();
            return Err(RootCustodyError::MaterialWrongLength { got });
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&decoded);
        decoded.zeroize();
        let root = import_root_key(&mut bytes, |candidate| {
            EnclaveRoot::from_key_bytes(root_id, candidate)
        })?;
        let provenance = RootProvenance::OpenBaoTransitionalSingleCustodian {
            ceremony_evidence_ref: ceremony_evidence_ref.to_owned(),
        };
        Ok((root, provenance))
    }

    fn audit_ref(&self, operation: &str) -> String {
        format!(
            "openbao-root-custody://{}/{}/{}/{}",
            origin_without_scheme(&self.endpoint_origin),
            self.transit_mount,
            self.key_name,
            operation
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{EnclaveRoot, SealingRootId, import_root_key};

    #[test]
    fn import_root_key_zeroizes_source_after_success() {
        let mut source = [0x5a; 32];
        let root_id = SealingRootId::new("test-root").expect("root id");
        let result = import_root_key(&mut source, |candidate| {
            assert_eq!(candidate, [0x5a; 32]);
            EnclaveRoot::from_key_bytes(root_id, candidate)
        });

        assert_eq!(result.expect("root").root_id().value(), "test-root");
        assert_eq!(source, [0; 32]);
    }

    #[test]
    fn import_root_key_zeroizes_source_after_failure() {
        let mut source = [0xa5; 32];
        let result = import_root_key(&mut source, |_| Err::<EnclaveRoot, _>("ingress refused"));

        assert_eq!(result.expect_err("ingress must fail"), "ingress refused");
        assert_eq!(source, [0; 32]);
    }
}
