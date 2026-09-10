//! This crate's own sources, embedded so the engine-identity axis can hash them.

pub const CRATE_SOURCES: &[(&str, &str)] = &[
    ("canary.rs", include_str!("canary.rs")),
    ("error.rs", include_str!("error.rs")),
    ("lib.rs", include_str!("lib.rs")),
    ("materialize.rs", include_str!("materialize.rs")),
    ("sources.rs", include_str!("sources.rs")),
];
