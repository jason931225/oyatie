//! Foundry edits: the wire plane of the M3 write spine.
//!
//! Log bytes must never track another crate's in-memory shape, so this
//! crate depends on NOTHING — the missing edge is the compiler-enforced
//! guarantee.
#![forbid(unsafe_code)]
// ADR-0083 Tier 3: tests legitimately use `.unwrap()` / `.expect()` /
// `panic!()` to assert invariants under the `cfg(test)` exemption.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

mod decode;
mod edit;
mod encode;
mod property;
mod record;
mod value;

pub use decode::{DecodeError, decode_action_record, decode_denial_record};
pub use edit::{EditError, EditSet, EditTag, OntologyEdit};
pub use encode::{encode_action_record, encode_denial_record};
pub use property::{WireDataClass, WireProperty, WirePropertyError, WireTier};
pub use record::{ActionRecord, DenialRecord, RecordError, WIRE_FORMAT_VERSION};
pub use value::{WireDate, WireDouble, WireValue, WireValueError};
