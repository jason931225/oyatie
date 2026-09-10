//! The transport edge of the mTLS PEP: a rustls `ClientCertVerifier` that runs
//! the leaf through the same [`TrustdSvidVerifier`] the in-process PEP uses, so
//! the two checks cannot diverge. Tenant binding runs afterwards, at the
//! application layer, in [`crate::mtls::SpiffeCallerAuth`].

use std::sync::Arc;

use rustls::crypto::WebPkiSupportedAlgorithms;
use rustls::pki_types::{CertificateDer, UnixTime};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::{DigitallySignedStruct, DistinguishedName, Error, SignatureScheme};

use iam_identity_workload_svid_kernel::SvidVerifier;
use iam_identity_workload_svid_trustd::TrustdSvidVerifier;
use os_trustd_domain::TrustBundle;
use os_trustd_domain::signer::SigningBackend;

/// A rustls client-certificate verifier over a trustd [`TrustBundle`], owned
/// rather than borrowed so no lifetime escapes into the rustls `ServerConfig`.
///
/// An empty bundle trusts nothing and so denies every leaf; unlike
/// [`crate::mtls::SpiffeCallerAuth::new`] this constructor does not refuse one.
pub struct SvidClientCertVerifier<S: SigningBackend> {
    bundle: Arc<TrustBundle<S>>,
    supported_algs: WebPkiSupportedAlgorithms,
}

// Hand-written because the trait requires `Debug` and neither field implements
// it — and the bundle holds trust material that must not reach a log anyway.
impl<S: SigningBackend> std::fmt::Debug for SvidClientCertVerifier<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SvidClientCertVerifier")
    }
}

impl<S: SigningBackend> SvidClientCertVerifier<S> {
    #[must_use]
    pub fn new(bundle: Arc<TrustBundle<S>>) -> Self {
        let supported_algs =
            rustls::crypto::aws_lc_rs::default_provider().signature_verification_algorithms;
        Self {
            bundle,
            supported_algs,
        }
    }
}

impl<S: SigningBackend + Send + Sync + 'static> ClientCertVerifier for SvidClientCertVerifier<S> {
    fn offer_client_auth(&self) -> bool {
        true
    }

    fn client_auth_mandatory(&self) -> bool {
        true
    }

    /// Deliberately empty. Hints are advisory (RFC 8446 lets a client send any
    /// certificate it has) and a workload has exactly one SVID to offer; the
    /// trust decision lives entirely in [`Self::verify_client_cert`].
    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        &[]
    }

    fn verify_client_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        // Ignored: an SVID leaf is verified directly against the bundle's CA
        // SPKIs, so there is no intermediate chain to build.
        _intermediates: &[CertificateDer<'_>],
        now: UnixTime,
    ) -> Result<ClientCertVerified, Error> {
        let verifier = TrustdSvidVerifier::new(&self.bundle);
        match verifier.verify_peer(end_entity.as_ref(), now.as_secs()) {
            Ok(_spiffe_id) => Ok(ClientCertVerified::assertion()),
            Err(_err) => Err(Error::InvalidCertificate(
                rustls::CertificateError::ApplicationVerificationFailure,
            )),
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, Error> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.supported_algs)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, Error> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.supported_algs)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.supported_algs.supported_schemes()
    }
}
