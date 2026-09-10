//! The IR container and the renderers.

use std::collections::BTreeMap;

use port_engine_api::{Digest, PortError, RegionId, Renderer, TargetIr};

use crate::item::RustItem;
use crate::lower::lower_file;

/// The formatter this crate emits through, hashed into the receipt's `formatter_digest` axis.
pub const FORMATTER_ID: &str = concat!(
    "prettyplease ",
    env!("CARGO_PKG_VERSION"),
    " via port-engine-rust-ir"
);

/// An ordered set of emitted regions, each holding typed items.
///
/// Regions are declared before they are filled so the declared set and the emitted set can be
/// compared — `port-engine-kernel::emit` proves they are equal rather than trusting a renderer to
/// have produced what it said it would.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RustIr {
    regions: Vec<RegionId>,
    items: BTreeMap<RegionId, Vec<RustItem>>,
}

impl RustIr {
    /// Construct an IR declaring `region_ids` in order, with no items yet.
    #[must_use]
    pub fn new(region_ids: &[&str]) -> Self {
        Self {
            regions: region_ids
                .iter()
                .map(|id| RegionId((*id).to_owned()))
                .collect(),
            items: BTreeMap::new(),
        }
    }

    /// Attach items to an already-declared region.
    ///
    /// # Errors
    /// [`PortError::Render`] when `region` was not declared.
    pub fn set_items(&mut self, region: &str, items: Vec<RustItem>) -> Result<(), PortError> {
        let id = RegionId(region.to_owned());
        if !self.regions.contains(&id) {
            return Err(PortError::Render {
                detail: format!("region `{region}` is not declared on this RustIr"),
            });
        }
        self.items.insert(id, items);
        Ok(())
    }

    /// Borrow the items attached to `region`, if any.
    #[must_use]
    pub fn items(&self, region: &RegionId) -> Option<&[RustItem]> {
        self.items.get(region).map(Vec::as_slice)
    }

    /// Parse `region`'s items into a `syn::File`.
    ///
    /// # Errors
    /// [`PortError::Render`] when the region is unknown or its items do not assemble.
    pub fn file(&self, region: &RegionId) -> Result<syn::File, PortError> {
        let items = self.items.get(region).ok_or_else(|| PortError::Render {
            detail: format!("region `{}` carries no items", region.0),
        })?;
        lower_file(items)
    }
}

impl TargetIr for RustIr {
    fn target_language(&self) -> &str {
        "rust"
    }

    fn regions(&self) -> Vec<RegionId> {
        self.regions.clone()
    }
}

/// Deterministic empty renderer: emits zero-byte blobs for every declared region.
///
/// Kept for the wiring smoke that proves the `Renderer` seam is inhabitable without a real
/// formatter behind it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmptyRenderer {
    formatter_digest: Digest,
}

impl EmptyRenderer {
    /// Renderer with a fixed formatter digest for receipt wiring tests.
    #[must_use]
    pub fn new(formatter_digest: impl Into<String>) -> Self {
        Self {
            formatter_digest: Digest(formatter_digest.into()),
        }
    }
}

impl Renderer for EmptyRenderer {
    fn target_language(&self) -> &str {
        "rust"
    }

    fn formatter_digest(&self) -> Digest {
        self.formatter_digest.clone()
    }

    fn render(&self, ir: &dyn TargetIr) -> Result<BTreeMap<RegionId, Vec<u8>>, PortError> {
        Ok(ir.regions().into_iter().map(|r| (r, Vec::new())).collect())
    }
}

/// The real renderer: typed items in, formatted Rust source out.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RustRenderer;

impl RustRenderer {
    /// A renderer over the formatter this crate is built against.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Typed emit path. Region order follows the IR's declaration order; bytes are the formatted
    /// spelling of each region's items.
    ///
    /// # Errors
    /// [`PortError::Render`] when a region carries no items or its items do not assemble.
    pub fn render_rust_ir(&self, ir: &RustIr) -> Result<BTreeMap<RegionId, Vec<u8>>, PortError> {
        let mut out = BTreeMap::new();
        for region in ir.regions() {
            let file = ir.file(&region)?;
            out.insert(region, prettyplease::unparse(&file).into_bytes());
        }
        Ok(out)
    }
}

impl Renderer for RustRenderer {
    fn target_language(&self) -> &str {
        "rust"
    }

    fn formatter_digest(&self) -> Digest {
        // The kernel COMPARES digests and never computes one, so this reports the formatter's
        // identity and lets the hashing adapter turn it into a digest at the point a receipt is
        // built. Reporting the identity rather than a hash of it keeps this crate free of a
        // hashing dependency it would otherwise need only for this line.
        Digest(FORMATTER_ID.to_owned())
    }

    fn render(&self, _ir: &dyn TargetIr) -> Result<BTreeMap<RegionId, Vec<u8>>, PortError> {
        // Fail-closed for bare trait objects: the typed path needs a `RustIr`, and emitting empty
        // blobs through this type would claim a formatting run that never happened.
        Err(PortError::Render {
            detail: "RustRenderer::render requires typed RustIr via render_rust_ir".into(),
        })
    }
}
