//! # port-engine-identity — the receipt's `engine_digest` axis.
//!
//! **This crate owns the ENCODING, not the enumeration.** Which crates make up the engine is a
//! question only the facade can answer without inverting the dependency direction — an adapter
//! reaching into `core/` and `facade/` to read their sources would point the hexagon backwards, and
//! would also put files outside this package into this target's inputs, which no package-relative
//! build glob can express. So the facade passes the sources in and this decides what hashing them
//! means.

#![forbid(unsafe_code)]

mod sources;
pub use sources::CRATE_SOURCES;

use port_engine_api::Digest;
use port_engine_hash::digest_bytes;

/// Embedded engine identity label: which engine, at which programme milestone.
///
/// Retained beside the source digest because the two answer different questions. The sources say
/// what the engine IS; this says what it calls itself, and a label that drifts from the code is
/// worth being able to see.
const ENGINE_IDENTITY_JSON: &str = include_str!("engine-identity-v0.json");

/// Preimage version. Bumping it changes every digest, which is the point: a change to how identity
/// is computed is itself an engine change and must be visible as one.
const ENGINE_PREIMAGE_VERSION: &str = "engine-preimage-v1";

/// One crate's contribution to the engine's identity: its name, then the sources it owns as
/// `(crate-relative path, file contents)` pairs.
pub type CrateSources<'a> = (&'a str, &'a [(&'a str, &'a str)]);

#[must_use]
pub const fn w0_ready() -> bool {
    true
}

#[must_use]
pub fn engine_digest(crates: &[CrateSources<'_>]) -> Digest {
    digest_bytes(&engine_preimage(crates))
}

/// The bytes [`engine_digest`] hashes.
///
/// LENGTH-PREFIXED, and that is not decoration. Concatenating a name to its contents lets two
/// different manifests produce identical bytes — a file named `a` holding `bc` against a file named
/// `ab` holding `c` — so a digest over the naive concatenation could hold while the engine changed.
/// Prefixing each field with its length makes the encoding injective, which is the same argument
/// the snapshot preimage makes for the same reason.
///
/// Counts are fields too, so a truncated manifest cannot hash as a shorter one.
#[must_use]
pub fn engine_preimage(crates: &[CrateSources<'_>]) -> Vec<u8> {
    let mut out = Vec::new();
    field(&mut out, ENGINE_PREIMAGE_VERSION.as_bytes());
    field(&mut out, ENGINE_IDENTITY_JSON.as_bytes());
    field(&mut out, crates.len().to_string().as_bytes());

    for (name, sources) in crates {
        field(&mut out, name.as_bytes());
        field(&mut out, sources.len().to_string().as_bytes());
        for (path, contents) in *sources {
            field(&mut out, path.as_bytes());
            field(&mut out, contents.as_bytes());
        }
    }
    out
}

fn field(out: &mut Vec<u8>, value: &[u8]) {
    out.extend_from_slice(value.len().to_string().as_bytes());
    out.push(b':');
    out.extend_from_slice(value);
    out.push(b'\n');
}

/// Borrow the embedded identity label (diagnostics / golden tests).
#[must_use]
pub fn identity_json() -> &'static str {
    ENGINE_IDENTITY_JSON
}
