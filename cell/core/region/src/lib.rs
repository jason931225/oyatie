//! Cloud Region/AZ/Cell taxonomy kernel.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

mod catalog;
mod entities;
mod model;
mod validation;

pub use cell_location::{AzCode, CellId, CellLocationError, RegionCode};
pub use model::*;

#[cfg(test)]
mod tests;
