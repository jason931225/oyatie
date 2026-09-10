use data_ontology_kernel::{CalendarDate, PropertyValue};
use foundry_projection_draft::{
    Page, PageRequest, ProjectionCursor, ProjectionStoreError, PropertyPredicate,
};
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension, params};

use crate::codec::decode_object;
use crate::{require_trimmed, storage};

/// The monotone integer key for Integer-affinity kinds. Dates key as
/// `year * 10_000 + month * 100 + day`, which agrees with the kernel's
/// (year, month, day) lexicographic `Ord` — pinned by the adapter's
/// Date-range test so key drift is caught.
pub(crate) fn int_key(value: &PropertyValue) -> Option<i64> {
    match value {
        PropertyValue::Integer(number) => Some(*number),
        PropertyValue::Boolean(flag) => Some(i64::from(*flag)),
        PropertyValue::Date(date) => Some(date_key(*date)),
        PropertyValue::Timestamp { epoch_millis } => Some(*epoch_millis),
        _ => None,
    }
}

pub(crate) fn real_key(value: &PropertyValue) -> Option<f64> {
    match value {
        PropertyValue::Double(double) => Some(double.get()),
        _ => None,
    }
}

fn date_key(date: CalendarDate) -> i64 {
    i64::from(date.year()) * 10_000 + i64::from(date.month()) * 100 + i64::from(date.day())
}

pub(crate) fn scan(
    connection: &Connection,
    tenant_id: &str,
    entity_type: &str,
    predicate: Option<&PropertyPredicate>,
    page: &PageRequest,
) -> Result<Page, ProjectionStoreError> {
    require_trimmed(tenant_id, "blank tenant")?;
    if page.limit == 0 {
        return Err(ProjectionStoreError::Entry {
            detail: "zero page limit",
        });
    }
    if let Some(predicate) = predicate
        && let Some((property, kind)) = predicate.range_kind()
    {
        let drifted: Option<i64> = connection
            .query_row(
                "SELECT 1 FROM projection_property_index
                 WHERE tenant_id = ?1 AND entity_type = ?2 AND property = ?3
                   AND value_kind <> ?4
                 LIMIT 1",
                params![tenant_id, entity_type, property, kind],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?;
        if drifted.is_some() {
            return Err(ProjectionStoreError::KindMismatch {
                property: property.to_owned(),
            });
        }
    }
    let after = page
        .cursor
        .as_ref()
        .map(|cursor| cursor.after_object_ref.clone())
        .unwrap_or_default();
    let (sql, bindings) = candidate_query(tenant_id, entity_type, predicate, &after);
    let mut statement = connection.prepare(&sql).map_err(storage)?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(bindings), |row| {
            row.get::<_, Vec<u8>>(0)
        })
        .map_err(storage)?;
    let mut objects = Vec::new();
    let mut next = None;
    for row in rows {
        let bytes = row.map_err(storage)?;
        let object = decode_object(&bytes).map_err(|error| ProjectionStoreError::Storage {
            detail: format!("codec: {}", error.detail),
        })?;
        if let Some(predicate) = predicate
            && !predicate.matches(&object.entity)?
        {
            continue;
        }
        if objects.len() == page.limit {
            next = Some(ProjectionCursor {
                after_object_ref: objects
                    .last()
                    .map(|last: &foundry_projection_draft::ProjectedObject| last.entity.id.clone())
                    .unwrap_or_default(),
            });
            break;
        }
        objects.push(object);
    }
    Ok(Page { objects, next })
}

fn candidate_query(
    tenant_id: &str,
    entity_type: &str,
    predicate: Option<&PropertyPredicate>,
    after: &str,
) -> (String, Vec<SqlValue>) {
    let Some(predicate) = predicate else {
        return (
            "SELECT object_bytes FROM projection_objects
             WHERE tenant_id = ?1 AND entity_type = ?2 AND object_ref > ?3
             ORDER BY object_ref"
                .to_owned(),
            vec![
                SqlValue::Text(tenant_id.to_owned()),
                SqlValue::Text(entity_type.to_owned()),
                SqlValue::Text(after.to_owned()),
            ],
        );
    };
    let mut sql = String::from(
        "SELECT o.object_bytes FROM projection_property_index i
         JOIN projection_objects o
           ON o.tenant_id = i.tenant_id AND o.object_ref = i.object_ref
         WHERE i.tenant_id = ?1 AND i.entity_type = ?2 AND i.property = ?3
           AND i.value_kind = ?4 AND i.object_ref > ?5",
    );
    let (property, value_bindings) = match predicate {
        PropertyPredicate::Equals { property, value } => {
            let bindings = match (int_key(value), real_key(value), value.as_str()) {
                (Some(key), _, _) => {
                    sql.push_str(" AND i.int_value = ?6");
                    vec![SqlValue::Integer(key)]
                }
                (_, Some(key), _) => {
                    sql.push_str(" AND i.real_value = ?6");
                    vec![SqlValue::Real(key)]
                }
                (_, _, Some(text)) => {
                    sql.push_str(" AND i.text_value = ?6");
                    vec![SqlValue::Text(text.to_owned())]
                }
                _ => Vec::new(),
            };
            (property, bindings)
        }
        PropertyPredicate::Range { property, from, to } => {
            let bindings = match (int_key(from), int_key(to)) {
                (Some(low), Some(high)) => {
                    sql.push_str(" AND i.int_value BETWEEN ?6 AND ?7");
                    vec![SqlValue::Integer(low), SqlValue::Integer(high)]
                }
                _ => match (real_key(from), real_key(to)) {
                    (Some(low), Some(high)) => {
                        sql.push_str(" AND i.real_value BETWEEN ?6 AND ?7");
                        vec![SqlValue::Real(low), SqlValue::Real(high)]
                    }
                    _ => match (from.as_str(), to.as_str()) {
                        (Some(low), Some(high)) => {
                            sql.push_str(" AND i.text_value BETWEEN ?6 AND ?7");
                            vec![
                                SqlValue::Text(low.to_owned()),
                                SqlValue::Text(high.to_owned()),
                            ]
                        }
                        _ => Vec::new(),
                    },
                },
            };
            (property, bindings)
        }
    };
    sql.push_str(" ORDER BY i.object_ref");
    let kind = match predicate {
        PropertyPredicate::Equals { value, .. } => value.type_label(),
        PropertyPredicate::Range { from, .. } => from.type_label(),
    };
    let mut bindings = vec![
        SqlValue::Text(tenant_id.to_owned()),
        SqlValue::Text(entity_type.to_owned()),
        SqlValue::Text(property.to_owned()),
        SqlValue::Text(kind.to_owned()),
        SqlValue::Text(after.to_owned()),
    ];
    bindings.extend(value_bindings);
    (sql, bindings)
}
