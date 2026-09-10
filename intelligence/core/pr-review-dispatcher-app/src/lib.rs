//! Reviewer-panel fan-out for the post-CI-green review gate: per-facet
//! findings in, one PR verdict out.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod fanout;
pub mod rollup;
