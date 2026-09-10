use std::sync::atomic::{AtomicUsize, Ordering};

use foundry_ontology_app::{AppState, compose};
use foundry_records_draft::{ActionEnvelope, Receipt, RecordsLog, RecordsLogError, SealedEnvelope};

pub struct AlwaysFailingLog {
    pub detail: &'static str,
}

impl AlwaysFailingLog {
    fn fault(&self) -> RecordsLogError {
        RecordsLogError::Storage {
            detail: self.detail.to_owned(),
        }
    }
}

impl RecordsLog for AlwaysFailingLog {
    fn append(&mut self, _envelope: ActionEnvelope) -> Result<Receipt, RecordsLogError> {
        Err(self.fault())
    }

    fn replay(
        &self,
        _tenant_id: &str,
        _from_ordinal: u64,
    ) -> Result<Vec<SealedEnvelope>, RecordsLogError> {
        Err(self.fault())
    }

    fn head(&self, _tenant_id: &str) -> Result<u64, RecordsLogError> {
        Err(self.fault())
    }
}

/// Boot a process normally, then replace every tenant's action log with one
/// that fails. Composing first is deliberate: boot must succeed exactly as it
/// does in production, so the fault is a runtime one rather than a
/// configuration the process would have refused.
pub fn state_with_a_failing_log(
    config: &foundry_ontology_app::Config,
    detail: &'static str,
) -> AppState {
    state_with_a_failing_log_from(config.clone(), detail)
}

pub fn state_with_a_failing_log_from(
    config: foundry_ontology_app::Config,
    detail: &'static str,
) -> AppState {
    let mut state = compose(&config).expect("boots");
    for tenant in state.tenants.values_mut() {
        tenant.get_mut().action_log = Box::new(AlwaysFailingLog { detail });
    }
    state
}

/// A log whose head fails ONCE and then answers.
///
/// An always-failing log cannot tell a single observation from several: every
/// pass agrees, so three passes and one pass look identical. A transient
/// failure is the shape a released lock or a flaky store actually has, and it
/// is the shape that separates them — under three passes the first sees
/// the tenant as unsampleable and the second reads it fine, so the exposition
/// reports `lag 0` beside `unknown 0`, a pair describing no state the process
/// was ever in.
pub struct HeadFailsOnceLog {
    pub head_ordinal: u64,
    calls: AtomicUsize,
}

impl HeadFailsOnceLog {
    pub fn new(head_ordinal: u64) -> Self {
        Self {
            head_ordinal,
            calls: AtomicUsize::new(0),
        }
    }
}

impl RecordsLog for HeadFailsOnceLog {
    fn append(&mut self, _envelope: ActionEnvelope) -> Result<Receipt, RecordsLogError> {
        Err(RecordsLogError::Storage {
            detail: "not the subject of this double".to_owned(),
        })
    }

    fn replay(
        &self,
        _tenant_id: &str,
        _from_ordinal: u64,
    ) -> Result<Vec<SealedEnvelope>, RecordsLogError> {
        Ok(Vec::new())
    }

    fn head(&self, _tenant_id: &str) -> Result<u64, RecordsLogError> {
        if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            return Err(RecordsLogError::Storage {
                detail: "the first read fails, later reads succeed".to_owned(),
            });
        }
        Ok(self.head_ordinal)
    }
}

pub fn state_with_a_transiently_failing_head(
    config: &foundry_ontology_app::Config,
    head_ordinal: u64,
) -> AppState {
    let mut state = compose(config).expect("boots");
    for tenant in state.tenants.values_mut() {
        tenant.get_mut().action_log = Box::new(HeadFailsOnceLog::new(head_ordinal));
    }
    state
}

/// Fail ONE tenant's log, leaving the rest readable.
///
/// A whole-roster fault cannot produce a mixed state: an unreadable tenant
/// contributes no lag, so `unreadable > 0 && lag > 0` needs one tenant that
/// is read and behind alongside one that cannot be read at all.
pub fn state_with_one_failing_tenant(
    config: &foundry_ontology_app::Config,
    tenant_id: &str,
    detail: &'static str,
) -> AppState {
    let mut state = compose(config).expect("boots");
    let tenant = state
        .tenants
        .get_mut(tenant_id)
        .expect("the tenant to fault must be served");
    tenant.get_mut().action_log = Box::new(AlwaysFailingLog { detail });
    state
}
