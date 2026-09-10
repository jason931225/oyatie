//! Transitional shim: authorization evaluation is owned by `policy/`, not by
//! Identity. Depend on `policy-cedar-domain` directly the next time you open a
//! consumer, and delete this crate once the last one has moved.

pub use policy_cedar_domain::*;
