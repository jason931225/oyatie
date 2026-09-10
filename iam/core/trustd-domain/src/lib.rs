//! Rust model of the Talos (`siderolabs/talos`) `trustd` subsystem: the cluster
//! certificate authority, PKI primitives, certificate issuance and signing, the
//! `SecurityService` API surface, join-token verification, and node certificate
//! rotation.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(
    clippy::must_use_candidate,
    clippy::return_self_not_must_use,
    clippy::missing_errors_doc
)]

pub mod bundle;
pub mod ca;
pub mod certificate;
pub mod crl;
pub mod der;
pub mod error;
pub mod persistence;
pub mod service;
pub mod signer;
pub mod token;
pub mod x509;

pub use bundle::TrustBundle;
pub use ca::{CertificateAuthority, CertificateSigningRequest, IssuancePolicy};
pub use certificate::{CertUsage, Certificate, IssuedIdentity};
pub use crl::{RevocationEntry, RevocationList, RevocationReason};
pub use der::{IssuedDer, encode_ca_der, encode_leaf_der, issue_der};
pub use error::{Result, TrustError};
pub use persistence::KeySealer;
pub use service::{CertificateRequest, CertificateResponse, SecurityService};
#[cfg(any(test, feature = "modeled-crypto"))]
pub use signer::InMemorySigner;
pub use signer::{EcdsaP256Signer, SigningBackend};
pub use token::JoinToken;
pub use x509::{DistinguishedName, KeyPair, PEMEncoded, PEMLabel, SubjectAltNames, Validity};

#[cfg(test)]
mod integration_tests {
    use super::*;
    use os_kernel::Role;

    #[test]
    fn cargo_manifest_declares_no_default_feature() {
        let features: Vec<&str> = include_str!("../Cargo.toml")
            .lines()
            .skip_while(|l| l.trim() != "[features]")
            .skip(1)
            .take_while(|l| !l.trim_start().starts_with('['))
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect();

        assert_eq!(
            features,
            ["modeled-crypto = []"],
            "Cargo.toml [features] must not declare `default`: modeled-crypto \
             is non-default and that is the whole barrier"
        );
    }

    #[test]
    fn no_buck_target_enables_the_model() {
        let buck: String = include_str!("../BUCK")
            .lines()
            .filter(|l| !l.trim_start().starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n");

        assert!(
            buck.contains("os-trustd-domain"),
            "read the wrong BUCK file: a zero-count assertion below would pass vacuously"
        );
        assert_eq!(
            buck.matches("modeled-crypto").count(),
            0,
            "no buck2 target in this package may enable modeled-crypto: the crate gate \
             is cfg(any(test, feature = \"modeled-crypto\")), so the rust_test already \
             sees the modeled items via `test`, and the only thing a feature attribute \
             here could do is put them in a production library"
        );
    }

    #[test]
    fn bootstrap_join_authorize_and_renew() {
        let token = JoinToken::new("kube01.s3cr3ttoken99").unwrap();
        let ca = CertificateAuthority::bootstrap(
            "talos-ca",
            KeyPair::from_seed(b"ca-root-key"),
            InMemorySigner::from_seed("ca-root-key"),
            1_000,
            10_000_000,
        )
        .unwrap();
        let mut svc = SecurityService::new(token, ca);

        let node_key = KeyPair::from_seed(b"worker-1-key");
        let csr = CertificateSigningRequest::for_node(
            "worker-1",
            &node_key,
            CertUsage::ClientAuth,
            3_600,
        )
        .requesting_role("os:reader")
        .with_dns("worker-1.cluster.local");

        let req = CertificateRequest {
            join_token: "kube01.s3cr3ttoken99".to_string(),
            csr,
        };
        let resp = svc.handle_certificate(&req, &node_key, 2_000).unwrap();
        let cert = resp.identity.certificate.clone();

        assert!(svc.require_role(&cert, Role::Reader, 2_500).is_ok());
        assert!(svc.require_role(&cert, Role::Admin, 2_500).is_err());

        assert!(cert.sans.covers_dns("worker-1.cluster.local"));

        let renewed = svc
            .renew_if_needed(&cert, &node_key, 5_000)
            .unwrap()
            .expect("should renew near expiry");
        assert!(renewed.certificate.serial > cert.serial);
        assert!(
            svc.require_role(&renewed.certificate, Role::Reader, 5_500)
                .is_ok()
        );
    }

    #[test]
    fn wrong_token_blocks_issuance() {
        let token = JoinToken::new("kube01.s3cr3ttoken99").unwrap();
        let ca = CertificateAuthority::bootstrap(
            "talos-ca",
            KeyPair::from_seed(b"ca-root-key"),
            InMemorySigner::from_seed("ca-root-key"),
            1_000,
            10_000_000,
        )
        .unwrap();
        let mut svc = SecurityService::new(token, ca);
        let node_key = KeyPair::from_seed(b"worker-1-key");
        let csr = CertificateSigningRequest::for_node(
            "worker-1",
            &node_key,
            CertUsage::ClientAuth,
            3_600,
        );
        let req = CertificateRequest {
            join_token: "kube01.WRONGTOKEN99".to_string(),
            csr,
        };
        assert_eq!(
            svc.handle_certificate(&req, &node_key, 2_000)
                .unwrap_err()
                .kind(),
            "token_mismatch"
        );
    }
}
