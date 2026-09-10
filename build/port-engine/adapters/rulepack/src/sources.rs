//! This crate's own sources, embedded so the engine-identity axis can hash them.

pub const CRATE_SOURCES: &[(&str, &str)] = &[
    ("error.rs", include_str!("error.rs")),
    ("lib.rs", include_str!("lib.rs")),
    ("pack.rs", include_str!("pack.rs")),
    ("policy.rs", include_str!("policy.rs")),
    ("rule.rs", include_str!("rule.rs")),
    ("seams.rs", include_str!("seams.rs")),
    ("sources.rs", include_str!("sources.rs")),
    ("wire.rs", include_str!("wire.rs")),
];
