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

/// Each `build/toolchains/` path paired with the package-local mirror the digest binds, in stable
/// sort order. Mirrors are `.txt` so buck2 srcs globs include them.
///
/// The path and its bytes are ONE entry so no second list of paths can be bound, walked or fenced
/// independently of the bytes: an entry that names a file necessarily contributes that file's
/// bytes, and a file the digest never opened cannot appear here at all.
pub const CORPUS_MIRRORS: [(&str, &str); 6] = [
    ("BUCK", include_str!("corpus/toolchains.buck.txt")),
    ("OWNERS", include_str!("corpus/toolchains.owners.txt")),
    ("cache/BUCK", include_str!("corpus/cache.buck.txt")),
    ("cache/OWNERS", include_str!("corpus/cache.owners.txt")),
    ("cache/defs.bzl", include_str!("corpus/cache.defs.bzl.txt")),
    ("rust.bzl", include_str!("corpus/toolchains.rust.bzl.txt")),
];

/// Stable admission preimage: each `path\\0content\\0` in [`CORPUS_MIRRORS`] order.
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
            "sha256:b27b5e3f75b54b19ee0b59afad80abd578dca79d890b387f7e82d5a950825f54"
        );
        assert_eq!(d, toolchain_digest());
    }

    #[test]
    fn corpus_mirrors_are_nonempty() {
        for (path, content) in CORPUS_MIRRORS {
            assert!(!content.is_empty(), "{path} mirror is empty");
        }
    }
}
