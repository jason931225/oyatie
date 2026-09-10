//! Canonical encoding: the ONLY byte spelling of each value.
//!
//! No presence bytes: v1 has no optional wire field, and a presence byte
//! is the rule an optional field brings WITH it in the version that
//! first carries one. An existing version's layout is NEVER mutated;
//! evolution mints the next version.

use crate::edit::OntologyEdit;
use crate::property::WireProperty;
use crate::record::{ActionRecord, DenialRecord};
use crate::value::WireValue;

/// The canonical bytes of an [`ActionRecord`].
pub fn encode_action_record(record: &ActionRecord) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend(record.wire_format_version.to_le_bytes());
    string(&mut out, &record.principal_id);
    string(&mut out, &record.decision_id);
    string(&mut out, &record.audit_event_type);
    string(&mut out, &record.idempotency_key);
    out.extend(record.occurred_at_epoch_ms.to_le_bytes());
    properties(&mut out, &record.parameters);
    let edits = record.edits.edits();
    count(&mut out, edits.len());
    for item in edits {
        edit(&mut out, item);
    }
    out
}

/// The canonical bytes of a [`DenialRecord`].
pub fn encode_denial_record(record: &DenialRecord) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend(record.wire_format_version.to_le_bytes());
    string(&mut out, &record.gate);
    string(&mut out, &record.cause);
    string(&mut out, &record.principal_id);
    string(&mut out, &record.decision_id);
    string(&mut out, &record.action_id);
    string(&mut out, &record.object_ref);
    out.extend(record.occurred_at_epoch_ms.to_le_bytes());
    out
}

fn count(out: &mut Vec<u8>, n: usize) {
    let n = u32::try_from(n).unwrap_or(u32::MAX);
    out.extend(n.to_le_bytes());
}

fn string(out: &mut Vec<u8>, value: &str) {
    count(out, value.len());
    out.extend(value.as_bytes());
}

fn properties(out: &mut Vec<u8>, set: &[WireProperty]) {
    count(out, set.len());
    for property in set {
        string(out, &property.name);
        out.push(property.tier.tag());
        out.push(property.data_class.tag());
        value(out, &property.value);
    }
}

fn value(out: &mut Vec<u8>, item: &WireValue) {
    match item {
        WireValue::String(text) => {
            out.push(0);
            string(out, text);
        }
        WireValue::Integer(number) => {
            out.push(1);
            out.extend(number.to_le_bytes());
        }
        WireValue::Double(double) => {
            out.push(2);
            out.extend(double.sort_key().to_le_bytes());
        }
        WireValue::Boolean(flag) => {
            out.push(3);
            out.push(u8::from(*flag));
        }
        WireValue::Date(date) => {
            out.push(4);
            out.extend(date.year().to_le_bytes());
            out.push(date.month());
            out.push(date.day());
        }
        WireValue::Timestamp { epoch_millis } => {
            out.push(5);
            out.extend(epoch_millis.to_le_bytes());
        }
        WireValue::Array(items) => {
            out.push(6);
            count(out, items.len());
            for entry in items {
                value(out, entry);
            }
        }
        WireValue::Struct(entries) => {
            out.push(7);
            count(out, entries.len());
            for (key, entry) in entries {
                string(out, key);
                value(out, entry);
            }
        }
    }
}

fn edit(out: &mut Vec<u8>, item: &OntologyEdit) {
    out.push(item.tag().tag());
    match item {
        OntologyEdit::CreateObject {
            entity_type,
            properties: set,
        } => {
            string(out, entity_type);
            properties(out, set);
        }
        OntologyEdit::UpsertProperties { set } => properties(out, set),
        OntologyEdit::CreateLink {
            link_type,
            to_entity_id,
        } => {
            string(out, link_type);
            string(out, to_entity_id);
        }
    }
}
