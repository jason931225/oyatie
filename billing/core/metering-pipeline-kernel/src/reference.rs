//! Executable specification of the idempotent metering sink, proven by
//! [`crate::conformance`] in this crate's tests. NOT a production store: no
//! durability across process restart.

use std::collections::BTreeMap;
use std::sync::Mutex;

use crate::{
    DedupKey, IngestOutcome, LatenessPolicy, MeteringPipelineError, MeteringSink, UsageRecord,
};

#[derive(Debug)]
pub struct InMemorySink {
    policy: LatenessPolicy,
    records: Mutex<BTreeMap<DedupKey, UsageRecord>>,
}

impl InMemorySink {
    #[must_use]
    pub fn new(policy: LatenessPolicy) -> Self {
        Self {
            policy,
            records: Mutex::new(BTreeMap::new()),
        }
    }

    /// Exposed for operator and test visibility.
    ///
    /// # Errors
    /// Returns [`MeteringPipelineError::SinkUnavailable`] when the store
    /// lock is poisoned.
    pub fn len(&self) -> Result<usize, MeteringPipelineError> {
        Ok(self.locked()?.len())
    }

    /// # Errors
    /// Returns [`MeteringPipelineError::SinkUnavailable`] when the store
    /// lock is poisoned.
    pub fn is_empty(&self) -> Result<bool, MeteringPipelineError> {
        Ok(self.locked()?.is_empty())
    }

    fn locked(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, BTreeMap<DedupKey, UsageRecord>>, MeteringPipelineError>
    {
        self.records
            .lock()
            .map_err(|_| MeteringPipelineError::SinkUnavailable {
                detail: "reference sink lock poisoned".to_owned(),
            })
    }
}

impl Default for InMemorySink {
    fn default() -> Self {
        Self::new(LatenessPolicy::default())
    }
}

impl MeteringSink for InMemorySink {
    fn lateness_policy(&self) -> LatenessPolicy {
        self.policy
    }

    fn ingest(
        &self,
        record: UsageRecord,
        arrived_at_epoch_seconds: u64,
    ) -> Result<IngestOutcome, MeteringPipelineError> {
        let key = record.dedup_key();
        let mut records = self.locked()?;
        match records.get(&key) {
            None => {
                // Lateness admits first writes only; accepted replays must
                // remain idempotent after the window closes.
                self.policy
                    .admit(record.usage_hour, arrived_at_epoch_seconds)
                    .map_err(MeteringPipelineError::Rejected)?;
                records.insert(key, record);
                Ok(IngestOutcome::Recorded)
            }
            Some(existing)
                if existing.consumed_quantity_microunits == record.consumed_quantity_microunits =>
            {
                Ok(IngestOutcome::Duplicate)
            }
            Some(existing) => Err(MeteringPipelineError::QuantityConflict {
                key,
                recorded_microunits: existing.consumed_quantity_microunits,
                replayed_microunits: record.consumed_quantity_microunits,
            }),
        }
    }

    fn lookup(&self, key: &DedupKey) -> Result<Option<UsageRecord>, MeteringPipelineError> {
        Ok(self.locked()?.get(key).cloned())
    }
}
