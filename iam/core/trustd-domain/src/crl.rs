//! Refusing an otherwise-valid certificate that was administratively revoked:
//! an in-memory list keyed by serial, with RFC 5280 reason codes. Rotation, not
//! revocation, is the primary mechanism; this covers what rotation cannot.

use crate::certificate::Certificate;
use crate::error::{Result, TrustError};
use std::collections::BTreeMap;

/// Why a certificate was revoked. Mirrors the RFC 5280 `CRLReason` enumeration,
/// trimmed to the codes meaningful for a cluster PKI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevocationReason {
    /// No specific reason recorded.
    Unspecified,
    /// The private key was (or may have been) compromised.
    KeyCompromise,
    /// The issuing CA itself was compromised (whole generation invalid).
    CaCompromise,
    /// The subject's affiliation changed (e.g. node left the cluster).
    AffiliationChanged,
    /// The certificate was superseded by a re-issued one.
    Superseded,
    /// The node/operation the cert served was decommissioned.
    CessationOfOperation,
    /// Temporarily held (may be reinstated).
    CertificateHold,
}

impl RevocationReason {
    /// The numeric `CRLReason` code as used in RFC 5280.
    pub fn code(self) -> u8 {
        match self {
            RevocationReason::Unspecified => 0,
            RevocationReason::KeyCompromise => 1,
            RevocationReason::CaCompromise => 2,
            RevocationReason::AffiliationChanged => 3,
            RevocationReason::Superseded => 4,
            RevocationReason::CessationOfOperation => 5,
            RevocationReason::CertificateHold => 6,
        }
    }

    /// Whether this revocation is permanent (a held cert can be released).
    pub fn is_permanent(self) -> bool {
        !matches!(self, RevocationReason::CertificateHold)
    }

    /// Short stable tag for logging/serialization.
    pub fn as_str(self) -> &'static str {
        match self {
            RevocationReason::Unspecified => "unspecified",
            RevocationReason::KeyCompromise => "key-compromise",
            RevocationReason::CaCompromise => "ca-compromise",
            RevocationReason::AffiliationChanged => "affiliation-changed",
            RevocationReason::Superseded => "superseded",
            RevocationReason::CessationOfOperation => "cessation-of-operation",
            RevocationReason::CertificateHold => "certificate-hold",
        }
    }
}

/// A single revocation list entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RevocationEntry {
    /// The serial of the revoked certificate.
    pub serial: u64,
    /// Why it was revoked.
    pub reason: RevocationReason,
    /// When (Unix seconds) it was revoked.
    pub revoked_at: u64,
}

/// An in-memory certificate revocation list keyed by serial number.
///
/// trustd consults this before honouring an otherwise-valid certificate. The
/// list carries a monotonically increasing `crl_number` so peers can detect a
/// stale copy, matching the RFC 5280 `cRLNumber` extension.
#[derive(Debug, Clone, Default)]
pub struct RevocationList {
    entries: BTreeMap<u64, RevocationEntry>,
    crl_number: u64,
}

impl RevocationList {
    /// An empty revocation list at CRL number 0.
    pub fn new() -> Self {
        RevocationList {
            entries: BTreeMap::new(),
            crl_number: 0,
        }
    }

    /// The current CRL sequence number; bumped on every mutation.
    pub fn crl_number(&self) -> u64 {
        self.crl_number
    }

    /// Number of currently-revoked serials.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the list is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Revoke a certificate by serial. Re-revoking with a *stronger* reason
    /// (key/CA compromise) upgrades an existing hold; re-revoking with the same
    /// or weaker reason is idempotent. Returns the new CRL number.
    pub fn revoke(&mut self, serial: u64, reason: RevocationReason, now: u64) -> u64 {
        let upgrade = match self.entries.get(&serial) {
            None => true,
            // A hold can be hardened into a permanent revocation, never the
            // reverse.
            Some(existing) => !existing.reason.is_permanent() && reason.is_permanent(),
        };
        if upgrade {
            self.entries.insert(
                serial,
                RevocationEntry {
                    serial,
                    reason,
                    revoked_at: now,
                },
            );
            self.crl_number = self.crl_number.wrapping_add(1);
        }
        self.crl_number
    }

    /// Revoke the certificate object directly.
    pub fn revoke_cert(&mut self, cert: &Certificate, reason: RevocationReason, now: u64) -> u64 {
        self.revoke(cert.serial, reason, now)
    }

    /// Release a certificate that was only on *hold*. Returns an error if the
    /// serial is permanently revoked or was never held.
    pub fn release_hold(&mut self, serial: u64) -> Result<()> {
        match self.entries.get(&serial) {
            Some(e) if e.reason == RevocationReason::CertificateHold => {
                self.entries.remove(&serial);
                self.crl_number = self.crl_number.wrapping_add(1);
                Ok(())
            }
            Some(_) => Err(TrustError::invalid(
                "certificate is permanently revoked and cannot be released",
            )),
            None => Err(TrustError::not_found("serial is not on hold")),
        }
    }

    /// Whether a serial is currently revoked.
    pub fn is_revoked(&self, serial: u64) -> bool {
        self.entries.contains_key(&serial)
    }

    /// Look up the revocation entry for a serial, if any.
    pub fn entry(&self, serial: u64) -> Option<&RevocationEntry> {
        self.entries.get(&serial)
    }

    /// Ensure a certificate is not revoked; returns its serial on success.
    pub fn ensure_valid(&self, cert: &Certificate) -> Result<()> {
        if let Some(entry) = self.entries.get(&cert.serial) {
            return Err(TrustError::verification_failed(format!(
                "certificate serial {} revoked ({})",
                entry.serial,
                entry.reason.as_str()
            )));
        }
        Ok(())
    }

    /// Iterate over revoked entries in ascending serial order.
    pub fn iter(&self) -> impl Iterator<Item = &RevocationEntry> {
        self.entries.values()
    }

    /// Drop hold-only entries older than `cutoff` (housekeeping). Permanent
    /// revocations are retained. Returns how many were pruned.
    pub fn prune_expired_holds(&mut self, cutoff: u64) -> usize {
        let before = self.entries.len();
        self.entries
            .retain(|_, e| e.reason.is_permanent() || e.revoked_at >= cutoff);
        let pruned = before - self.entries.len();
        if pruned > 0 {
            self.crl_number = self.crl_number.wrapping_add(1);
        }
        pruned
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::certificate::CertUsage;
    use crate::x509::{DistinguishedName, SubjectAltNames, Validity};

    fn cert(serial: u64) -> Certificate {
        Certificate {
            serial,
            subject: DistinguishedName::common("node"),
            issuer: DistinguishedName::common("ca"),
            validity: Validity::from_duration(0, 100).unwrap(),
            usage: CertUsage::ClientAuth,
            sans: SubjectAltNames::default(),
            public_key_der: vec![1],
            signature: vec![2],
        }
    }

    #[test]
    fn revoke_and_check() {
        let mut crl = RevocationList::new();
        assert!(crl.is_empty());
        crl.revoke(7, RevocationReason::KeyCompromise, 100);
        assert!(crl.is_revoked(7));
        assert_eq!(crl.crl_number(), 1);
        assert_eq!(
            crl.entry(7).unwrap().reason,
            RevocationReason::KeyCompromise
        );
        assert!(crl.ensure_valid(&cert(7)).is_err());
        assert!(crl.ensure_valid(&cert(8)).is_ok());
    }

    #[test]
    fn idempotent_revoke_does_not_bump_crl() {
        let mut crl = RevocationList::new();
        crl.revoke(1, RevocationReason::Superseded, 10);
        let n = crl.crl_number();
        crl.revoke(1, RevocationReason::Superseded, 20);
        assert_eq!(crl.crl_number(), n);
        assert_eq!(crl.len(), 1);
    }

    #[test]
    fn hold_can_be_upgraded_and_released() {
        let mut crl = RevocationList::new();
        crl.revoke(5, RevocationReason::CertificateHold, 10);
        assert!(crl.is_revoked(5));
        crl.release_hold(5).unwrap();
        assert!(!crl.is_revoked(5));
        crl.revoke(5, RevocationReason::CertificateHold, 20);
        crl.revoke(5, RevocationReason::KeyCompromise, 30);
        assert_eq!(
            crl.entry(5).unwrap().reason,
            RevocationReason::KeyCompromise
        );
        assert_eq!(crl.release_hold(5).unwrap_err().kind(), "invalid");
    }

    #[test]
    fn release_unknown_serial_errors() {
        let mut crl = RevocationList::new();
        assert_eq!(crl.release_hold(99).unwrap_err().kind(), "not_found");
    }

    #[test]
    fn reason_codes_match_rfc() {
        assert_eq!(RevocationReason::KeyCompromise.code(), 1);
        assert_eq!(RevocationReason::CaCompromise.code(), 2);
        assert!(RevocationReason::KeyCompromise.is_permanent());
        assert!(!RevocationReason::CertificateHold.is_permanent());
    }

    #[test]
    fn prune_keeps_permanent_drops_old_holds() {
        let mut crl = RevocationList::new();
        crl.revoke(1, RevocationReason::CertificateHold, 10);
        crl.revoke(2, RevocationReason::KeyCompromise, 10);
        crl.revoke(3, RevocationReason::CertificateHold, 100);
        let pruned = crl.prune_expired_holds(50);
        assert_eq!(pruned, 1); // only serial 1
        assert!(!crl.is_revoked(1));
        assert!(crl.is_revoked(2));
        assert!(crl.is_revoked(3));
    }

    #[test]
    fn revoke_cert_uses_serial() {
        let mut crl = RevocationList::new();
        crl.revoke_cert(&cert(42), RevocationReason::AffiliationChanged, 5);
        assert!(crl.is_revoked(42));
    }
}
