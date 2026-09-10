//! This crate's own sources, embedded so the engine-identity axis can hash them.

pub const CRATE_SOURCES: &[(&str, &str)] = &[
    ("lib.rs", include_str!("lib.rs")),
    ("sources.rs", include_str!("sources.rs")),
];
