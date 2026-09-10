//! The [`Certificate`] type: an issued, signed X.509 certificate.

use crate::error::{Result, TrustError};
use crate::x509::{DistinguishedName, PEMEncoded, PEMLabel, SubjectAltNames, Validity};
use os_kernel::role::RoleSet;

/// What the CA is willing to issue, and what the holder may then do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertUsage {
    /// The only usage that may sign other certificates.
    CertificateAuthority,
    /// A TLS server certificate.
    ServerAuth,
    /// A TLS client certificate, which is what a workload SVID is.
    ClientAuth,
}

impl CertUsage {
    /// Whether a certificate with this usage may sign other certificates.
    pub fn can_sign(self) -> bool {
        matches!(self, CertUsage::CertificateAuthority)
    }
}

/// An issued certificate: the parsed shape, plus the signature that links it to
/// its issuer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Certificate {
    /// Monotonic per issuing CA.
    pub serial: u64,
    /// The subject distinguished name.
    pub subject: DistinguishedName,
    /// Equal to `subject` for a self-signed CA.
    pub issuer: DistinguishedName,
    /// The validity window.
    pub validity: Validity,
    /// The intended usage.
    pub usage: CertUsage,
    /// The subject alternative names.
    pub sans: SubjectAltNames,
    /// DER bytes of the subject's public key.
    pub public_key_der: Vec<u8>,
    /// The issuer's signature over [`Certificate::tbs_bytes`].
    pub signature: Vec<u8>,
}

impl Certificate {
    /// The RBAC roles encoded in the subject's organizational units.
    pub fn roles(&self) -> RoleSet {
        RoleSet::parse_ous(self.subject.organizations.iter().map(String::as_str))
    }

    /// Whether this is a CA certificate.
    pub fn is_ca(&self) -> bool {
        self.usage.can_sign()
    }

    /// Whether the certificate is currently valid at `now`.
    pub fn is_valid_at(&self, now: u64) -> bool {
        self.validity.contains(now)
    }

    /// The signing input: a deterministic serialization standing in for the DER
    /// `TBSCertificate`. Every field a verifier trusts must appear here, or it
    /// could be altered without breaking the signature.
    pub fn tbs_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.serial.to_be_bytes());
        buf.extend_from_slice(self.subject.to_rfc().as_bytes());
        buf.push(b'|');
        buf.extend_from_slice(self.issuer.to_rfc().as_bytes());
        buf.push(b'|');
        buf.extend_from_slice(&self.validity.not_before.to_be_bytes());
        buf.extend_from_slice(&self.validity.not_after.to_be_bytes());
        buf.push(self.usage as u8);
        for d in &self.sans.dns_names {
            buf.extend_from_slice(d.as_bytes());
            buf.push(b',');
        }
        for ip in &self.sans.ip_addresses {
            buf.extend_from_slice(ip.as_bytes());
            buf.push(b',');
        }
        // Separator: without it a DNS SAN could be shifted into the URI section
        // and change the SPIFFE identity without changing these bytes.
        buf.push(b'|');
        for uri in &self.sans.uris {
            buf.extend_from_slice(uri.as_bytes());
            buf.push(b',');
        }
        buf.extend_from_slice(&self.public_key_der);
        buf
    }

    /// PEM envelope: the TBS bytes then the signature, so a verifier can split
    /// them back apart.
    pub fn to_pem(&self) -> PEMEncoded {
        let mut der = self.tbs_bytes();
        der.extend_from_slice(&self.signature);
        PEMEncoded::new(PEMLabel::Certificate, der)
    }

    /// The structural invariants checked before a certificate is trusted at all.
    pub fn validate(&self) -> Result<()> {
        if self.subject.common_name.is_empty() {
            return Err(TrustError::invalid("certificate has empty common name"));
        }
        if self.signature.is_empty() {
            return Err(TrustError::invalid("certificate is unsigned"));
        }
        if self.validity.not_after <= self.validity.not_before {
            return Err(TrustError::invalid("certificate validity window is empty"));
        }
        Ok(())
    }

    /// The subject CN, which carries the node or workload name.
    pub fn common_name(&self) -> &str {
        &self.subject.common_name
    }
}

/// What a CSR requester gets back: its signed leaf plus the chain to anchor it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuedIdentity {
    /// The signed certificate.
    pub certificate: Certificate,
    /// PEM form of [`IssuedIdentity::certificate`].
    pub cert_pem: PEMEncoded,
    /// Always empty: the requester generated and kept its own private key.
    pub key_pem: PEMEncoded,
    /// The issuing CA, for chain building.
    pub ca_pem: PEMEncoded,
}

impl IssuedIdentity {
    /// The common name of the issued identity.
    pub fn name(&self) -> &str {
        self.certificate.common_name()
    }

    /// The issued certificate's textual PEM.
    pub fn cert_pem_text(&self) -> String {
        self.cert_pem.encode()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::x509::DistinguishedName;

    fn sample(usage: CertUsage) -> Certificate {
        Certificate {
            serial: 1,
            subject: DistinguishedName::common("node-1").with_org("os:reader"),
            issuer: DistinguishedName::common("talos-ca"),
            validity: Validity::from_duration(0, 100).unwrap(),
            usage,
            sans: SubjectAltNames::default(),
            public_key_der: vec![1, 2, 3],
            signature: vec![9, 9],
        }
    }

    #[test]
    fn roles_decoded_from_ou() {
        let c = sample(CertUsage::ClientAuth);
        assert!(c.roles().can_read());
        assert!(!c.roles().can_write());
    }

    #[test]
    fn validate_rejects_unsigned() {
        let mut c = sample(CertUsage::ClientAuth);
        c.signature.clear();
        assert_eq!(c.validate().unwrap_err().kind(), "invalid");
    }

    #[test]
    fn ca_flag_follows_usage() {
        assert!(sample(CertUsage::CertificateAuthority).is_ca());
        assert!(!sample(CertUsage::ServerAuth).is_ca());
    }

    #[test]
    fn tbs_changes_with_serial() {
        let a = sample(CertUsage::ClientAuth);
        let mut b = a.clone();
        b.serial = 2;
        assert_ne!(a.tbs_bytes(), b.tbs_bytes());
    }
}
