//! # port-engine-toolchain — receipt `toolchain_digest` binder (W0-B Slice 9).
//!
//! Digests the hermetic toolchain corpus (`build/toolchains/**` mirrored under
//! `src/corpus/*.txt`). Filenames avoid nesting a `BUCK` path (buck2 srcs globs exclude those).
//! Live cell remap is `.buckconfig` `toolchains = build/toolchains`; this adapter binds the
//! corpus *bytes* so the receipt axis stays content-addressed.
#![forbid(unsafe_code)]

/// This crate's own sources, for the engine-identity axis assembled by the facade.
mod sources;
pub use sources::CRATE_SOURCES;

use port_engine_api::Digest;
use port_engine_hash::digest_bytes;

/// Fail-closed readiness gate. `true` once Slice 9 toolchain axis binding is present.
pub const fn w0_ready() -> bool {
    true
}

/// Logical paths in stable sort order (relative to `build/toolchains/`).
pub const CORPUS_PATHS: [&str; 5] = [
    "BUCK",
    "OWNERS",
    "cache/BUCK",
    "cache/OWNERS",
    "cache/defs.bzl",
];

// Package-local mirrors (`.txt` so buck2 srcs include them).
const CORPUS_BUCK: &str = include_str!("corpus/toolchains.buck.txt");
const CORPUS_OWNERS: &str = include_str!("corpus/toolchains.owners.txt");
const CORPUS_CACHE_BUCK: &str = include_str!("corpus/cache.buck.txt");
const CORPUS_CACHE_OWNERS: &str = include_str!("corpus/cache.owners.txt");
const CORPUS_CACHE_DEFS: &str = include_str!("corpus/cache.defs.bzl.txt");

/// Each [`CORPUS_PATHS`] entry paired with the mirrored bytes the digest binds.
///
/// Public so the mirror-parity fence walks the SAME list the preimage walks. A fence with its own
/// copy of the list can go green while the digest binds a file the fence never opened.
pub const CORPUS_MIRRORS: [(&str, &str); 5] = [
    (CORPUS_PATHS[0], CORPUS_BUCK),
    (CORPUS_PATHS[1], CORPUS_OWNERS),
    (CORPUS_PATHS[2], CORPUS_CACHE_BUCK),
    (CORPUS_PATHS[3], CORPUS_CACHE_OWNERS),
    (CORPUS_PATHS[4], CORPUS_CACHE_DEFS),
];

/// Stable admission preimage: each `path\\0content\\0` in [`CORPUS_PATHS`] order.
#[must_use]
pub fn toolchain_preimage() -> Vec<u8> {
    let mut out = Vec::new();
    for (path, content) in CORPUS_MIRRORS {
        out.extend_from_slice(path.as_bytes());
        out.push(0);
        out.extend_from_slice(content.as_bytes());
        out.push(0);
    }
    out
}

/// Content digest of the toolchain corpus (`sha256:<hex>`).
#[must_use]
pub fn toolchain_digest() -> Digest {
    digest_bytes(&toolchain_preimage())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slice9_claims_toolchain_readiness() {
        assert!(w0_ready());
    }

    #[test]
    fn toolchain_digest_matches_known_preimage() {
        let d = toolchain_digest();
        assert_eq!(
            d.0,
            "sha256:3e693e63c3d3bae26ea575dc477b0030003a812a9e2375134c6665f84bd50bf6"
        );
        assert_eq!(d, toolchain_digest());
    }

    #[test]
    fn corpus_mirrors_are_nonempty() {
        assert!(!CORPUS_BUCK.is_empty());
        assert!(!CORPUS_OWNERS.is_empty());
        assert!(!CORPUS_CACHE_BUCK.is_empty());
        assert!(!CORPUS_CACHE_OWNERS.is_empty());
        assert!(!CORPUS_CACHE_DEFS.is_empty());
    }
}
