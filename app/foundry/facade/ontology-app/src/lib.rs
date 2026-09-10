#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod auth;
pub mod authz;
pub mod composition;
pub mod config;
pub mod dto;
pub mod metrics;
mod migrate;
pub mod observability;
pub mod observation;
pub mod pdp;
pub mod read_dto;
pub mod reads;
pub mod routes;
pub mod seed;
pub mod slo;
mod status;
pub mod submit;

pub use auth::OperatorCredential;
pub use authz::{Caller, PolicyEnforcementPoint};
pub use composition::{AppState, BootError, TenantState, compose};
pub use config::{Config, ConfigError};
pub use pdp::{PepError, Surface};
pub use routes::{router, router_from};
pub use seed::SeedError;
