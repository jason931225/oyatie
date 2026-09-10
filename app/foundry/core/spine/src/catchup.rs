//! Catch-up: bring a durable projection to its log's head by refolding
//! from the store's own `applied_head`.
//!
//! **Catch-up resumes INCLUSIVE of the store's own head entry.** The
//! re-apply dedups when the store really does hold this log, and is
//! refused when it does not. Without it, catch-up would happily top up
//! a store built from a DIFFERENT log and leave it reporting
//! `applied_head == log head` over rows that are not `fold(log)`.
//!
//! No check here proves the whole prefix: a divergence that changes an
//! applied outcome without changing any poison still passes. The only
//! full audit is a rebuild from empty, via
//! [`foundry_projection_draft::ProjectionStore::reset_tenant`].
//!
//! **Precondition: `registry` should be the snapshot the store was
//! built under.** Nothing here compares snapshots: an evolution
//! surfaces only where outcomes CHANGED, and every refusal reads
//! equally as a different log or a different registry. After one,
//! rebuild from empty and run this function over the whole log.

use data_ontology_kernel::OntologyEngine;
use foundry_projection_draft::{ProjectionStore, ProjectionStoreError};
use foundry_records_draft::SealedEnvelope;

use crate::emission::poison_label;
use crate::fold::fold_from_scratch;
use crate::state::ProjectionState;
use crate::writethrough::{WriteThroughError, project_through};

/// Where a catch-up began, and where it left the store.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CaughtUp {
    pub resumed_from: u64,
    /// The store's head afterwards, READ BACK from the store rather
    /// than computed, so this is the same number the store will report
    /// to anyone else who asks.
    pub head: u64,
    /// Whether the resume point was re-applied and agreed with the log.
    /// False only for a rebuild from empty, which has no resume point.
    /// This is a fact rather than arithmetic: a non-empty store whose
    /// resume point is absent from the log is refused
    /// ([`CatchUpError::ResumePointMissingFromLog`]) before any mirror
    /// runs, so reaching a `CaughtUp` with `resumed_from > 0` means that
    /// entry WAS re-applied and the store accepted it as a duplicate.
    pub revalidated: bool,
}

/// Why a catch-up refused. None of these is a poison: a poison derives
/// from (log bytes, registry snapshot) and repeats identically on every
/// replay, and not one of these does.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CatchUpError {
    /// The store could not be read — its head, or its poison ledger.
    /// Never read as zero or as empty: those readings would rebuild the
    /// whole log over contents we cannot see, turning an infrastructure
    /// blip into a destructive operation.
    Read(ProjectionStoreError),
    /// The store claims more of the log than the log holds. Serving it
    /// would answer from rows the log can no longer justify, so it is a
    /// refusal rather than a no-op.
    StoreAheadOfLog { store_head: u64, log_head: u64 },
    /// The log does not begin at ordinal 1. Resume state is rebuilt by
    /// folding everything BELOW the store's head, so a log that starts
    /// later cannot produce it — the information is gone. `replay(from)`
    /// hands back a slice, which makes this the easy caller mistake, and
    /// left unchecked it mirrors entries against a fresh fold and writes
    /// poisons that derive from where the caller cut rather than from
    /// the log.
    LogDoesNotStartAtOne { first_ordinal: u64 },
    /// The log has no entry at the store's head, so the resume point
    /// cannot be re-applied and nothing validates the store against
    /// this log. Distinct from a divergent resume point: that one
    /// disagrees, this one is absent.
    ResumePointMissingFromLog { ordinal: u64 },
    /// An entry in the log belongs to another tenant. The fold would
    /// poison it [`crate::PoisonReason::TenantMismatch`] — correctly,
    /// but that spends THIS tenant's ordinals and writes the poisons
    /// into its ledger — a head that reports caught-up over entries this
    /// tenant never wrote, and its own log refused forever afterwards.
    /// A wholly foreign log leaves the store holding nothing at all; a
    /// log with foreign entries mixed in leaves it holding the rest.
    /// `store == fold(log)` would technically hold either way; the harm
    /// is that the log was never this tenant's to fold.
    ForeignTenantEntry { ordinal: u64 },
    /// The entry the store stopped at is not this log's entry at that
    /// ordinal: the store belongs to a different log.
    DivergentResumePoint { ordinal: u64 },
    /// The poisons the store holds below its head are not the poisons
    /// this (log, registry) produces there. Two readings, and the
    /// refusal cannot tell them apart: the store was built from a
    /// different log, or it was built under a different registry
    /// snapshot. Either way resuming would leave a mixture — part of
    /// the projection folded under one input, part under another, with
    /// `applied_head` equal to the log head and nothing reporting that
    /// `store == fold(log, registry)` is false.
    DivergentPrefixPoisons {
        store_holds: Vec<(u64, String)>,
        log_produces: Vec<(u64, String)>,
    },
    /// The mirror halted on a store refusal, which names its ordinal.
    Mirror(WriteThroughError),
}

/// Fold `log` into `store` from wherever the store already stands.
///
/// `log` must be the tenant's **complete** log, in ordinal order, from
/// ordinal 1 — not the slice after the store's head. Resume state is
/// rebuilt by folding everything below that head, and state at ordinal
/// K cannot be rebuilt from a log beginning at K. `RecordsLog::replay`
/// takes a `from` argument, so passing `applied_head` (or `+ 1`) is the
/// natural mistake; both are refused rather than silently mirrored
/// against a fresh fold. A rebuild from scratch is not a separate code
/// path — it is this function against an empty store.
pub fn catch_up(
    tenant_id: &str,
    registry: &OntologyEngine,
    store: &mut dyn ProjectionStore,
    log: &[SealedEnvelope],
) -> Result<CaughtUp, CatchUpError> {
    let resumed_from = store.applied_head(tenant_id).map_err(CatchUpError::Read)?;
    if let Some(first) = log.first()
        && first.receipt.ordinal != 1
    {
        return Err(CatchUpError::LogDoesNotStartAtOne {
            first_ordinal: first.receipt.ordinal,
        });
    }
    if let Some(foreign) = log
        .iter()
        .find(|sealed| sealed.envelope.tenant_id != tenant_id)
    {
        return Err(CatchUpError::ForeignTenantEntry {
            ordinal: foreign.receipt.ordinal,
        });
    }
    let log_head = log.last().map_or(0, |sealed| sealed.receipt.ordinal);
    if resumed_from > log_head {
        return Err(CatchUpError::StoreAheadOfLog {
            store_head: resumed_from,
            log_head,
        });
    }
    let resume_at = resume_at_inclusive(resumed_from);
    let split = log
        .iter()
        .position(|sealed| sealed.receipt.ordinal >= resume_at)
        .unwrap_or(log.len());
    if resumed_from > 0 && log.get(split).map(|sealed| sealed.receipt.ordinal) != Some(resumed_from)
    {
        return Err(CatchUpError::ResumePointMissingFromLog {
            ordinal: resumed_from,
        });
    }
    // Rebuild the in-memory fold to exactly what the store durably
    // holds, so every entry mirrored from here carries the same bytes it
    // carried the first time — which is what lets the store recognise a
    // re-apply as a duplicate rather than as divergence.
    let mut state = fold_from_scratch(tenant_id, registry, log[..split].iter());
    check_prefix_agrees(tenant_id, store, &state, resume_at)?;
    project_through(&mut state, store, &log[split..]).map_err(|error| match error {
        WriteThroughError::Store {
            ordinal,
            error: ProjectionStoreError::DivergentReplay { .. },
        } if ordinal == resumed_from => CatchUpError::DivergentResumePoint { ordinal },
        other => CatchUpError::Mirror(other),
    })?;
    let head = store.applied_head(tenant_id).map_err(CatchUpError::Read)?;
    Ok(CaughtUp {
        resumed_from,
        head,
        revalidated: resumed_from > 0,
    })
}

fn resume_at_inclusive(store_head: u64) -> u64 {
    store_head.max(1)
}

/// The poisons the store holds below its head must be the poisons this
/// (log, registry) produces there.
///
/// Revalidating the head entry cannot see a divergence beneath it: one
/// envelope is one object, so a head entry's mirrored bytes can be
/// IDENTICAL under two logs that disagree earlier — it dedups, and the
/// store finishes at the log's head holding a prefix from somewhere
/// else. The prefix fold is already computed to rebuild the resume
/// state, so this costs one additional store read and no additional
/// folding.
///
/// It does not close every case: a divergence that changes an applied
/// outcome without changing any poison still passes. That limit, and
/// its remedy, are the module's.
fn check_prefix_agrees(
    tenant_id: &str,
    store: &dyn ProjectionStore,
    state: &ProjectionState,
    resume_at: u64,
) -> Result<(), CatchUpError> {
    let log_produces: Vec<(u64, String)> = state
        .poison
        .iter()
        .map(|(ordinal, reason)| (*ordinal, poison_label(reason).to_owned()))
        .collect();
    let mut store_holds: Vec<(u64, String)> = store
        .poisoned(tenant_id)
        .map_err(CatchUpError::Read)?
        .into_iter()
        .filter(|(ordinal, _)| *ordinal < resume_at)
        .collect();
    store_holds.sort();
    if store_holds == log_produces {
        return Ok(());
    }
    Err(CatchUpError::DivergentPrefixPoisons {
        store_holds,
        log_produces,
    })
}
