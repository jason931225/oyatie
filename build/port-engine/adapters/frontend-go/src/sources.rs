//! This crate's own sources, embedded so the engine-identity axis can hash them.

pub const CRATE_SOURCES: &[(&str, &str)] = &[
    ("convert.rs", include_str!("convert.rs")),
    ("error.rs", include_str!("error.rs")),
    ("lib.rs", include_str!("lib.rs")),
    ("model.rs", include_str!("model.rs")),
    ("sources.rs", include_str!("sources.rs")),
    ("vocabulary.rs", include_str!("vocabulary.rs")),
    ("wire.rs", include_str!("wire.rs")),
];
