use crate::envelope::{ActionEnvelope, Receipt, SealedEnvelope};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecordsLogError {
    /// The idempotency key was already spent on a different envelope. The log
    /// fails loudly rather than silently deduplicating divergent content.
    IdempotencyConflict {
        tenant_id: String,
        idempotency_key: String,
    },
    /// Adapter-level failure (I/O, corruption); the message is diagnostic.
    Storage { detail: String },
}

/// A per-tenant, append-only Action log.
///
/// The executable meaning of this contract is [`crate::conformance`]; an
/// adapter that passes the suite implements the port.
pub trait RecordsLog {
    /// Append one envelope. Re-appending a byte-identical envelope under the
    /// same idempotency key returns the original receipt marked deduplicated.
    fn append(&mut self, envelope: ActionEnvelope) -> Result<Receipt, RecordsLogError>;

    /// Every sealed envelope for the tenant with `ordinal >= from_ordinal`,
    /// in ordinal order. Replay from beyond the head is empty, not an error.
    fn replay(
        &self,
        tenant_id: &str,
        from_ordinal: u64,
    ) -> Result<Vec<SealedEnvelope>, RecordsLogError>;

    /// The tenant's highest ordinal; zero when the tenant has no envelopes.
    fn head(&self, tenant_id: &str) -> Result<u64, RecordsLogError>;
}
