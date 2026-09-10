//! Sealing-root provenance: HOW root material came into existence, typed.
//!
//! ADR-0537 step 0 mandates a Shamir M-of-N, geo-separated, dual-control
//! root ceremony. The ADR-0510 transitional custody (OpenBao) DEFERS that
//! ceremony — and the deferral is carried as a type, not narrated in docs,
//! so every boot path receives the custody posture together with the root
//! and can log, gate, or refuse on it. At W5 cutover the quorum variant
//! becomes the only production-constructible provenance.

use std::fmt;

/// Typed record of how a sealing root was established.
///
/// Deliberately NOT `Copy`/`Default`: a provenance exists only where root
/// material was actually ingested, and each variant names its authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootProvenance {
    /// ADR-0510 transitional posture: OpenBao generated and custodies the
    /// root as a SINGLE-CUSTODIAN exportable transit key — explicitly WEAKER
    /// than the ADR-0537 step-0 quorum doctrine, accepted only for the
    /// transitional window.
    OpenBaoTransitionalSingleCustodian { ceremony_evidence_ref: String },
    /// ADR-0537 step 0: Shamir M-of-N reconstruction from geo-separated,
    /// dual-control custodian shares. Typed placeholder until the ceremony
    /// tooling lands — no production constructor exists yet, but boot paths
    /// can already demand this variant at W5 cutover.
    ShamirQuorumCeremony {
        threshold: u8,
        share_count: u8,
        ceremony_evidence_ref: String,
    },
}

impl RootProvenance {
    /// Whether this provenance satisfies the ADR-0537 step-0 quorum
    /// doctrine. Boot paths use this to gate or alarm on transitional
    /// posture; at W5 cutover the gate flips to fail-closed.
    pub fn satisfies_quorum_doctrine(&self) -> bool {
        match self {
            Self::OpenBaoTransitionalSingleCustodian { .. } => false,
            Self::ShamirQuorumCeremony {
                threshold,
                share_count,
                ..
            } => *threshold >= 2 && share_count >= threshold,
        }
    }

    pub fn ceremony_evidence_ref(&self) -> &str {
        match self {
            Self::OpenBaoTransitionalSingleCustodian {
                ceremony_evidence_ref,
            }
            | Self::ShamirQuorumCeremony {
                ceremony_evidence_ref,
                ..
            } => ceremony_evidence_ref,
        }
    }
}

impl fmt::Display for RootProvenance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OpenBaoTransitionalSingleCustodian { .. } => f.write_str(
                "openbao-transitional-single-custodian (ADR-0510; defers ADR-0537 step 0)",
            ),
            Self::ShamirQuorumCeremony {
                threshold,
                share_count,
                ..
            } => {
                write!(
                    f,
                    "shamir-quorum-ceremony {threshold}-of-{share_count} (ADR-0537 step 0)"
                )
            }
        }
    }
}
