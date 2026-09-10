//! The discard path: empty one tenant's projection so a rebuild can
//! start from nothing.

use foundry_projection_draft::ProjectionStoreError;
use rusqlite::{Connection, OptionalExtension};

use crate::{require_trimmed, storage};

pub(crate) fn reset_tenant(
    connection: &mut Connection,
    tenant_id: &str,
) -> Result<u64, ProjectionStoreError> {
    require_trimmed(tenant_id, "blank tenant")?;
    // One immediate transaction, like an apply. THE TRANSACTION is what
    // makes a crash safe — an uncommitted discard rolls back whole — so
    // statement order carries no crash-recovery meaning and no claim is
    // made for it. A half-applied discard would leave precisely the
    // mixture this was called to escape, which is why it cannot be a
    // sequence of independent deletes.
    let transaction = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(storage)?;
    let discarded: i64 = transaction
        .query_row(
            "SELECT applied_ordinal FROM projection_heads WHERE tenant_id = ?1",
            [tenant_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(storage)?
        .unwrap_or(0);
    let discarded = u64::try_from(discarded).map_err(|_| ProjectionStoreError::Storage {
        detail: format!("applied head is not a valid ordinal: {discarded}"),
    })?;
    for table in [
        "projection_entries",
        "projection_objects",
        "projection_property_index",
        "projection_links",
        "projection_poisons",
        "projection_heads",
    ] {
        transaction
            .execute(
                &format!("DELETE FROM {table} WHERE tenant_id = ?1"),
                [tenant_id],
            )
            .map_err(storage)?;
    }
    transaction.commit().map_err(storage)?;
    Ok(discarded)
}
