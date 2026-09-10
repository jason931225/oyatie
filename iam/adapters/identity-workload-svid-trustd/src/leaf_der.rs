//! X.509 leaf-DER parsing and fail-closed verification of a presented peer leaf.

use x509_parser::certificate::X509Certificate;
use x509_parser::extensions::GeneralName;
use x509_parser::prelude::FromDer;

/// The outcome classes of parsing and verifying a real leaf DER.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeafVerifyError {
    Undecodable(String),
    UntrustedIssuer(String),
    Expired,
    CaCapableLeaf,
    MissingClientAuthEku,
    NoUriSan,
    AmbiguousUriSan,
}

/// Verify `leaf_der` against `trusted_ca_spki_ders` at `now` and return its single URI SAN.
/// # Errors
/// [`LeafVerifyError`] for every reject path.
pub fn verify_leaf_der(
    leaf_der: &[u8],
    trusted_ca_spki_ders: &[Vec<u8>],
    now: u64,
) -> Result<String, LeafVerifyError> {
    let (rest, cert) = X509Certificate::from_der(leaf_der)
        .map_err(|e| LeafVerifyError::Undecodable(format!("DER parse failed: {e}")))?;
    if !rest.is_empty() {
        return Err(LeafVerifyError::Undecodable(format!(
            "{} trailing bytes after the leaf certificate",
            rest.len()
        )));
    }

    let verified = trusted_ca_spki_ders.iter().any(|spki| {
        match x509_parser::x509::SubjectPublicKeyInfo::from_der(spki) {
            Ok((spki_rest, ca_spki)) => {
                spki_rest.is_empty() && cert.verify_signature(Some(&ca_spki)).is_ok()
            }
            Err(_) => false,
        }
    });
    if !verified {
        return Err(LeafVerifyError::UntrustedIssuer(
            "leaf signature did not verify under any trusted CA".to_string(),
        ));
    }

    let not_before = cert.validity().not_before.timestamp();
    let not_after = cert.validity().not_after.timestamp();
    let now_i = i64::try_from(now).unwrap_or(i64::MAX);
    if now_i < not_before || now_i >= not_after {
        return Err(LeafVerifyError::Expired);
    }

    // Absent basicConstraints means `cA` is FALSE (RFC 5280); a malformed one is rejected.
    match cert.basic_constraints() {
        Ok(Some(bc)) if bc.value.ca => return Err(LeafVerifyError::CaCapableLeaf),
        Ok(_) => {}
        Err(_) => return Err(LeafVerifyError::CaCapableLeaf),
    }

    match cert.extended_key_usage() {
        Ok(Some(eku)) if eku.value.client_auth => {}
        Ok(_) => return Err(LeafVerifyError::MissingClientAuthEku),
        Err(_) => return Err(LeafVerifyError::MissingClientAuthEku),
    }

    let uris = uri_sans(&cert)?;
    match uris.as_slice() {
        [] => Err(LeafVerifyError::NoUriSan),
        [single] => Ok(single.clone()),
        _ => Err(LeafVerifyError::AmbiguousUriSan),
    }
}

/// Read the single SPIFFE URI SAN from an ALREADY-TRUSTED leaf: no chain or validity
/// check, so never call it on a presented peer leaf (use [`verify_leaf_der`]).
/// # Errors
/// [`LeafVerifyError`] when the leaf is undecodable or its URI SAN count is not one.
pub fn extract_single_uri_san(leaf_der: &[u8]) -> Result<String, LeafVerifyError> {
    let (rest, cert) = X509Certificate::from_der(leaf_der)
        .map_err(|e| LeafVerifyError::Undecodable(format!("DER parse failed: {e}")))?;
    if !rest.is_empty() {
        return Err(LeafVerifyError::Undecodable(format!(
            "{} trailing bytes after the certificate",
            rest.len()
        )));
    }
    match uri_sans(&cert)?.as_slice() {
        [] => Err(LeafVerifyError::NoUriSan),
        [single] => Ok(single.clone()),
        _ => Err(LeafVerifyError::AmbiguousUriSan),
    }
}

fn uri_sans(cert: &X509Certificate<'_>) -> Result<Vec<String>, LeafVerifyError> {
    let san = cert
        .subject_alternative_name()
        .map_err(|e| LeafVerifyError::Undecodable(format!("malformed SAN extension: {e}")))?;
    let Some(san) = san else {
        return Ok(Vec::new());
    };
    Ok(san
        .value
        .general_names
        .iter()
        .filter_map(|gn| match gn {
            GeneralName::URI(uri) => Some((*uri).to_string()),
            _ => None,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use os_trustd_domain::ca::{CertificateAuthority, CertificateSigningRequest};
    use os_trustd_domain::der;
    use os_trustd_domain::signer::EcdsaP256Signer;
    use os_trustd_domain::x509::KeyPair;

    const URI: &str = "spiffe://oyatie.cell-7/tenant/ten_acme/secrets-sync";

    fn real_ca() -> (CertificateAuthority<EcdsaP256Signer>, EcdsaP256Signer) {
        let signer = EcdsaP256Signer::generate().unwrap();
        let ca_key = KeyPair::new(signer.private_key_der(), signer.public_key_spki_der());
        let ca = CertificateAuthority::bootstrap(
            "oyatie-cell-7-ca",
            ca_key,
            signer.clone(),
            1_000,
            10_000_000,
        )
        .unwrap();
        (ca, signer)
    }

    fn issue(
        ca: &mut CertificateAuthority<EcdsaP256Signer>,
        ca_signer: &EcdsaP256Signer,
        uri: &str,
    ) -> (Vec<u8>, Vec<u8>) {
        let wl = EcdsaP256Signer::generate().unwrap();
        let wl_key = KeyPair::new(wl.private_key_der(), wl.public_key_spki_der());
        let csr = CertificateSigningRequest::for_workload("wl", uri, &wl_key, 3_600);
        let leaf = ca.sign_csr(&csr, 2_000).unwrap();
        let der = der::encode_leaf_der(&leaf, &wl, ca.certificate(), ca_signer).unwrap();
        let ca_spki = ca.certificate().public_key_der.clone();
        (der, ca_spki)
    }

    #[test]
    fn real_leaf_verifies_and_yields_uri() {
        let (mut ca, sgn) = real_ca();
        let (leaf, ca_spki) = issue(&mut ca, &sgn, URI);
        let got = verify_leaf_der(&leaf, &[ca_spki], 2_500).unwrap();
        assert_eq!(got, URI);
    }

    #[test]
    fn forged_leaf_from_untrusted_ca_fails_signature() {
        let (mut rogue, rogue_sgn) = real_ca();
        let (forged, _rogue_spki) = issue(&mut rogue, &rogue_sgn, URI);
        let (good, _g) = real_ca();
        let good_spki = good.certificate().public_key_der.clone();
        let err = verify_leaf_der(&forged, &[good_spki], 2_500).unwrap_err();
        assert!(matches!(err, LeafVerifyError::UntrustedIssuer(_)));
    }

    #[test]
    fn expired_leaf_is_distinct_deny() {
        let (mut ca, sgn) = real_ca();
        let (leaf, ca_spki) = issue(&mut ca, &sgn, URI); // valid [2000,5600)
        assert_eq!(
            verify_leaf_der(&leaf, &[ca_spki], 6_000).unwrap_err(),
            LeafVerifyError::Expired
        );
    }

    #[test]
    fn post_signature_uri_tamper_breaks_verification() {
        let (mut ca, sgn) = real_ca();
        let (leaf, ca_spki) = issue(&mut ca, &sgn, URI);
        let mut tampered = leaf.clone();
        let i = tampered.len() / 3;
        tampered[i] ^= 0xFF;
        let res = verify_leaf_der(&tampered, &[ca_spki], 2_500);
        assert!(res.is_err(), "tampered real-DER leaf must not verify");
    }

    #[test]
    fn undecodable_bytes_are_rejected() {
        let err = verify_leaf_der(b"not-a-der-cert", &[vec![1, 2, 3]], 2_500).unwrap_err();
        assert!(matches!(err, LeafVerifyError::Undecodable(_)));
    }

    #[test]
    fn workload_leaf_carries_clientauth_and_is_not_ca() {
        let (mut ca, sgn) = real_ca();
        let (leaf, ca_spki) = issue(&mut ca, &sgn, URI);
        assert_eq!(verify_leaf_der(&leaf, &[ca_spki], 2_500).unwrap(), URI);
    }

    #[test]
    fn ca_capable_leaf_is_rejected() {
        use os_trustd_domain::certificate::{CertUsage, Certificate};
        use os_trustd_domain::x509::{DistinguishedName, SubjectAltNames, Validity};
        let (ca, sgn) = real_ca();
        let leaf_signer = EcdsaP256Signer::generate().unwrap();
        let ca_leaf = Certificate {
            serial: 42,
            subject: DistinguishedName::common("rogue-sub-ca"),
            issuer: DistinguishedName::common("oyatie-cell-7-ca"),
            validity: Validity {
                not_before: 1_000,
                not_after: 9_000,
            },
            usage: CertUsage::CertificateAuthority,
            sans: SubjectAltNames {
                uris: vec![URI.to_string()],
                ..Default::default()
            },
            public_key_der: leaf_signer.public_key_spki_der(),
            signature: vec![0x01],
        };
        let der_bytes =
            der::encode_leaf_der(&ca_leaf, &leaf_signer, ca.certificate(), &sgn).unwrap();
        let ca_spki = ca.certificate().public_key_der.clone();
        assert_eq!(
            verify_leaf_der(&der_bytes, &[ca_spki], 2_500).unwrap_err(),
            LeafVerifyError::CaCapableLeaf
        );
    }

    #[test]
    fn leaf_without_clientauth_eku_is_rejected() {
        use os_trustd_domain::certificate::{CertUsage, Certificate};
        use os_trustd_domain::x509::{DistinguishedName, SubjectAltNames, Validity};
        let (ca, sgn) = real_ca();
        let leaf_signer = EcdsaP256Signer::generate().unwrap();
        let server_leaf = Certificate {
            serial: 43,
            subject: DistinguishedName::common("cloud-iam-pdp"),
            issuer: DistinguishedName::common("oyatie-cell-7-ca"),
            validity: Validity {
                not_before: 1_000,
                not_after: 9_000,
            },
            usage: CertUsage::ServerAuth,
            sans: SubjectAltNames {
                uris: vec![URI.to_string()],
                ..Default::default()
            },
            public_key_der: leaf_signer.public_key_spki_der(),
            signature: vec![0x01],
        };
        let der_bytes =
            der::encode_leaf_der(&server_leaf, &leaf_signer, ca.certificate(), &sgn).unwrap();
        let ca_spki = ca.certificate().public_key_der.clone();
        assert_eq!(
            verify_leaf_der(&der_bytes, &[ca_spki], 2_500).unwrap_err(),
            LeafVerifyError::MissingClientAuthEku
        );
    }

    #[test]
    fn extract_single_uri_san_reads_identity_without_verifying() {
        let (mut ca, sgn) = real_ca();
        let (leaf, _ca_spki) = issue(&mut ca, &sgn, URI);
        assert_eq!(extract_single_uri_san(&leaf).unwrap(), URI);
        assert!(matches!(
            extract_single_uri_san(b"not-a-der").unwrap_err(),
            LeafVerifyError::Undecodable(_)
        ));
    }

    #[test]
    fn node_leaf_without_uri_san_is_no_uri() {
        let (mut ca, sgn) = real_ca();
        use os_trustd_domain::certificate::CertUsage;
        let wl = EcdsaP256Signer::generate().unwrap();
        let wl_key = KeyPair::new(wl.private_key_der(), wl.public_key_spki_der());
        let csr =
            CertificateSigningRequest::for_node("node-1", &wl_key, CertUsage::ClientAuth, 3_600)
                .with_dns("node-1.cluster.local");
        let leaf = ca.sign_csr(&csr, 2_000).unwrap();
        let der_bytes = der::encode_leaf_der(&leaf, &wl, ca.certificate(), &sgn).unwrap();
        let ca_spki = ca.certificate().public_key_der.clone();
        assert_eq!(
            verify_leaf_der(&der_bytes, &[ca_spki], 2_500).unwrap_err(),
            LeafVerifyError::NoUriSan
        );
    }
}
