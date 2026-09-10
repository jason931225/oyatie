#![forbid(unsafe_code)]

mod sources;
pub use sources::CRATE_SOURCES;

pub mod cli;
pub mod driver;
pub mod engine;
pub mod receipt_codec;
pub mod receipt_e2e;

pub const fn w0_ready() -> bool {
    driver::w0_ready()
}
