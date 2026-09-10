use foundry_projection_draft::{
    AppliedEntry, ApplyReceipt, EntryOutcome, KeyDesignations, ProjectedObject,
    ProjectionStoreError,
};
use rusqlite::{Connection, OptionalExtension, params};

use crate::codec::{encode_entry, encode_object};
use crate::{codec_error, key_law, require_trimmed, scan, storage};

pub(crate) fn apply(
    connection: &mut Connection,
    entry: AppliedEntry,
    keys: &KeyDesignations,
) -> Result<ApplyReceipt, ProjectionStoreError> {
    validate(&entry)?;
    let entry_bytes = encode_entry(&entry).map_err(codec_error)?;
    let transaction = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(storage)?;
    let head: u64 = transaction
        .query_row(
            "SELECT applied_ordinal FROM projection_heads WHERE tenant_id = ?1",
            params![entry.tenant_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(storage)?
        .unwrap_or(0);
    if entry.ordinal <= head {
        let stored: Option<Vec<u8>> = transaction
            .query_row(
                "SELECT entry_bytes FROM projection_entries
                     WHERE tenant_id = ?1 AND ordinal = ?2",
                params![entry.tenant_id, entry.ordinal],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?;
        return match stored {
            Some(bytes) if bytes == entry_bytes => Ok(ApplyReceipt {
                ordinal: entry.ordinal,
                deduplicated: true,
            }),
            _ => Err(ProjectionStoreError::DivergentReplay {
                ordinal: entry.ordinal,
            }),
        };
    }
    if entry.ordinal != head + 1 {
        return Err(ProjectionStoreError::NonDenseOrdinal {
            expected: head + 1,
            found: entry.ordinal,
        });
    }
    match &entry.outcome {
        EntryOutcome::Applied { objects, links } => {
            if !keys.is_empty() {
                for (index, object) in objects.iter().enumerate() {
                    key_law::check(
                        &transaction,
                        &entry.tenant_id,
                        object,
                        keys,
                        &objects[..index],
                    )?;
                }
            }
            for object in objects {
                write_object(&transaction, &entry.tenant_id, object)?;
            }
            for edge in links {
                transaction
                    .execute(
                        "INSERT OR REPLACE INTO projection_links
                         (tenant_id, from_ref, link_type, to_ref, observed_at_epoch_ms)
                         VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![
                            entry.tenant_id,
                            edge.from_object_ref,
                            edge.link_type,
                            edge.to_object_ref,
                            edge.observed_at_epoch_ms
                        ],
                    )
                    .map_err(storage)?;
            }
        }
        EntryOutcome::Poisoned { reason } => {
            transaction
                .execute(
                    "INSERT INTO projection_poisons (tenant_id, ordinal, reason)
                         VALUES (?1, ?2, ?3)",
                    params![entry.tenant_id, entry.ordinal, reason],
                )
                .map_err(storage)?;
        }
    }
    transaction
        .execute(
            "INSERT INTO projection_entries (tenant_id, ordinal, entry_bytes)
                 VALUES (?1, ?2, ?3)",
            params![entry.tenant_id, entry.ordinal, entry_bytes],
        )
        .map_err(storage)?;
    transaction
        .execute(
            "INSERT INTO projection_heads (tenant_id, applied_ordinal) VALUES (?1, ?2)
                 ON CONFLICT (tenant_id) DO UPDATE SET applied_ordinal = ?2",
            params![entry.tenant_id, entry.ordinal],
        )
        .map_err(storage)?;
    transaction.commit().map_err(storage)?;
    Ok(ApplyReceipt {
        ordinal: entry.ordinal,
        deduplicated: false,
    })
}

fn validate(entry: &AppliedEntry) -> Result<(), ProjectionStoreError> {
    require_trimmed(&entry.tenant_id, "blank entry tenant")?;
    match &entry.outcome {
        EntryOutcome::Applied { objects, .. } => {
            for object in objects {
                if object.entity.tenant_id != entry.tenant_id {
                    return Err(ProjectionStoreError::Entry {
                        detail: "object outside the entry's tenant",
                    });
                }
                require_trimmed(&object.last_actor, "blank object actor")?;
            }
        }
        EntryOutcome::Poisoned { reason } => {
            require_trimmed(reason, "blank poison reason")?;
        }
    }
    Ok(())
}

fn write_object(
    transaction: &rusqlite::Transaction<'_>,
    tenant_id: &str,
    object: &ProjectedObject,
) -> Result<(), ProjectionStoreError> {
    let bytes = encode_object(object).map_err(codec_error)?;
    let entity_type = object.entity.entity_type.value.as_str();
    transaction
        .execute(
            "INSERT INTO projection_objects (tenant_id, object_ref, entity_type, object_bytes)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (tenant_id, object_ref)
             DO UPDATE SET entity_type = ?3, object_bytes = ?4",
            params![tenant_id, object.entity.id, entity_type, bytes],
        )
        .map_err(storage)?;
    transaction
        .execute(
            "DELETE FROM projection_property_index WHERE tenant_id = ?1 AND object_ref = ?2",
            params![tenant_id, object.entity.id],
        )
        .map_err(storage)?;
    for (name, property) in &object.entity.properties {
        let value = &property.value.value;
        transaction
            .execute(
                "INSERT INTO projection_property_index
                 (tenant_id, object_ref, entity_type, property, value_kind,
                  int_value, real_value, text_value)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    tenant_id,
                    object.entity.id,
                    entity_type,
                    name,
                    value.type_label(),
                    scan::int_key(value),
                    scan::real_key(value),
                    value.as_str(),
                ],
            )
            .map_err(storage)?;
    }
    Ok(())
}
