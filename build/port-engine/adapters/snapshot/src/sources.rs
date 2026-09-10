//! This crate's own sources, embedded so the engine-identity axis can hash them.

pub const CRATE_SOURCES: &[(&str, &str)] = &[
    ("admit.rs", include_str!("admit.rs")),
    ("admitted.rs", include_str!("admitted.rs")),
    ("error.rs", include_str!("error.rs")),
    ("lib.rs", include_str!("lib.rs")),
    ("preimage.rs", include_str!("preimage.rs")),
    ("sources.rs", include_str!("sources.rs")),
];
