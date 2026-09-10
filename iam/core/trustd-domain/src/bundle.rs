//! Trust bundle: the trusted CA set a leaf is verified against. A CA rotation
//! trusts more than one generation at once, so verification asks the whole set.

use crate::certificate::{CertUsage, Certificate};
use crate::crl::RevocationList;
use crate::error::{Result, TrustError};
use crate::signer::SigningBackend;
use crate::x509::PEMEncoded;
use std::collections::BTreeMap;

struct TrustAnchor<S: SigningBackend> {
    cert: Certificate,
    signer: S,
}

/// A set of trusted CA roots indexed by subject DN string.
pub struct TrustBundle<S: SigningBackend> {
    anchors: BTreeMap<String, TrustAnchor<S>>,
}

impl<S: SigningBackend> Default for TrustBundle<S> {
    fn default() -> Self {
        TrustBundle {
            anchors: BTreeMap::new(),
        }
    }
}

impl<S: SigningBackend> TrustBundle<S> {
    /// An empty trust bundle.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a trusted CA anchor; refuses a certificate that is not a CA.
    pub fn add_anchor(&mut self, ca_cert: Certificate, signer: S) -> Result<()> {
        if !ca_cert.is_ca() {
            return Err(TrustError::invalid("trust anchor is not a CA certificate"));
        }
        ca_cert.validate()?;
        let key = ca_cert.subject.to_rfc();
        self.anchors.insert(
            key,
            TrustAnchor {
                cert: ca_cert,
                signer,
            },
        );
        Ok(())
    }

    /// Number of trusted anchors.
    pub fn len(&self) -> usize {
        self.anchors.len()
    }

    /// Whether the bundle holds no anchors.
    pub fn is_empty(&self) -> bool {
        self.anchors.is_empty()
    }

    /// Whether a CA with the given subject DN string is trusted.
    pub fn trusts(&self, subject_rfc: &str) -> bool {
        self.anchors.contains_key(subject_rfc)
    }

    /// PEM documents for every trusted CA, suitable for writing a ca-bundle file.
    pub fn ca_pems(&self) -> Vec<PEMEncoded> {
        self.anchors.values().map(|a| a.cert.to_pem()).collect()
    }

    /// The SubjectPublicKeyInfo DER of every trusted CA anchor: the key set the
    /// adapter's real-DER verifier checks a presented leaf's signature against.
    pub fn trusted_ca_spki_ders(&self) -> Vec<Vec<u8>> {
        self.anchors
            .values()
            .map(|a| a.cert.public_key_der.clone())
            .collect()
    }

    /// The trust-anchor certificates, in stable subject-DN order.
    pub fn anchor_certificates(&self) -> Vec<&Certificate> {
        self.anchors.values().map(|a| &a.cert).collect()
    }

    /// Remove a CA anchor by subject DN; returns whether one existed.
    pub fn remove_anchor(&mut self, subject_rfc: &str) -> bool {
        self.anchors.remove(subject_rfc).is_some()
    }

    /// Verify a leaf against the bundle at `now`: structurally valid, inside its
    /// validity window, and issued by an unexpired trusted anchor that signed it.
    pub fn verify_leaf(&self, leaf: &Certificate, now: u64) -> Result<()> {
        leaf.validate()?;
        if !leaf.is_valid_at(now) {
            return Err(TrustError::expired(
                "leaf certificate not valid at given time",
            ));
        }
        let issuer_key = leaf.issuer.to_rfc();
        let anchor = self
            .anchors
            .get(&issuer_key)
            .ok_or_else(|| TrustError::verification_failed("no trusted CA matches issuer"))?;
        if anchor.cert.validity.is_expired(now) {
            return Err(TrustError::expired("anchoring CA certificate has expired"));
        }
        if !anchor.signer.verify(&leaf.tbs_bytes(), &leaf.signature) {
            return Err(TrustError::verification_failed(
                "leaf signature not produced by trusted CA",
            ));
        }
        Ok(())
    }

    /// Verify a leaf and additionally reject a serial listed in `crl`.
    pub fn verify_leaf_with_crl(
        &self,
        leaf: &Certificate,
        crl: &RevocationList,
        now: u64,
    ) -> Result<()> {
        crl.ensure_valid(leaf)?;
        self.verify_leaf(leaf, now)
    }

    /// Verify the chain, then require the leaf to carry `expected` usage.
    pub fn verify_leaf_usage(
        &self,
        leaf: &Certificate,
        expected: CertUsage,
        now: u64,
    ) -> Result<()> {
        self.verify_leaf(leaf, now)?;
        if leaf.usage != expected {
            return Err(TrustError::verification_failed(
                "certificate usage does not match required usage",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ca::{CertificateAuthority, CertificateSigningRequest};
    use crate::certificate::CertUsage;
    use crate::crl::{RevocationList, RevocationReason};
    use crate::signer::InMemorySigner;
    use crate::x509::KeyPair;

    fn make_ca(name: &str, seed: &str) -> CertificateAuthority<InMemorySigner> {
        CertificateAuthority::bootstrap(
            name,
            KeyPair::from_seed(seed.as_bytes()),
            InMemorySigner::from_seed(seed),
            1000,
            1_000_000,
        )
        .unwrap()
    }

    fn leaf_from(ca: &mut CertificateAuthority<InMemorySigner>, name: &str) -> Certificate {
        let key = KeyPair::from_seed(name.as_bytes());
        let csr = CertificateSigningRequest::for_node(name, &key, CertUsage::ClientAuth, 3600);
        ca.sign_csr(&csr, 2000).unwrap()
    }

    #[test]
    fn rejects_non_ca_anchor() {
        let mut ca = make_ca("ca", "ca");
        let leaf = leaf_from(&mut ca, "node");
        let mut bundle = TrustBundle::new();
        assert_eq!(
            bundle
                .add_anchor(leaf, InMemorySigner::from_seed("x"))
                .unwrap_err()
                .kind(),
            "invalid"
        );
    }

    #[test]
    fn verifies_leaf_from_trusted_anchor() {
        let mut ca = make_ca("talos-ca", "ca-seed");
        let leaf = leaf_from(&mut ca, "node-1");
        let mut bundle = TrustBundle::new();
        bundle
            .add_anchor(
                ca.certificate().clone(),
                InMemorySigner::from_seed("ca-seed"),
            )
            .unwrap();
        assert!(bundle.trusts("CN=talos-ca"));
        assert!(bundle.verify_leaf(&leaf, 2500).is_ok());
    }

    #[test]
    fn rejects_leaf_from_untrusted_ca() {
        let mut rogue = make_ca("rogue", "rogue");
        let leaf = leaf_from(&mut rogue, "node-1");
        let good = make_ca("talos-ca", "ca-seed");
        let mut bundle = TrustBundle::new();
        bundle
            .add_anchor(
                good.certificate().clone(),
                InMemorySigner::from_seed("ca-seed"),
            )
            .unwrap();
        assert_eq!(
            bundle.verify_leaf(&leaf, 2500).unwrap_err().kind(),
            "verification_failed"
        );
    }

    #[test]
    fn multi_generation_rotation_trusts_both() {
        let mut old = make_ca("talos-ca-gen1", "gen1");
        let mut new = make_ca("talos-ca-gen2", "gen2");
        let old_leaf = leaf_from(&mut old, "node-old");
        let new_leaf = leaf_from(&mut new, "node-new");
        let mut bundle = TrustBundle::new();
        bundle
            .add_anchor(old.certificate().clone(), InMemorySigner::from_seed("gen1"))
            .unwrap();
        bundle
            .add_anchor(new.certificate().clone(), InMemorySigner::from_seed("gen2"))
            .unwrap();
        assert_eq!(bundle.len(), 2);
        assert!(bundle.verify_leaf(&old_leaf, 2500).is_ok());
        assert!(bundle.verify_leaf(&new_leaf, 2500).is_ok());

        assert!(bundle.remove_anchor("CN=talos-ca-gen1"));
        assert!(bundle.verify_leaf(&old_leaf, 2500).is_err());
        assert!(bundle.verify_leaf(&new_leaf, 2500).is_ok());
    }

    #[test]
    fn crl_blocks_verification() {
        let mut ca = make_ca("talos-ca", "ca-seed");
        let leaf = leaf_from(&mut ca, "node-1");
        let mut bundle = TrustBundle::new();
        bundle
            .add_anchor(
                ca.certificate().clone(),
                InMemorySigner::from_seed("ca-seed"),
            )
            .unwrap();
        let mut crl = RevocationList::new();
        assert!(bundle.verify_leaf_with_crl(&leaf, &crl, 2500).is_ok());
        crl.revoke_cert(&leaf, RevocationReason::KeyCompromise, 2400);
        assert!(bundle.verify_leaf_with_crl(&leaf, &crl, 2500).is_err());
    }

    #[test]
    fn usage_mismatch_rejected() {
        let mut ca = make_ca("talos-ca", "ca-seed");
        let leaf = leaf_from(&mut ca, "node-1"); // ClientAuth
        let mut bundle = TrustBundle::new();
        bundle
            .add_anchor(
                ca.certificate().clone(),
                InMemorySigner::from_seed("ca-seed"),
            )
            .unwrap();
        assert!(
            bundle
                .verify_leaf_usage(&leaf, CertUsage::ClientAuth, 2500)
                .is_ok()
        );
        assert!(
            bundle
                .verify_leaf_usage(&leaf, CertUsage::ServerAuth, 2500)
                .is_err()
        );
    }

    #[test]
    fn expired_leaf_rejected() {
        let mut ca = make_ca("talos-ca", "ca-seed");
        let leaf = leaf_from(&mut ca, "node-1"); // valid [2000, 5600)
        let mut bundle = TrustBundle::new();
        bundle
            .add_anchor(
                ca.certificate().clone(),
                InMemorySigner::from_seed("ca-seed"),
            )
            .unwrap();
        assert_eq!(
            bundle.verify_leaf(&leaf, 6000).unwrap_err().kind(),
            "expired"
        );
    }
}
