//! Per-AGENT-TOKEN reserve-then-reconcile quota: estimate prompt + max tokens,
//! reserve that estimate against the agent's remaining budget, skip the reserve
//! write when headroom exceeds [`QUOTA_AMPLE_THRESHOLD_PCT`], then reconcile the
//! actual usage on response. State is keyed on `(TenantId, AgentToken)`, NOT
//! source IP, so NAT-fleet agents are correctly attributed.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use crate::{RepositoryError, TenantId};

/// Opaque per-agent identity token. Carries no credential material.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct AgentToken(pub String); // data_class: TENANT_SCOPED

impl fmt::Display for AgentToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Budget configuration for one `(TenantId, AgentToken)` pair.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AgentQuotaBudget {
    /// Total token budget for the current window.
    pub budget_tokens: u64, // data_class: INTERNAL_ONLY
    /// Unix-epoch milliseconds at which the window resets (0 = no reset).
    pub window_reset_unix_ms: u64, // data_class: INTERNAL_ONLY
}

/// Snapshot of the current quota state for one `(TenantId, AgentToken)` pair.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AgentQuotaSnapshot {
    /// Total budget for the window.
    pub budget_tokens: u64, // data_class: INTERNAL_ONLY
    /// Currently available tokens (budget_tokens minus in-flight reserves).
    pub remaining_tokens: u64, // data_class: INTERNAL_ONLY
    /// When the window resets (0 = not configured).
    pub window_reset_unix_ms: u64, // data_class: INTERNAL_ONLY
}

/// Typed failure from a quota store operation. data_class: INTERNAL_ONLY —
/// detail fields must never echo agent payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QuotaError {
    /// The agent's remaining budget is insufficient for the requested reserve.
    BudgetExceeded {
        /// The agent whose budget was exceeded. data_class: TENANT_SCOPED
        agent: AgentToken,
        /// The number of tokens requested. data_class: INTERNAL_ONLY
        requested: u64,
        /// The remaining budget at the time of the check. data_class: INTERNAL_ONLY
        remaining: u64,
    },
    /// A backing-store failure.
    Repository(RepositoryError),
}

impl fmt::Display for QuotaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BudgetExceeded {
                agent,
                requested,
                remaining,
            } => write!(
                f,
                "quota budget exceeded for agent {}: requested {}, remaining {}",
                agent.0, requested, remaining
            ),
            Self::Repository(e) => write!(f, "quota store error: {e}"),
        }
    }
}

impl std::error::Error for QuotaError {}

/// Port for per-AGENT-TOKEN token-budget accounting, keyed on
/// `(TenantId, AgentToken)`. Two agents holding the same token value in
/// different tenants MUST be isolated by the implementation.
pub trait AgentQuotaStore: Send + Sync {
    /// Return the current budget snapshot for `(tenant_id, agent)`; an
    /// unconfigured pair yields an all-zero snapshot the caller must interpret.
    ///
    /// # Errors
    /// [`RepositoryError`] on store failure.
    fn snapshot(
        &self,
        tenant_id: &TenantId,
        agent: &AgentToken,
    ) -> Result<AgentQuotaSnapshot, RepositoryError>;

    /// Atomically reserve `tokens` from the agent's remaining budget, or refuse
    /// without mutating the store.
    ///
    /// # Errors
    /// [`QuotaError::BudgetExceeded`] when `remaining < tokens`;
    /// [`QuotaError::Repository`] on store failure.
    fn reserve(
        &mut self,
        tenant_id: &TenantId,
        agent: &AgentToken,
        tokens: u64,
    ) -> Result<(), QuotaError>;

    /// Reconcile a previous reserve: replace the reserved `estimate` with
    /// `actual_used`, crediting an over-reserve back (capped at `budget_tokens`)
    /// or debiting the excess (floored at 0). A skipped reserve passes
    /// `estimate = 0`, so the whole of `actual_used` is debited.
    ///
    /// # Errors
    /// [`RepositoryError`] on store failure.
    fn reconcile(
        &mut self,
        tenant_id: &TenantId,
        agent: &AgentToken,
        estimate: u64,
        actual_used: u64,
    ) -> Result<(), RepositoryError>;
}

/// Fraction of budget that must remain for [`should_skip_reserve`] to skip the
/// hot-path reserve write. Reconcile still always runs.
pub const QUOTA_AMPLE_THRESHOLD_PCT: u64 = 80;

/// Whether the agent's remaining budget is ample enough to skip the reserve
/// write. A zero budget never skips.
#[must_use]
pub fn should_skip_reserve(snap: &AgentQuotaSnapshot) -> bool {
    if snap.budget_tokens == 0 {
        return false;
    }
    let remaining_pct = snap.remaining_tokens.saturating_mul(100) / snap.budget_tokens;
    remaining_pct > QUOTA_AMPLE_THRESHOLD_PCT
}

#[derive(Clone, Debug)]
struct QuotaEntry {
    budget_tokens: u64,        // data_class: INTERNAL_ONLY
    remaining_tokens: u64,     // data_class: INTERNAL_ONLY
    window_reset_unix_ms: u64, // data_class: INTERNAL_ONLY
}

/// In-memory [`AgentQuotaStore`] reference adapter for tests and single-node
/// bring-up. Interior mutability is `Arc<Mutex<_>>` so clones share state.
#[derive(Clone, Debug, Default)]
pub struct InMemoryAgentQuotaStore {
    entries: Arc<Mutex<BTreeMap<(String, String), QuotaEntry>>>,
    // data_class: INTERNAL_ONLY
}

impl InMemoryAgentQuotaStore {
    /// Build an empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed (or replace) the budget for `(tenant_id, agent)`. Must be called
    /// before [`AgentQuotaStore::reserve`]; `remaining_tokens` starts at budget.
    pub fn set_budget(&mut self, tenant_id: TenantId, agent: AgentToken, budget: AgentQuotaBudget) {
        if let Ok(mut guard) = self.entries.lock() {
            guard.insert(
                (tenant_id.0, agent.0),
                QuotaEntry {
                    budget_tokens: budget.budget_tokens,
                    remaining_tokens: budget.budget_tokens,
                    window_reset_unix_ms: budget.window_reset_unix_ms,
                },
            );
        }
    }
}

impl AgentQuotaStore for InMemoryAgentQuotaStore {
    fn snapshot(
        &self,
        tenant_id: &TenantId,
        agent: &AgentToken,
    ) -> Result<AgentQuotaSnapshot, RepositoryError> {
        let guard = self
            .entries
            .lock()
            .map_err(|_| RepositoryError::new("quota store mutex poisoned"))?;
        let key = (tenant_id.0.clone(), agent.0.clone());
        match guard.get(&key) {
            Some(entry) => Ok(AgentQuotaSnapshot {
                budget_tokens: entry.budget_tokens,
                remaining_tokens: entry.remaining_tokens,
                window_reset_unix_ms: entry.window_reset_unix_ms,
            }),
            None => Ok(AgentQuotaSnapshot {
                budget_tokens: 0,
                remaining_tokens: 0,
                window_reset_unix_ms: 0,
            }),
        }
    }

    fn reserve(
        &mut self,
        tenant_id: &TenantId,
        agent: &AgentToken,
        tokens: u64,
    ) -> Result<(), QuotaError> {
        let mut guard = self.entries.lock().map_err(|_| {
            QuotaError::Repository(RepositoryError::new("quota store mutex poisoned"))
        })?;
        let key = (tenant_id.0.clone(), agent.0.clone());
        let entry = guard
            .get_mut(&key)
            .ok_or_else(|| QuotaError::Repository(RepositoryError::new("quota entry not found")))?;

        if entry.remaining_tokens < tokens {
            return Err(QuotaError::BudgetExceeded {
                agent: agent.clone(),
                requested: tokens,
                remaining: entry.remaining_tokens,
            });
        }
        entry.remaining_tokens -= tokens;
        Ok(())
    }

    fn reconcile(
        &mut self,
        tenant_id: &TenantId,
        agent: &AgentToken,
        estimate: u64,
        actual_used: u64,
    ) -> Result<(), RepositoryError> {
        let mut guard = self
            .entries
            .lock()
            .map_err(|_| RepositoryError::new("quota store mutex poisoned"))?;
        let key = (tenant_id.0.clone(), agent.0.clone());
        let entry = match guard.get_mut(&key) {
            Some(e) => e,
            None => return Ok(()),
        };

        if actual_used < estimate {
            let credit = estimate - actual_used;
            entry.remaining_tokens = entry
                .remaining_tokens
                .saturating_add(credit)
                .min(entry.budget_tokens);
        } else if actual_used > estimate {
            let extra = actual_used - estimate;
            entry.remaining_tokens = entry.remaining_tokens.saturating_sub(extra);
        }
        Ok(())
    }
}
