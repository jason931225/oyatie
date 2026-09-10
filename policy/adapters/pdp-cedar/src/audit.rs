use std::fmt;
use std::sync::{Arc, Mutex};

use audit_chain_domain::{
    AuditAppendInput, AuditChain, AuditChainError, AuditEvent, Ed25519SigningKey,
    Ed25519VerificationKeySet, Plane,
};
use audit_file_adapter::{FileAuditLedger, FileAuditLedgerError};
use data_boundary_kernel::{DataClass, Purpose};
use shared_pdp_kernel::{
    DecisionAuditRecord, EntitySlice, PdpError, PdpOutcome, PolicyBundle, PolicyDecisionPoint,
};
use shared_platform_contracts_kernel::pdp::{AuthorizationRequest, Decision, PolicyVersion};
use shared_ulid_id_kernel::IdGenerator;

use super::CedarPdp;

pub const PDP_DECISION_AUDIT_SURFACE: &str = "authorization.pdp.decision";

pub struct PdpDecisionAuditChainLogger {
    ledger: FileAuditLedger,
    signer: Ed25519SigningKey,
    trusted_keys: Ed25519VerificationKeySet,
    chain: Mutex<AuditChain>,
}

impl fmt::Debug for PdpDecisionAuditChainLogger {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PdpDecisionAuditChainLogger")
            .field("ledger", &self.ledger)
            .field("signer", &self.signer)
            .finish_non_exhaustive()
    }
}

impl PdpDecisionAuditChainLogger {
    /// # Errors
    /// [`PdpAuditChainError::Load`] when the persisted ledger cannot be read or
    /// replayed as per-tenant audit-chain shards.
    pub fn new(
        ledger: FileAuditLedger,
        signer: Ed25519SigningKey,
        trusted_keys: Ed25519VerificationKeySet,
    ) -> Result<Self, PdpAuditChainError> {
        verify_signer_is_trusted(&signer, &trusted_keys)?;
        let chain = load_trusted_multi_tenant_shards(&ledger, &trusted_keys)?;
        Ok(Self {
            ledger,
            signer,
            trusted_keys,
            chain: Mutex::new(chain),
        })
    }

    /// # Errors
    /// Returns an error if the in-memory chain lock is poisoned, the audit-chain
    /// kernel rejects the append, or the file ledger cannot persist the new tip.
    pub fn record(&self, audit: &DecisionAuditRecord) -> Result<AuditEvent, PdpAuditChainError> {
        let mut current = self
            .chain
            .lock()
            .map_err(|_| PdpAuditChainError::LockPoisoned)?;
        let persisted = load_trusted_multi_tenant_shards(&self.ledger, &self.trusted_keys)?;
        if persisted != *current {
            return Err(PdpAuditChainError::Persist(
                FileAuditLedgerError::ChainDiverged,
            ));
        }
        let mut next = current.clone();
        let event = next
            .append_signed(decision_audit_input(audit), &self.signer)
            .map_err(PdpAuditChainError::Append)?
            .clone();
        self.ledger
            .append_chain(&next)
            .map_err(PdpAuditChainError::Persist)?;
        load_trusted_multi_tenant_shards(&self.ledger, &self.trusted_keys)?;
        *current = next;
        Ok(event)
    }
}

fn verify_signer_is_trusted(
    signer: &Ed25519SigningKey,
    trusted_keys: &Ed25519VerificationKeySet,
) -> Result<(), PdpAuditChainError> {
    let trusted_key = trusted_keys
        .trusted_key_for(signer.key_id())
        .map_err(PdpAuditChainError::UntrustedSigner)?;
    let signer_key = signer.verification_key();
    if &signer_key != trusted_key {
        return Err(PdpAuditChainError::UntrustedSigner(
            AuditChainError::Ed25519SignatureKeyMismatch {
                key_id: signer.key_id().to_owned(),
            },
        ));
    }
    Ok(())
}

fn load_trusted_multi_tenant_shards(
    ledger: &FileAuditLedger,
    trusted_keys: &Ed25519VerificationKeySet,
) -> Result<AuditChain, PdpAuditChainError> {
    let chain = ledger
        .load_multi_tenant_shards()
        .map_err(PdpAuditChainError::Load)?;
    chain
        .verify_signed_with_keys(trusted_keys)
        .map_err(PdpAuditChainError::TrustedSignatureReplay)?;
    Ok(chain)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PdpAuditChainError {
    Load(FileAuditLedgerError),
    TrustedSignatureReplay(AuditChainError),
    UntrustedSigner(AuditChainError),
    Append(AuditChainError),
    Persist(FileAuditLedgerError),
    LockPoisoned,
}

impl fmt::Display for PdpAuditChainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Load(error) => write!(formatter, "load failed: {error:?}"),
            Self::TrustedSignatureReplay(error) => {
                write!(formatter, "trusted signature replay failed: {error:?}")
            }
            Self::UntrustedSigner(error) => {
                write!(formatter, "audit-chain signer is not trusted: {error:?}")
            }
            Self::Append(error) => write!(formatter, "append failed: {error:?}"),
            Self::Persist(error) => write!(formatter, "persist failed: {error:?}"),
            Self::LockPoisoned => write!(formatter, "audit-chain logger lock poisoned"),
        }
    }
}

impl std::error::Error for PdpAuditChainError {}

/// Fail-closes unless each decision is durably signed into the audit-chain
/// ledger BEFORE the authorization outcome is returned to the caller.
pub struct AuditChainCedarPdp {
    inner: CedarPdp,
    audit_logger: PdpDecisionAuditChainLogger,
}

impl AuditChainCedarPdp {
    /// # Errors
    /// [`PdpError::BundleRejected`] when the bundle is invalid.
    pub fn load(
        bundle: &PolicyBundle,
        id_gen: Arc<dyn IdGenerator>,
        cache_capacity: usize,
        audit_logger: PdpDecisionAuditChainLogger,
    ) -> Result<Self, PdpError> {
        Ok(Self {
            inner: CedarPdp::load(bundle, id_gen, cache_capacity)?,
            audit_logger,
        })
    }

    pub fn swap_bundle(&self, bundle: &PolicyBundle) -> Result<(), PdpError> {
        self.inner.swap_bundle(bundle)
    }
}

impl PolicyDecisionPoint for AuditChainCedarPdp {
    fn authorize(
        &self,
        request: &AuthorizationRequest,
        entities: &EntitySlice,
    ) -> Result<PdpOutcome, PdpError> {
        let outcome = self.inner.authorize(request, entities)?;
        self.audit_logger
            .record(&outcome.audit)
            .map_err(|error| PdpError::AuditChainEmission {
                detail: error.to_string(),
            })?;
        Ok(outcome)
    }

    fn loaded_policy_version(&self) -> PolicyVersion {
        self.inner.loaded_policy_version()
    }
}

fn decision_audit_input(audit: &DecisionAuditRecord) -> AuditAppendInput {
    AuditAppendInput {
        tenant_id: audit.tenant_id.clone(),
        surface: PDP_DECISION_AUDIT_SURFACE.to_owned(),
        plane: Plane::Control,
        purpose: Purpose::CoreService,
        data_classes: vec![DataClass::InternalOnly, DataClass::Audit],
        decision: decision_lineage_payload(audit),
    }
}

fn decision_lineage_payload(audit: &DecisionAuditRecord) -> String {
    serde_json::json!({
        "decision_id": &audit.decision_id,
        "request_id": &audit.request_id,
        "tenant_id": &audit.tenant_id,
        "principal": {
            "entity_type": &audit.principal.entity_type,
            "entity_id": &audit.principal.entity_id,
        },
        "resource": {
            "entity_type": &audit.resource.entity_type,
            "entity_id": &audit.resource.entity_id,
        },
        "action": &audit.action,
        "decision": decision_label(audit.decision),
        "policy_version": audit.policy_version.as_str(),
        "determining_policy_ids": &audit.determining_policy_ids,
        "cache_hit": audit.cache_hit,
    })
    .to_string()
}

fn decision_label(decision: Decision) -> &'static str {
    match decision {
        Decision::Allow => "allow",
        Decision::Deny => "deny",
    }
}
