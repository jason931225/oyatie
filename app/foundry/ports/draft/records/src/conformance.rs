//! The executable contract: every adapter runs this suite unchanged.
//!
//! Checks return `Err(String)` naming the violated clause rather than
//! panicking, so an adapter's test can attribute a failure to the contract
//! line it broke.

use crate::envelope::ActionEnvelope;
use crate::log::RecordsLog;

pub trait RecordsFixture {
    type Log: RecordsLog;

    fn log(&mut self) -> &mut Self::Log;

    /// Close and reopen the underlying store; `false` means the fixture is
    /// volatile and durability cannot be checked against it.
    fn reopen(&mut self) -> bool;
}

fn envelope(tenant: &str, object: &str, key: &str) -> ActionEnvelope {
    ActionEnvelope::new(tenant, object, "upsert", key, 1, b"{}".to_vec(), 1)
        .expect("conformance fixtures construct valid envelopes")
}

fn fail(clause: &str, detail: String) -> String {
    format!("{clause}: {detail}")
}

pub fn check_append_assigns_dense_per_tenant_ordinals<F: RecordsFixture>(
    fixture: &mut F,
) -> Result<(), String> {
    let log = fixture.log();
    for (i, key) in ["k1", "k2", "k3"].iter().enumerate() {
        let receipt = log
            .append(envelope("ten_a", "obj:1", key))
            .map_err(|e| fail("append", format!("{e:?}")))?;
        if receipt.ordinal != i as u64 + 1 {
            return Err(fail(
                "ordinals are dense and start at one",
                format!("append {} got ordinal {}", i + 1, receipt.ordinal),
            ));
        }
    }
    let other = log
        .append(envelope("ten_b", "obj:1", "k1"))
        .map_err(|e| fail("append", format!("{e:?}")))?;
    if other.ordinal != 1 {
        return Err(fail(
            "ordinal streams are per tenant",
            format!(
                "first append for a second tenant got ordinal {}",
                other.ordinal
            ),
        ));
    }
    Ok(())
}

pub fn check_object_sequences_are_dense_per_object<F: RecordsFixture>(
    fixture: &mut F,
) -> Result<(), String> {
    let log = fixture.log();
    let seq = |r: crate::Receipt| (r.ordinal, r.object_sequence);
    let a1 = seq(log
        .append(envelope("ten_a", "obj:a", "k1"))
        .map_err(|e| format!("{e:?}"))?);
    let b1 = seq(log
        .append(envelope("ten_a", "obj:b", "k2"))
        .map_err(|e| format!("{e:?}"))?);
    let a2 = seq(log
        .append(envelope("ten_a", "obj:a", "k3"))
        .map_err(|e| format!("{e:?}"))?);
    if a1 != (1, 1) || b1 != (2, 1) || a2 != (3, 2) {
        return Err(fail(
            "object sequences are dense per (tenant, object_ref)",
            format!("got {a1:?} {b1:?} {a2:?}, want (1,1) (2,1) (3,2)"),
        ));
    }
    Ok(())
}

pub fn check_idempotent_replay_returns_the_original_receipt<F: RecordsFixture>(
    fixture: &mut F,
) -> Result<(), String> {
    let log = fixture.log();
    let first = log
        .append(envelope("ten_a", "obj:1", "same-key"))
        .map_err(|e| format!("{e:?}"))?;
    let again = log
        .append(envelope("ten_a", "obj:1", "same-key"))
        .map_err(|e| fail("idempotent re-append must succeed", format!("{e:?}")))?;
    if !again.deduplicated {
        return Err(fail(
            "re-append is marked deduplicated",
            format!("{again:?}"),
        ));
    }
    if (again.ordinal, again.object_sequence) != (first.ordinal, first.object_sequence) {
        return Err(fail(
            "a deduplicated receipt restates the original position",
            format!("first {first:?}, again {again:?}"),
        ));
    }
    let head = log.head("ten_a").map_err(|e| format!("{e:?}"))?;
    if head != 1 {
        return Err(fail(
            "a deduplicated append appends nothing",
            format!("head is {head}"),
        ));
    }
    Ok(())
}

pub fn check_conflicting_idempotency_key_reuse_is_refused<F: RecordsFixture>(
    fixture: &mut F,
) -> Result<(), String> {
    let log = fixture.log();
    log.append(envelope("ten_a", "obj:1", "spent-key"))
        .map_err(|e| format!("{e:?}"))?;
    match log.append(envelope("ten_a", "obj:DIFFERENT", "spent-key")) {
        Err(crate::RecordsLogError::IdempotencyConflict { .. }) => Ok(()),
        other => Err(fail(
            "a spent key with divergent content fails loudly",
            format!("{other:?}"),
        )),
    }
}

pub fn check_replay_returns_envelopes_in_order<F: RecordsFixture>(
    fixture: &mut F,
) -> Result<(), String> {
    let log = fixture.log();
    for key in ["k1", "k2", "k3", "k4"] {
        log.append(envelope("ten_a", "obj:1", key))
            .map_err(|e| format!("{e:?}"))?;
    }
    let tail = log.replay("ten_a", 3).map_err(|e| format!("{e:?}"))?;
    let ordinals: Vec<u64> = tail.iter().map(|sealed| sealed.receipt.ordinal).collect();
    if ordinals != [3, 4] {
        return Err(fail(
            "replay(from) is inclusive and ordered",
            format!("{ordinals:?}"),
        ));
    }
    let beyond = log.replay("ten_a", 9).map_err(|e| format!("{e:?}"))?;
    if !beyond.is_empty() {
        return Err(fail(
            "replay beyond the head is empty, not an error",
            format!("{beyond:?}"),
        ));
    }
    Ok(())
}

pub fn check_replay_is_tenant_isolated<F: RecordsFixture>(fixture: &mut F) -> Result<(), String> {
    let log = fixture.log();
    log.append(envelope("ten_a", "obj:1", "k1"))
        .map_err(|e| format!("{e:?}"))?;
    log.append(envelope("ten_b", "obj:1", "k1"))
        .map_err(|e| format!("{e:?}"))?;
    let replayed = log.replay("ten_b", 1).map_err(|e| format!("{e:?}"))?;
    if replayed.len() != 1 || replayed[0].envelope.tenant_id != "ten_b" {
        return Err(fail(
            "replay never crosses tenants",
            format!("tenant b saw {} envelopes", replayed.len()),
        ));
    }
    Ok(())
}

pub fn check_head_tracks_the_last_ordinal<F: RecordsFixture>(
    fixture: &mut F,
) -> Result<(), String> {
    let log = fixture.log();
    if log.head("ten_a").map_err(|e| format!("{e:?}"))? != 0 {
        return Err(fail("an empty tenant has head zero", String::new()));
    }
    log.append(envelope("ten_a", "obj:1", "k1"))
        .map_err(|e| format!("{e:?}"))?;
    log.append(envelope("ten_a", "obj:1", "k2"))
        .map_err(|e| format!("{e:?}"))?;
    let head = log.head("ten_a").map_err(|e| format!("{e:?}"))?;
    if head != 2 {
        return Err(fail("head is the last assigned ordinal", format!("{head}")));
    }
    Ok(())
}

pub fn check_durability_across_reopen<F: RecordsFixture>(fixture: &mut F) -> Result<(), String> {
    let before = {
        let log = fixture.log();
        log.append(envelope("ten_a", "obj:1", "k1"))
            .map_err(|e| format!("{e:?}"))?;
        log.append(envelope("ten_a", "obj:2", "k2"))
            .map_err(|e| format!("{e:?}"))?;
        log.replay("ten_a", 1).map_err(|e| format!("{e:?}"))?
    };
    if !fixture.reopen() {
        // Volatile fixture: the check proves nothing, and says so by proving
        // nothing rather than by passing vacuously against lost state.
        return Ok(());
    }
    let after = fixture
        .log()
        .replay("ten_a", 1)
        .map_err(|e| format!("{e:?}"))?;
    if after != before {
        return Err(fail(
            "a reopened log replays byte-identically",
            format!("{} envelopes before, {} after", before.len(), after.len()),
        ));
    }
    Ok(())
}
