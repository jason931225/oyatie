//! What the engine IS, for the receipt's `engine_digest` axis.
//!
//! Each crate embeds only what it OWNS, as `CRATE_SOURCES`. This joins them; `port-engine-identity`
//! decides what hashing the join means.

use port_engine_identity::CrateSources;

/// Every crate the engine is built from, with the sources each one owns.
///
/// The order is part of the preimage, so a reordering would move the digest without the engine
/// changing — a false `Explained`, the mirror of the false `Unchanged` this axis exists to stop.
/// Sorted by crate name, held there by `engine_crates_are_sorted_by_crate_name`.
#[must_use]
pub fn engine_crates() -> Vec<CrateSources<'static>> {
    vec![
        ("port-engine-analysis", port_engine_analysis::CRATE_SOURCES),
        ("port-engine-api", port_engine_api::CRATE_SOURCES),
        ("port-engine-app", crate::CRATE_SOURCES),
        ("port-engine-emit", port_engine_emit::CRATE_SOURCES),
        (
            "port-engine-frontend-go",
            port_engine_frontend_go::CRATE_SOURCES,
        ),
        ("port-engine-hash", port_engine_hash::CRATE_SOURCES),
        ("port-engine-identity", port_engine_identity::CRATE_SOURCES),
        ("port-engine-kernel", port_engine_kernel::CRATE_SOURCES),
        ("port-engine-rulepack", port_engine_rulepack::CRATE_SOURCES),
        ("port-engine-rust-ir", port_engine_rust_ir::CRATE_SOURCES),
        ("port-engine-snapshot", port_engine_snapshot::CRATE_SOURCES),
        (
            "port-engine-source-pin",
            port_engine_source_pin::CRATE_SOURCES,
        ),
        (
            "port-engine-toolchain",
            port_engine_toolchain::CRATE_SOURCES,
        ),
        (
            "port-engine-transform",
            port_engine_transform::CRATE_SOURCES,
        ),
    ]
}

/// The engine's content digest — the receipt's `engine_digest` axis.
#[must_use]
pub fn engine_digest() -> port_engine_api::Digest {
    port_engine_identity::engine_digest(&engine_crates())
}
