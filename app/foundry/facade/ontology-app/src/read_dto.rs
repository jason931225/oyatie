use data_ontology_kernel::PropertyValue;
use serde::Serialize;

/// `?revision=N`, parsed from the RAW query string inside the handler.
///
/// It is deliberately NOT a typed extractor. An extractor — even one whose
/// every field is optional — still rejects a malformed value
/// (`?revision=abc`, `?revision=-1`, an out-of-range number, a duplicate
/// key) BEFORE the handler runs, which would answer an unauthenticated
/// stranger with a 400 describing the shape of the API instead of asking
/// for a credential. Parsing here is what actually makes
/// authenticate-then-authorize true on this route, rather than only
/// appearing to.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum RevisionPin {
    Pinned(u32),
    Unusable,
}

impl RevisionPin {
    /// Canonical form only: the key and value are matched literally and
    /// are NOT percent-decoded, so `?%72evision=1` and `?revision=%31`
    /// are `Unusable`. That is a real narrowing against a typed
    /// extractor, and it is deliberate — decoding here would be a second
    /// parser to keep honest. Every narrowed case lands as the same
    /// typed refusal as any other unusable shape, so it can mislead no
    /// caller and reveals nothing.
    pub(crate) fn parse(raw: Option<&str>) -> Self {
        let Some(raw) = raw else {
            return Self::Unusable;
        };
        let mut found: Option<u32> = None;
        for pair in raw.split('&') {
            let Some((key, value)) = pair.split_once('=') else {
                continue;
            };
            if key != "revision" {
                continue;
            }
            // A repeated key is unusable rather than last-wins: two
            // different pins in one request have no single honest answer.
            if found.is_some() {
                return Self::Unusable;
            }
            match value.parse::<u32>() {
                Ok(revision) => found = Some(revision),
                Err(_) => return Self::Unusable,
            }
        }
        found.map_or(Self::Unusable, Self::Pinned)
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct PropertyBody {
    pub(crate) value_type: &'static str, // data_class: INTERNAL_ONLY
    pub(crate) data_class: &'static str, // data_class: INTERNAL_ONLY
    pub(crate) value: serde_json::Value, // data_class: PROPERTY_VALUE_PRIVACY_CLASS
}

#[derive(Debug, Serialize)]
pub(crate) struct PinnedObjectBody {
    pub(crate) object_ref: String,         // data_class: TENANT_SCOPED
    pub(crate) written_revision: u32,      // data_class: INTERNAL_ONLY
    pub(crate) upcast_state: &'static str, // data_class: INTERNAL_ONLY
    pub(crate) properties: std::collections::BTreeMap<String, PropertyBody>, // data_class: PROPERTY_VALUE_PRIVACY_CLASS
}

/// The total kernel-value → JSON conversion. Every variant is named, so a
/// new kernel variant breaks this exhaustively at compile time rather than
/// silently falling back to a debug rendering.
pub(crate) fn json_value(value: &PropertyValue) -> serde_json::Value {
    match value {
        PropertyValue::String(text) => serde_json::Value::String(text.clone()),
        PropertyValue::Integer(number) => serde_json::Value::Number((*number).into()),
        PropertyValue::Double(double) => serde_json::Number::from_f64(double.get())
            .map_or(serde_json::Value::Null, serde_json::Value::Number),
        PropertyValue::Boolean(flag) => serde_json::Value::Bool(*flag),
        PropertyValue::Date(date) => serde_json::Value::String(format!(
            "{:04}-{:02}-{:02}",
            date.year(),
            date.month(),
            date.day()
        )),
        PropertyValue::Timestamp { epoch_millis } => {
            serde_json::Value::Number((*epoch_millis).into())
        }
        PropertyValue::Array(items) => {
            serde_json::Value::Array(items.iter().map(json_value).collect())
        }
        PropertyValue::Struct(fields) => serde_json::Value::Object(
            fields
                .iter()
                .map(|(name, inner)| (name.clone(), json_value(inner)))
                .collect(),
        ),
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct HistoryRow {
    pub(crate) ordinal: u64,             // data_class: INTERNAL_ONLY
    pub(crate) object_sequence: u64,     // data_class: INTERNAL_ONLY
    pub(crate) principal_id: String,     // data_class: TENANT_SCOPED
    pub(crate) decision_id: String,      // data_class: INTERNAL_ONLY
    pub(crate) action_type: String,      // data_class: INTERNAL_ONLY
    pub(crate) audit_event_type: String, // data_class: INTERNAL_ONLY
    pub(crate) schema_revision: u32,     // data_class: INTERNAL_ONLY
}

#[derive(Debug, Serialize)]
pub(crate) struct AuditRow {
    pub(crate) ordinal: u64,                  // data_class: INTERNAL_ONLY
    pub(crate) object_ref: String,            // data_class: TENANT_SCOPED
    pub(crate) action_type: String,           // data_class: INTERNAL_ONLY
    pub(crate) disposition: &'static str,     // data_class: INTERNAL_ONLY
    pub(crate) poison_reason: Option<String>, // data_class: INTERNAL_ONLY
    pub(crate) principal_id: Option<String>,  // data_class: TENANT_SCOPED
    pub(crate) decision_id: Option<String>,   // data_class: INTERNAL_ONLY
}

#[derive(Debug, Serialize)]
pub(crate) struct EntityTypeRow {
    pub(crate) entity_type: String,     // data_class: INTERNAL_ONLY
    pub(crate) revision: u32,           // data_class: INTERNAL_ONLY
    pub(crate) properties: Vec<String>, // data_class: INTERNAL_ONLY
}
