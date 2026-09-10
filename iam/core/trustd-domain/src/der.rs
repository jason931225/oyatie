//! Bridges the in-memory [`Certificate`] shape model to real ASN.1 X.509 DER,
//! signed by the issuing CA key through `rcgen` on the `aws-lc-rs` backend.

use rcgen::string::Ia5String;
use rcgen::{
    CertificateParams, DistinguishedName as RcgenDn, DnType, ExtendedKeyUsagePurpose, IsCa, Issuer,
    KeyUsagePurpose, SanType, SerialNumber,
};
use time::OffsetDateTime;

use crate::certificate::{CertUsage, Certificate};
use crate::error::{Result, TrustError};
use crate::signer::EcdsaP256Signer;

/// An issued leaf plus the CA certificate a verifier anchors it against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IssuedDer {
    /// DER bytes of the issued X.509 leaf certificate.
    pub leaf_der: Vec<u8>,
    /// DER bytes of the issuing CA certificate.
    pub ca_der: Vec<u8>,
}

/// Self-sign `ca_cert` with its own key, materialising a trust anchor's DER.
///
/// # Errors
/// [`TrustError`] if any field is unrepresentable or rcgen fails to sign.
pub fn encode_ca_der(ca_cert: &Certificate, ca_signer: &EcdsaP256Signer) -> Result<Vec<u8>> {
    let params = params_from_certificate(ca_cert)?;
    let cert = params
        .self_signed(ca_signer.key_pair())
        .map_err(|e| TrustError::Other(format!("CA self-sign failed: {e}")))?;
    Ok(cert.der().as_ref().to_vec())
}

/// Sign a leaf for `subject_signer`'s public key with the CA's key. Every SAN in
/// `leaf_cert`, the SPIFFE URI included, is signed into the certificate rather
/// than carried alongside it.
///
/// # Errors
/// [`TrustError`] on an unrepresentable field (non-ASCII URI/DNS SAN, bad IP) or
/// an rcgen signing failure.
pub fn encode_leaf_der(
    leaf_cert: &Certificate,
    subject_signer: &EcdsaP256Signer,
    ca_cert: &Certificate,
    ca_signer: &EcdsaP256Signer,
) -> Result<Vec<u8>> {
    let leaf_params = params_from_certificate(leaf_cert)?;
    let issuer_params = params_from_certificate(ca_cert)?;
    let issuer = Issuer::new(issuer_params, ca_signer.key_pair());
    let cert = leaf_params
        .signed_by(subject_signer.key_pair(), &issuer)
        .map_err(|e| TrustError::Other(format!("leaf signing failed: {e}")))?;
    Ok(cert.der().as_ref().to_vec())
}

/// Build the leaf DER and its issuing CA DER in one call.
///
/// # Errors
/// [`TrustError`] as for [`encode_leaf_der`] / [`encode_ca_der`].
pub fn issue_der(
    leaf_cert: &Certificate,
    subject_signer: &EcdsaP256Signer,
    ca_cert: &Certificate,
    ca_signer: &EcdsaP256Signer,
) -> Result<IssuedDer> {
    Ok(IssuedDer {
        leaf_der: encode_leaf_der(leaf_cert, subject_signer, ca_cert, ca_signer)?,
        ca_der: encode_ca_der(ca_cert, ca_signer)?,
    })
}

fn params_from_certificate(cert: &Certificate) -> Result<CertificateParams> {
    let mut params = CertificateParams::new(Vec::<String>::new())
        .map_err(|e| TrustError::Other(format!("rcgen params init failed: {e}")))?;
    params.distinguished_name = distinguished_name(cert);
    params.not_before = offset_from_unix(cert.validity.not_before)?;
    params.not_after = offset_from_unix(cert.validity.not_after)?;
    params.serial_number = Some(SerialNumber::from(cert.serial));
    params.subject_alt_names = subject_alt_names(cert)?;
    apply_usage(&mut params, cert.usage);
    Ok(params)
}

fn distinguished_name(cert: &Certificate) -> RcgenDn {
    let mut dn = RcgenDn::new();
    dn.push(DnType::CommonName, cert.subject.common_name.as_str());
    for org in &cert.subject.organizations {
        dn.push(DnType::OrganizationName, org.as_str());
    }
    dn
}

fn subject_alt_names(cert: &Certificate) -> Result<Vec<SanType>> {
    let mut sans = Vec::new();
    for dns in &cert.sans.dns_names {
        sans.push(SanType::DnsName(ia5(dns)?));
    }
    for ip in &cert.sans.ip_addresses {
        let addr = ip
            .parse::<std::net::IpAddr>()
            .map_err(|_| TrustError::invalid(format!("invalid IP SAN '{ip}'")))?;
        sans.push(SanType::IpAddress(addr));
    }
    for uri in &cert.sans.uris {
        sans.push(SanType::URI(ia5(uri)?));
    }
    Ok(sans)
}

fn apply_usage(params: &mut CertificateParams, usage: CertUsage) {
    match usage {
        CertUsage::CertificateAuthority => {
            params.is_ca = IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
            params.key_usages = vec![
                KeyUsagePurpose::KeyCertSign,
                KeyUsagePurpose::CrlSign,
                KeyUsagePurpose::DigitalSignature,
            ];
        }
        CertUsage::ServerAuth => {
            params.is_ca = IsCa::NoCa;
            params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
            params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        }
        CertUsage::ClientAuth => {
            params.is_ca = IsCa::NoCa;
            params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
            params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];
        }
    }
}

fn ia5(s: &str) -> Result<Ia5String> {
    Ia5String::try_from(s.to_string())
        .map_err(|_| TrustError::invalid(format!("value '{s}' is not a valid IA5 string")))
}

fn offset_from_unix(secs: u64) -> Result<OffsetDateTime> {
    let s = i64::try_from(secs)
        .map_err(|_| TrustError::invalid(format!("timestamp {secs} out of range")))?;
    OffsetDateTime::from_unix_timestamp(s)
        .map_err(|_| TrustError::invalid(format!("timestamp {secs} out of range")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ca::{CertificateAuthority, CertificateSigningRequest};
    use crate::x509::KeyPair;
    use x509_parser::prelude::FromDer;

    const SPIFFE_URI: &str = "spiffe://oyatie.cell-7/tenant/ten_acme/secrets-sync";

    fn real_ca() -> (CertificateAuthority<EcdsaP256Signer>, EcdsaP256Signer) {
        let signer = EcdsaP256Signer::generate().unwrap();
        let ca_keypair = KeyPair::new(signer.private_key_der(), signer.public_key_spki_der());
        let ca = CertificateAuthority::bootstrap(
            "oyatie-cell-7-ca",
            ca_keypair,
            signer.clone(),
            1_000,
            10_000_000,
        )
        .unwrap();
        (ca, signer)
    }

    #[test]
    fn leaf_der_parses_and_carries_the_signed_spiffe_uri() {
        let (mut ca, ca_signer) = real_ca();
        let wl_signer = EcdsaP256Signer::generate().unwrap();
        let wl_key = KeyPair::new(wl_signer.private_key_der(), wl_signer.public_key_spki_der());
        let csr =
            CertificateSigningRequest::for_workload("secrets-sync", SPIFFE_URI, &wl_key, 3_600);
        let leaf = ca.sign_csr(&csr, 2_000).unwrap();

        let der = encode_leaf_der(&leaf, &wl_signer, ca.certificate(), &ca_signer).unwrap();

        let (_, parsed) = x509_parser::certificate::X509Certificate::from_der(&der).unwrap();
        let san = parsed.subject_alternative_name().unwrap().unwrap();
        let uris: Vec<String> = san
            .value
            .general_names
            .iter()
            .filter_map(|gn| match gn {
                x509_parser::extensions::GeneralName::URI(u) => Some((*u).to_string()),
                _ => None,
            })
            .collect();
        assert_eq!(uris, vec![SPIFFE_URI.to_string()]);
    }

    #[test]
    fn leaf_signature_verifies_against_real_ca_and_tamper_breaks_it() {
        let (mut ca, ca_signer) = real_ca();
        let wl_signer = EcdsaP256Signer::generate().unwrap();
        let wl_key = KeyPair::new(wl_signer.private_key_der(), wl_signer.public_key_spki_der());
        let csr =
            CertificateSigningRequest::for_workload("secrets-sync", SPIFFE_URI, &wl_key, 3_600);
        let leaf = ca.sign_csr(&csr, 2_000).unwrap();

        let issued = issue_der(&leaf, &wl_signer, ca.certificate(), &ca_signer).unwrap();
        let (_, leaf_parsed) =
            x509_parser::certificate::X509Certificate::from_der(&issued.leaf_der).unwrap();
        let (_, ca_parsed) =
            x509_parser::certificate::X509Certificate::from_der(&issued.ca_der).unwrap();

        assert!(
            leaf_parsed
                .verify_signature(Some(ca_parsed.public_key()))
                .is_ok()
        );

        let mut tampered = issued.leaf_der.clone();
        let mid = tampered.len() / 2;
        tampered[mid] ^= 0xFF;
        // A tampered leaf may fail to parse OR fail verification; both are DENY.
        let broke = match x509_parser::certificate::X509Certificate::from_der(&tampered) {
            Err(_) => true,
            Ok((_, t)) => t.verify_signature(Some(ca_parsed.public_key())).is_err(),
        };
        assert!(broke, "tampering a real-DER leaf must break verification");
    }
}
