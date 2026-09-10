//! Workload X.509-SVID trustd adapter over the os-trustd-domain CA (ADR-0561).
//! ADR-0083 Tier-3 is the provenance for the cfg(test) lint exemption below.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use os_trustd_domain::ca::CertificateSigningRequest;
use os_trustd_domain::certificate::Certificate;
use os_trustd_domain::der;
use os_trustd_domain::service::{CertificateRequest, SecurityService};
use os_trustd_domain::signer::EcdsaP256Signer;
use os_trustd_domain::x509::KeyPair;
use os_trustd_domain::{TrustBundle, TrustError};

use iam_identity_workload_svid_kernel::{
    IssueError, SpiffeId, SvidRequest, SvidVerifier, VerifyError, WorkloadIdentityIssuer, X509Svid,
};

pub mod leaf_der;

use leaf_der::LeafVerifyError;

/// Issues workload SVIDs over a trustd [`SecurityService`] with a real ECDSA-P256 CA signer.
pub struct TrustdSvidIssuer<'a> {
    service: &'a mut SecurityService<EcdsaP256Signer>,
    join_token: String,
    workload_signer: EcdsaP256Signer,
    ca_signer: EcdsaP256Signer,
    workload_name: String,
}

impl<'a> TrustdSvidIssuer<'a> {
    pub fn new(
        service: &'a mut SecurityService<EcdsaP256Signer>,
        join_token: impl Into<String>,
        workload_signer: EcdsaP256Signer,
        ca_signer: EcdsaP256Signer,
        workload_name: impl Into<String>,
    ) -> Self {
        Self {
            service,
            join_token: join_token.into(),
            workload_signer,
            ca_signer,
            workload_name: workload_name.into(),
        }
    }

    /// Issue an X.509-SVID for `request` as of `now`.
    ///
    /// # Errors
    /// [`IssueError`] when trustd refuses issuance or real-DER minting fails.
    pub fn issue(&mut self, request: &SvidRequest, now: u64) -> Result<X509Svid, IssueError> {
        let requester_key = KeyPair::new(
            self.workload_signer.private_key_der(),
            self.workload_signer.public_key_spki_der(),
        );
        let csr = CertificateSigningRequest::for_workload(
            self.workload_name.clone(),
            request.spiffe_id.as_uri(),
            &requester_key,
            request.ttl_secs,
        );
        let cert_request = CertificateRequest {
            join_token: self.join_token.clone(),
            csr,
        };
        let response = self
            .service
            .handle_certificate(&cert_request, &requester_key, now)
            .map_err(|err| IssueError::new(err.to_string()))?;
        let cert = response.identity.certificate;
        let leaf_der = der::encode_leaf_der(
            &cert,
            &self.workload_signer,
            self.service.ca_certificate(),
            &self.ca_signer,
        )
        .map_err(|err| IssueError::new(format!("real DER issuance failed: {err}")))?;
        Ok(X509Svid {
            spiffe_id: request.spiffe_id.clone(),
            leaf_der,
        })
    }
}

impl WorkloadIdentityIssuer for TrustdSvidIssuer<'_> {
    fn issue_x509_svid(&self, _request: &SvidRequest, _now: u64) -> Result<X509Svid, IssueError> {
        Err(IssueError::new(
            "use TrustdSvidIssuer::issue (the trait's &self shape cannot advance the CA serial)",
        ))
    }
}

/// Verifies presented peer SVIDs (real X.509 DER) against a trustd [`TrustBundle`].
pub struct TrustdSvidVerifier<'a, S: os_trustd_domain::signer::SigningBackend> {
    bundle: &'a TrustBundle<S>,
}

impl<'a, S: os_trustd_domain::signer::SigningBackend> TrustdSvidVerifier<'a, S> {
    pub fn new(bundle: &'a TrustBundle<S>) -> Self {
        Self { bundle }
    }

    /// Verify a decoded trustd [`Certificate`] and return its SPIFFE id.
    ///
    /// # Errors
    /// [`VerifyError`] on an untrusted/expired chain or a bad/absent URI SAN.
    pub fn verify_certificate(
        &self,
        cert: &Certificate,
        now: u64,
    ) -> Result<SpiffeId, VerifyError> {
        self.bundle
            .verify_leaf(cert, now)
            .map_err(|err| match err {
                TrustError::Expired(_) => VerifyError::Expired,
                other => VerifyError::UntrustedIssuer {
                    detail: other.to_string(),
                },
            })?;
        match cert.sans.uris.as_slice() {
            [] => Err(VerifyError::NoSpiffeUriSan),
            [uri] => SpiffeId::parse(uri).map_err(VerifyError::MalformedSpiffeId),
            _ => Err(VerifyError::AmbiguousUriSan),
        }
    }
}

impl<S: os_trustd_domain::signer::SigningBackend> SvidVerifier for TrustdSvidVerifier<'_, S> {
    fn verify_peer(&self, leaf_der: &[u8], now: u64) -> Result<SpiffeId, VerifyError> {
        let trusted = self.bundle.trusted_ca_spki_ders();
        let uri = leaf_der::verify_leaf_der(leaf_der, &trusted, now).map_err(map_leaf_err)?;
        SpiffeId::parse(&uri).map_err(VerifyError::MalformedSpiffeId)
    }
}

fn map_leaf_err(err: LeafVerifyError) -> VerifyError {
    match err {
        LeafVerifyError::Expired => VerifyError::Expired,
        LeafVerifyError::NoUriSan => VerifyError::NoSpiffeUriSan,
        LeafVerifyError::AmbiguousUriSan => VerifyError::AmbiguousUriSan,
        LeafVerifyError::Undecodable(detail) => VerifyError::UntrustedIssuer {
            detail: format!("undecodable peer leaf: {detail}"),
        },
        LeafVerifyError::UntrustedIssuer(detail) => VerifyError::UntrustedIssuer { detail },
        LeafVerifyError::CaCapableLeaf => VerifyError::UntrustedIssuer {
            detail: "peer leaf is CA-capable (basicConstraints cA TRUE); a CA must not authenticate as a workload".to_string(),
        },
        LeafVerifyError::MissingClientAuthEku => VerifyError::UntrustedIssuer {
            detail: "peer leaf does not carry the clientAuth extended key usage required of a caller".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use os_trustd_domain::JoinToken;
    use os_trustd_domain::ca::CertificateAuthority;
    use os_trustd_domain::certificate::CertUsage;

    const JOIN_TOKEN: &str = "clusterid.clustersecret";

    fn real_service() -> (SecurityService<EcdsaP256Signer>, EcdsaP256Signer) {
        let ca_signer = EcdsaP256Signer::generate().unwrap();
        let ca_key = KeyPair::new(ca_signer.private_key_der(), ca_signer.public_key_spki_der());
        let token = JoinToken::new(JOIN_TOKEN).unwrap();
        let ca = CertificateAuthority::bootstrap(
            "oyatie-cell-7-ca",
            ca_key,
            ca_signer.clone(),
            1_000,
            10_000_000,
        )
        .unwrap();
        (SecurityService::new(token, ca), ca_signer)
    }

    fn bundle_for(
        svc: &SecurityService<EcdsaP256Signer>,
        ca_signer: &EcdsaP256Signer,
    ) -> TrustBundle<EcdsaP256Signer> {
        let mut bundle = TrustBundle::new();
        bundle
            .add_anchor(svc.ca_certificate().clone(), ca_signer.clone())
            .unwrap();
        bundle
    }

    #[test]
    fn issue_then_verify_round_trips_spiffe_id() {
        let (mut svc, ca_signer) = real_service();
        let wl = EcdsaP256Signer::generate().unwrap();
        let uri = "spiffe://oyatie.cell-7/tenant/ten_acme/secrets-sync";
        let mut issuer =
            TrustdSvidIssuer::new(&mut svc, JOIN_TOKEN, wl, ca_signer.clone(), "secrets-sync");
        let request = SvidRequest::new(SpiffeId::parse(uri).unwrap(), 3_600);
        let svid = issuer.issue(&request, 2_000).unwrap();
        assert_eq!(svid.spiffe_id.as_uri(), uri);

        let bundle = bundle_for(&svc, &ca_signer);
        let verifier = TrustdSvidVerifier::new(&bundle);
        let recovered = verifier.verify_peer(&svid.leaf_der, 2_500).unwrap();
        assert_eq!(recovered.as_uri(), uri);
    }

    #[test]
    fn forged_leaf_from_untrusted_ca_is_denied() {
        let (mut rogue_svc, rogue_signer) = real_service();
        let wl = EcdsaP256Signer::generate().unwrap();
        let mut issuer =
            TrustdSvidIssuer::new(&mut rogue_svc, JOIN_TOKEN, wl, rogue_signer.clone(), "evil");
        let request = SvidRequest::new(
            SpiffeId::parse("spiffe://oyatie.cell-7/tenant/ten_acme/evil").unwrap(),
            3_600,
        );
        let forged = issuer.issue(&request, 2_000).unwrap();

        let (svc, ca_signer) = real_service();
        let bundle = bundle_for(&svc, &ca_signer);
        let verifier = TrustdSvidVerifier::new(&bundle);
        let err = verifier.verify_peer(&forged.leaf_der, 2_500).unwrap_err();
        assert!(matches!(err, VerifyError::UntrustedIssuer { .. }));
    }

    #[test]
    fn expired_leaf_is_denied_distinctly() {
        let (mut svc, ca_signer) = real_service();
        let wl = EcdsaP256Signer::generate().unwrap();
        let mut issuer = TrustdSvidIssuer::new(&mut svc, JOIN_TOKEN, wl, ca_signer.clone(), "x");
        let request = SvidRequest::new(
            SpiffeId::parse("spiffe://oyatie.cell-7/platform/cloud-iam-pdp").unwrap(),
            3_600,
        );
        let svid = issuer.issue(&request, 2_000).unwrap(); // valid [2000, 5600)
        let bundle = bundle_for(&svc, &ca_signer);
        let verifier = TrustdSvidVerifier::new(&bundle);
        assert_eq!(
            verifier.verify_peer(&svid.leaf_der, 6_000).unwrap_err(),
            VerifyError::Expired
        );
    }

    #[test]
    fn leaf_without_uri_san_is_denied() {
        let (mut svc, ca_signer) = real_service();
        let node = EcdsaP256Signer::generate().unwrap();
        let node_key = KeyPair::new(node.private_key_der(), node.public_key_spki_der());
        let node_csr =
            CertificateSigningRequest::for_node("node-1", &node_key, CertUsage::ClientAuth, 3_600)
                .with_dns("node-1.cluster.local");
        let cert_request = CertificateRequest {
            join_token: JOIN_TOKEN.to_string(),
            csr: node_csr,
        };
        let resp = svc
            .handle_certificate(&cert_request, &node_key, 2_000)
            .unwrap();
        let leaf_der = der::encode_leaf_der(
            &resp.identity.certificate,
            &node,
            svc.ca_certificate(),
            &ca_signer,
        )
        .unwrap();
        let bundle = bundle_for(&svc, &ca_signer);
        let verifier = TrustdSvidVerifier::new(&bundle);
        assert_eq!(
            verifier.verify_peer(&leaf_der, 2_500).unwrap_err(),
            VerifyError::NoSpiffeUriSan
        );
    }

    #[test]
    fn bad_join_token_refuses_issuance() {
        let (mut svc, ca_signer) = real_service();
        let wl = EcdsaP256Signer::generate().unwrap();
        let mut issuer = TrustdSvidIssuer::new(&mut svc, "clusterid.WRONG", wl, ca_signer, "x");
        let request = SvidRequest::new(
            SpiffeId::parse("spiffe://oyatie.cell-7/platform/x").unwrap(),
            3_600,
        );
        assert!(issuer.issue(&request, 2_000).is_err());
    }

    #[test]
    fn garbage_leaf_is_untrusted_deny() {
        let (svc, ca_signer) = real_service();
        let bundle = bundle_for(&svc, &ca_signer);
        let verifier = TrustdSvidVerifier::new(&bundle);
        let err = verifier
            .verify_peer(b"garbage-not-a-real-der-leaf", 2_500)
            .unwrap_err();
        assert!(matches!(err, VerifyError::UntrustedIssuer { .. }));
    }
}
