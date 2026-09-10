//! The tenant is the CREDENTIAL's on both migration surfaces. A plan whose
//! own `tenant_id` disagrees is refused rather than rewritten.
//!
//! That check is DEFENCE IN DEPTH, not the control that stops a cross-tenant
//! access: the PDP refuses a caller whose credential does not carry the
//! tenant, and `tenant_of` resolves by `caller.tenant_id` unconditionally, so
//! deleting the check would still refuse a foreign plan — with a worse
//! diagnostic, which is the reason to keep it.

pub(crate) mod attest;
pub(crate) mod run;

use serde::Deserialize;

use foundry_edits::WireDouble;
use foundry_spine::{
    DefaultValue, MigrationPlan, UpcastTransform, ValueConversion, migration_attestation,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlanRequest {
    pub(crate) tenant_id: String,              // data_class: INTERNAL_ONLY
    pub(crate) entity_type: String,            // data_class: INTERNAL_ONLY
    pub(crate) from_revision: u32,             // data_class: INTERNAL_ONLY
    pub(crate) to_revision: u32,               // data_class: INTERNAL_ONLY
    pub(crate) action_type: String,            // data_class: INTERNAL_ONLY
    pub(crate) audit_event_type: String,       // data_class: INTERNAL_ONLY
    pub(crate) declared_at_epoch_seconds: u64, // data_class: INTERNAL_ONLY
    #[serde(default)]
    pub(crate) transforms: Vec<WireTransform>, // data_class: INTERNAL_ONLY
}

/// The transform vocabulary, mirrored on the wire.
///
/// The spine's `UpcastTransform` carries no serde, and giving it some would
/// make a kernel type's field names a public wire contract that could not
/// then be refactored. The mirror is the seam: it is a facade concern that
/// this vocabulary is expressible in JSON at all, and the mapping below is
/// where a wire value becomes a domain one — including the refusals, which
/// a derive would have had nowhere to put.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum WireTransform {
    CopyAs {
        from: String, // data_class: INTERNAL_ONLY
        to: String,   // data_class: INTERNAL_ONLY
    },
    ConvertAs {
        from: String,       // data_class: INTERNAL_ONLY
        to: String,         // data_class: INTERNAL_ONLY
        conversion: String, // data_class: INTERNAL_ONLY
    },
    DefaultTo {
        to: String,         // data_class: INTERNAL_ONLY
        value: WireDefault, // data_class: PROPERTY_VALUE_PRIVACY_CLASS
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum WireDefault {
    String { value: String },        // data_class: PROPERTY_VALUE_PRIVACY_CLASS
    Integer { value: i64 },          // data_class: PROPERTY_VALUE_PRIVACY_CLASS
    Boolean { value: bool },         // data_class: PROPERTY_VALUE_PRIVACY_CLASS
    Double { value: f64 },           // data_class: PROPERTY_VALUE_PRIVACY_CLASS
    Timestamp { epoch_millis: i64 }, // data_class: PROPERTY_VALUE_PRIVACY_CLASS
}

impl WireDefault {
    /// A non-finite double is refused here rather than canonicalised. NaN
    /// has no total order and no canonical bytes, so a plan carrying one
    /// could not produce a stable digest — `WireDouble::new` is the law and
    /// this surface reports it rather than routing around it.
    ///
    /// No JSON request can reach that refusal, and that is measured, not
    /// assumed: the grammar has no `NaN` or `Infinity` literal, and
    /// `serde_json` fails an out-of-range exponent (`1e400`) with "number
    /// out of range" rather than yielding an infinity. The arm stays because
    /// the law belongs to `WireDouble`, not to the codec that happens to
    /// front it today; a second codec would arrive to find it already held.
    fn into_domain(self) -> Result<DefaultValue, &'static str> {
        Ok(match self {
            WireDefault::String { value } => DefaultValue::String(value),
            WireDefault::Integer { value } => DefaultValue::Integer(value),
            WireDefault::Boolean { value } => DefaultValue::Boolean(value),
            WireDefault::Double { value } => DefaultValue::Double(
                WireDouble::new(value).map_err(|_| "a default double must be finite")?,
            ),
            WireDefault::Timestamp { epoch_millis } => DefaultValue::Timestamp { epoch_millis },
        })
    }
}

impl WireTransform {
    fn into_domain(self) -> Result<UpcastTransform, &'static str> {
        Ok(match self {
            WireTransform::CopyAs { from, to } => UpcastTransform::CopyAs { from, to },
            WireTransform::ConvertAs {
                from,
                to,
                conversion,
            } => UpcastTransform::ConvertAs {
                from,
                to,
                conversion: match conversion.as_str() {
                    "integer_to_string" => ValueConversion::IntegerToString,
                    "boolean_to_integer" => ValueConversion::BooleanToInteger,
                    _ => return Err("that conversion is not one this process performs"),
                },
            },
            WireTransform::DefaultTo { to, value } => UpcastTransform::DefaultTo {
                to,
                value: value.into_domain()?,
            },
        })
    }
}
