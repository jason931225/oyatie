use std::fmt;

use shared_platform_contracts_kernel::ContractViolation;
use shared_platform_contracts_kernel::pdp::PolicyVersion;

/// Why the PDP refused to decide. Every variant is fail-closed: a PEP MUST
/// treat any error as deny.
#[derive(Debug, Clone, PartialEq)]
pub enum PdpError {
    /// The request violates the locked PDP contract.
    InvalidRequest(Vec<ContractViolation>),
    /// The caller pinned a zookie freshness floor the loaded bundle does not
    /// satisfy (equality-only comparison per the contract): the PDP refuses
    /// rather than answer against stale policy.
    StalePolicyVersion {
        required: PolicyVersion,
        loaded: PolicyVersion,
    },
    /// The bundle failed parse/strict-validation/link and was NOT loaded.
    BundleRejected { detail: String },
    /// The request's action slug has no engine mapping in the loaded bundle.
    UnknownAction { action: String },
    /// Engine-level evaluation failure (malformed entity slice, etc.).
    Evaluation { detail: String },
    /// A decision id could not be minted; the decision is not emitted
    /// because it would be unattributable in the audit chain.
    DecisionIdUnavailable { detail: String },
    /// The PDP reached a decision but could not durably append the signed
    /// audit-chain event. Callers must fail closed rather than use an
    /// unaudited authorization outcome.
    AuditChainEmission { detail: String },
    /// The wrapped PDP returned only after its elapsed-time budget.
    RuntimeTimeout { deadline_ms: u64 },
    /// The wrapped PDP panicked; the guard caught it and failed closed.
    RuntimePanic { detail: String },
    /// Runtime fault streak opened the guard circuit; no inner PDP call ran.
    CircuitOpen { consecutive_failures: u32 },
}

impl fmt::Display for PdpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(violations) => {
                write!(f, "invalid authorization request: ")?;
                for (i, v) in violations.iter().enumerate() {
                    if i > 0 {
                        write!(f, "; ")?;
                    }
                    write!(f, "{v}")?;
                }
                Ok(())
            }
            Self::StalePolicyVersion { required, loaded } => write!(
                f,
                "policy bundle too stale: caller pinned {} but loaded version is {}",
                required.as_str(),
                loaded.as_str()
            ),
            Self::BundleRejected { detail } => write!(f, "policy bundle rejected: {detail}"),
            Self::UnknownAction { action } => {
                write!(
                    f,
                    "action {action:?} has no engine mapping in the loaded bundle"
                )
            }
            Self::Evaluation { detail } => write!(f, "evaluation failed: {detail}"),
            Self::DecisionIdUnavailable { detail } => {
                write!(f, "decision id unavailable: {detail}")
            }
            Self::AuditChainEmission { detail } => {
                write!(f, "audit-chain emission failed: {detail}")
            }
            Self::RuntimeTimeout { deadline_ms } => {
                write!(
                    f,
                    "PDP runtime elapsed budget exceeded after {deadline_ms}ms"
                )
            }
            Self::RuntimePanic { detail } => write!(f, "PDP runtime panicked: {detail}"),
            Self::CircuitOpen {
                consecutive_failures,
            } => write!(
                f,
                "PDP runtime circuit is open after {consecutive_failures} consecutive failures"
            ),
        }
    }
}

impl std::error::Error for PdpError {}

impl PdpError {
    /// Whether this error should count toward the fail-closed circuit breaker.
    /// Caller-shape refusals stay deny outcomes without accusing the runtime.
    #[must_use]
    pub fn is_runtime_fault(&self) -> bool {
        matches!(
            self,
            Self::DecisionIdUnavailable { .. }
                | Self::AuditChainEmission { .. }
                | Self::RuntimeTimeout { .. }
                | Self::RuntimePanic { .. }
        )
    }
}
