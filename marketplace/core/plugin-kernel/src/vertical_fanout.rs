//! Vertical fan-out: a closed [`FanoutVertical`] enum and the
//! [`FanoutTarget`] pairing each vertical with its Cosign-signed
//! regional-pack identifier.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

const FANOUT_PACK_ID_PREFIX: &str = "vpack_";
const FANOUT_COSIGN_PREFIX: &str = "cosign:";
pub const FANOUT_VERTICAL_COUNT: usize = 13;
pub const FANOUT_SCHEMA_VERSION: u32 = 1;

/// Error variants for [`FanoutTarget`] construction.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum FanoutError {
    InvalidPackId,
    InvalidCosignSignature,
    DuplicateVertical,
}

/// Korea is the already-proven vertical and is intentionally absent:
/// this enum is the complement set.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FanoutVertical {
    Healthcare,
    Industrial,
    Logistics,
    Fintech,
    Legal,
    Retail,
    Education,
    PublicSector,
    Hospitality,
    Construction,
    RealEstate,
    Agriculture,
    Food,
}

impl FanoutVertical {
    /// Canonical kebab-case name used in regional-pack identifiers.
    pub fn pack_slug(self) -> &'static str {
        match self {
            Self::Healthcare => "healthcare",
            Self::Industrial => "industrial",
            Self::Logistics => "logistics",
            Self::Fintech => "fintech",
            Self::Legal => "legal",
            Self::Retail => "retail",
            Self::Education => "education",
            Self::PublicSector => "public-sector",
            Self::Hospitality => "hospitality",
            Self::Construction => "construction",
            Self::RealEstate => "real-estate",
            Self::Agriculture => "agriculture",
            Self::Food => "food",
        }
    }

    /// Ordered array of all 13 fan-out verticals.
    pub fn all() -> [Self; FANOUT_VERTICAL_COUNT] {
        [
            Self::Healthcare,
            Self::Industrial,
            Self::Logistics,
            Self::Fintech,
            Self::Legal,
            Self::Retail,
            Self::Education,
            Self::PublicSector,
            Self::Hospitality,
            Self::Construction,
            Self::RealEstate,
            Self::Agriculture,
            Self::Food,
        ]
    }
}

/// Capability-pack deployment target: vertical + Cosign-signed pack id.
///
/// `pack_id` must carry the `vpack_` prefix and `cosign_signature`
/// must carry the `cosign:` prefix (ADR-0039).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FanoutTarget {
    pub vertical: FanoutVertical, // data_class: INTERNAL_ONLY
    pub pack_id: String,          // data_class: INTERNAL_ONLY
    pub cosign_signature: String, // data_class: INTERNAL_ONLY
    pub schema_version: u32,      // data_class: INTERNAL_ONLY
}

impl FanoutTarget {
    /// Construct and validate a fan-out target.
    ///
    /// In addition to the `vpack_` prefix check, `pack_id` must match the
    /// canonical pack id for `vertical` (`vpack_<vertical.pack_slug()>`),
    /// preventing mismatched pairs like `Healthcare` + `vpack_fintech`.
    pub fn new(
        vertical: FanoutVertical,
        pack_id: impl Into<String>,
        cosign_signature: impl Into<String>,
    ) -> Result<Self, FanoutError> {
        let pack_id = pack_id.into();
        if !pack_id.starts_with(FANOUT_PACK_ID_PREFIX)
            || pack_id.len() <= FANOUT_PACK_ID_PREFIX.len()
        {
            return Err(FanoutError::InvalidPackId);
        }
        if pack_id != Self::canonical_pack_id(vertical) {
            return Err(FanoutError::InvalidPackId);
        }
        let cosign_signature = cosign_signature.into();
        if !cosign_signature.starts_with(FANOUT_COSIGN_PREFIX)
            || cosign_signature.len() <= FANOUT_COSIGN_PREFIX.len()
        {
            return Err(FanoutError::InvalidCosignSignature);
        }
        Ok(Self {
            vertical,
            pack_id,
            cosign_signature,
            schema_version: FANOUT_SCHEMA_VERSION,
        })
    }

    /// Expected pack_id for this vertical under the canonical naming scheme.
    ///
    /// e.g. `FanoutVertical::Healthcare` → `"vpack_healthcare"`.
    pub fn canonical_pack_id(vertical: FanoutVertical) -> String {
        format!("{}{}", FANOUT_PACK_ID_PREFIX, vertical.pack_slug())
    }
}

/// Roster of all 13 fan-out targets (one per vertical).
///
/// Duplicate verticals are rejected to enforce the one-pack-per-vertical
/// invariant.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FanoutRoster {
    targets: Vec<FanoutTarget>,
}

impl FanoutRoster {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a fan-out target.
    ///
    /// Re-validates that `target.pack_id` matches the canonical id for
    /// `target.vertical`, catching targets whose invariants were bypassed by
    /// direct field construction rather than [`FanoutTarget::new`].
    /// Returns [`FanoutError::InvalidPackId`] on mismatch or
    /// [`FanoutError::DuplicateVertical`] if the vertical is already present.
    pub fn register(&mut self, target: FanoutTarget) -> Result<(), FanoutError> {
        // Re-validate pack_id: catches direct field construction bypassing new().
        if target.pack_id != FanoutTarget::canonical_pack_id(target.vertical) {
            return Err(FanoutError::InvalidPackId);
        }
        // Re-validate cosign prefix: catches targets whose cosign_signature was
        // set directly (e.g. empty string) without going through FanoutTarget::new.
        if !target.cosign_signature.starts_with(FANOUT_COSIGN_PREFIX)
            || target.cosign_signature.len() <= FANOUT_COSIGN_PREFIX.len()
        {
            return Err(FanoutError::InvalidCosignSignature);
        }
        // Re-validate schema_version is the canonical value.
        if target.schema_version != FANOUT_SCHEMA_VERSION {
            return Err(FanoutError::InvalidPackId);
        }
        if self.targets.iter().any(|t| t.vertical == target.vertical) {
            return Err(FanoutError::DuplicateVertical);
        }
        self.targets.push(target);
        Ok(())
    }

    pub fn get(&self, vertical: FanoutVertical) -> Option<&FanoutTarget> {
        self.targets.iter().find(|t| t.vertical == vertical)
    }

    pub fn len(&self) -> usize {
        self.targets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }

    /// Returns true when all 13 verticals are registered (preview-complete).
    pub fn is_preview_complete(&self) -> bool {
        self.targets.len() == FANOUT_VERTICAL_COUNT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(v: FanoutVertical) -> FanoutTarget {
        FanoutTarget::new(
            v,
            FanoutTarget::canonical_pack_id(v),
            format!("cosign:sha256:{}", v.pack_slug()),
        )
        .expect("valid target")
    }

    #[test]
    fn fanout_vertical_all_returns_13_variants() {
        assert_eq!(FanoutVertical::all().len(), FANOUT_VERTICAL_COUNT);
        // Spot-check ordering stability
        assert_eq!(FanoutVertical::all()[0], FanoutVertical::Healthcare);
        assert_eq!(FanoutVertical::all()[12], FanoutVertical::Food);
    }

    #[test]
    fn pack_slug_has_no_uppercase_and_matches_spec_names() {
        for v in FanoutVertical::all() {
            let slug = v.pack_slug();
            assert!(
                slug.bytes().all(|b| b.is_ascii_lowercase() || b == b'-'),
                "slug '{slug}' contains non-kebab-case chars"
            );
        }
        assert_eq!(FanoutVertical::PublicSector.pack_slug(), "public-sector");
        assert_eq!(FanoutVertical::RealEstate.pack_slug(), "real-estate");
    }

    #[test]
    fn fanout_target_rejects_bad_pack_id_and_cosign() {
        let bad_pack = FanoutTarget::new(
            FanoutVertical::Healthcare,
            "healthcare",
            "cosign:sha256:abc",
        )
        .expect_err("pack_id must carry vpack_ prefix");
        assert_eq!(bad_pack, FanoutError::InvalidPackId);

        let bad_cosign = FanoutTarget::new(
            FanoutVertical::Healthcare,
            "vpack_healthcare",
            "openssl:abc",
        )
        .expect_err("cosign prefix required");
        assert_eq!(bad_cosign, FanoutError::InvalidCosignSignature);
    }

    #[test]
    fn fanout_target_canonical_pack_id_matches_slug() {
        assert_eq!(
            FanoutTarget::canonical_pack_id(FanoutVertical::Healthcare),
            "vpack_healthcare"
        );
        assert_eq!(
            FanoutTarget::canonical_pack_id(FanoutVertical::PublicSector),
            "vpack_public-sector"
        );
    }

    #[test]
    fn roster_rejects_duplicate_vertical() {
        let mut roster = FanoutRoster::new();
        roster.register(target(FanoutVertical::Healthcare)).unwrap();
        let dup = roster
            .register(target(FanoutVertical::Healthcare))
            .expect_err("duplicate vertical rejected");
        assert_eq!(dup, FanoutError::DuplicateVertical);
    }

    #[test]
    fn roster_preview_complete_when_all_13_registered() {
        let mut roster = FanoutRoster::new();
        assert!(!roster.is_preview_complete());
        for v in FanoutVertical::all() {
            roster.register(target(v)).unwrap();
        }
        assert!(roster.is_preview_complete());
        assert_eq!(roster.len(), FANOUT_VERTICAL_COUNT);
        assert!(roster.get(FanoutVertical::Food).is_some());
    }

    #[test]
    fn roster_get_returns_none_for_unregistered_vertical() {
        let roster = FanoutRoster::new();
        assert!(roster.get(FanoutVertical::Logistics).is_none());
    }
}
