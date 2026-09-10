use std::collections::BTreeMap;
use std::path::Path;

use foundry_records_draft::{RecordsLog, RecordsLogError, SealedEnvelope};
use foundry_records_sqlite_draft::SqliteRecordsLog;
use foundry_spine::{ProjectionState, SyncStatus, fold_from_scratch};
use tokio::sync::Mutex;

use crate::auth::OperatorCredential;
use crate::authz::PolicyEnforcementPoint;
use crate::config::Config;
use crate::seed::registry_for;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BootError {
    ActionLogUnopenable {
        detail: String,
    },
    DenialLogUnopenable {
        detail: String,
    },
    /// Both logs name one path. A shared store would let a refusal land in
    /// the log it was refused from.
    LogPathsAliased,
    NoTenantsConfigured,
    SeedRefused {
        tenant_id: String,
        detail: String,
    },
    ReplayFailed {
        tenant_id: String,
        detail: String,
    },
    PolicyRejected {
        detail: String,
    },
}

impl std::fmt::Display for BootError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ActionLogUnopenable { detail } => {
                write!(formatter, "the action log could not be opened: {detail}")
            }
            Self::DenialLogUnopenable { detail } => {
                write!(formatter, "the denial trail could not be opened: {detail}")
            }
            Self::LogPathsAliased => write!(
                formatter,
                "the action log and the denial trail must be distinct stores"
            ),
            Self::NoTenantsConfigured => {
                write!(
                    formatter,
                    "no tenants configured; the roster is the served set"
                )
            }
            Self::SeedRefused { tenant_id, detail } => {
                write!(
                    formatter,
                    "tenant {tenant_id} could not be seeded: {detail}"
                )
            }
            Self::ReplayFailed { tenant_id, detail } => {
                write!(
                    formatter,
                    "tenant {tenant_id} could not be replayed: {detail}"
                )
            }
            Self::PolicyRejected { detail } => {
                write!(formatter, "the policy seed was rejected: {detail}")
            }
        }
    }
}

pub struct TenantState {
    pub projection: ProjectionState,
    /// Held behind the PORT so a test can install a log that fails on demand.
    pub action_log: Box<dyn RecordsLog + Send>,
    pub denial_log: Box<dyn RecordsLog + Send>,
}

impl std::fmt::Debug for TenantState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TenantState")
            .field("applied_ordinal", &self.projection.applied_ordinal)
            .finish_non_exhaustive()
    }
}

impl TenantState {
    /// The three handles `submit` requires, borrowed together so the
    /// borrow checker enforces what the write path already assumes.
    pub fn write_handles(
        &mut self,
    ) -> (
        &mut dyn RecordsLog,
        &mut dyn RecordsLog,
        &mut ProjectionState,
    ) {
        (
            &mut *self.action_log,
            &mut *self.denial_log,
            &mut self.projection,
        )
    }
}

impl TenantState {
    /// Costs a full replay — O(tenant log), unbounded — and callers hold the
    /// tenant mutex across it, so the observation in `metrics` degrades while
    /// it runs.
    ///
    /// The `Applied` disposition `audit_view` reports is sound only while
    /// this process is the sole writer: an entry appended below our
    /// `applied_ordinal` by another writer would pass its filter and be
    /// labelled applied by a process that never folded it.
    pub fn entries_now(&self, tenant_id: &str) -> Result<Vec<SealedEnvelope>, RecordsLogError> {
        self.action_log.replay(tenant_id, 1)
    }

    pub fn sync_status(&self, tenant_id: &str) -> Result<SyncStatus, RecordsLogError> {
        Ok(self
            .projection
            .sync_status(self.action_log.head(tenant_id)?))
    }
}

/// Everything the surface serves from. One mutex per tenant is honest: a
/// submission needs the log and the projection together, and SQLite is a
/// single-writer store.
#[derive(Debug)]
pub struct AppState {
    pub tenants: BTreeMap<String, Mutex<TenantState>>,
    pub pep: PolicyEnforcementPoint,
    pub metrics: crate::metrics::Metrics,
    pub operators: Vec<OperatorCredential>,
}

impl AppState {
    pub fn tenant_count(&self) -> usize {
        self.tenants.len()
    }
}

pub fn compose(config: &Config) -> Result<AppState, BootError> {
    if config.tenants.is_empty() {
        return Err(BootError::NoTenantsConfigured);
    }
    if paths_alias(&config.action_log, &config.denial_log) {
        return Err(BootError::LogPathsAliased);
    }
    let action_log = SqliteRecordsLog::open(&config.action_log).map_err(|error| {
        BootError::ActionLogUnopenable {
            detail: format!("{error:?}"),
        }
    })?;
    refuse_unless_denial_trail_opens(&config.denial_log)?;

    let pep = PolicyEnforcementPoint::load(POLICY_VERSION).map_err(|error| {
        BootError::PolicyRejected {
            detail: error.to_string(),
        }
    })?;

    let mut tenants = BTreeMap::new();
    for tenant_id in &config.tenants {
        let registry = registry_for(tenant_id).map_err(|error| BootError::SeedRefused {
            tenant_id: tenant_id.clone(),
            detail: error.to_string(),
        })?;
        let entries = action_log
            .replay(tenant_id, 1)
            .map_err(|error| BootError::ReplayFailed {
                tenant_id: tenant_id.clone(),
                detail: format!("{error:?}"),
            })?;
        let projection = fold_from_scratch(tenant_id, &registry, entries.iter());
        drop(entries);
        let action_log = SqliteRecordsLog::open(&config.action_log).map_err(|error| {
            BootError::ActionLogUnopenable {
                detail: format!("{error:?}"),
            }
        })?;
        let denial_log = SqliteRecordsLog::open(&config.denial_log).map_err(|error| {
            BootError::DenialLogUnopenable {
                detail: format!("{error:?}"),
            }
        })?;
        tenants.insert(
            tenant_id.clone(),
            Mutex::new(TenantState {
                projection,
                action_log: Box::new(action_log),
                denial_log: Box::new(denial_log),
            }),
        );
    }
    Ok(AppState {
        tenants,
        pep,
        metrics: crate::metrics::Metrics::default(),
        operators: config.operators.clone(),
    })
}

const POLICY_VERSION: &str = "psv-000001";

fn refuse_unless_denial_trail_opens(path: &Path) -> Result<(), BootError> {
    SqliteRecordsLog::open(path)
        .map(drop)
        .map_err(|error| BootError::DenialLogUnopenable {
            detail: format!("{error:?}"),
        })
}

/// Two configured paths name one store. Canonicalization is best-effort —
/// the paths need not exist yet — so a literal match is the fallback.
fn paths_alias(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}
