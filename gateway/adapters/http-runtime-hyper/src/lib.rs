//! This is the ONLY crate in the hyper foundation that may name a
//! hyper-family dependency; every sibling stays std-only (ADR-0090,
//! ADR-0092).
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

mod admission;
mod execution;
mod response;
mod supervisor;
pub use admission::{
    InvalidServingLimits, ServingEvents, ServingLimits, ServingPhase, ServingSnapshot,
};
pub use supervisor::{ServingControl, ServingOutcome, ServingReport};

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::net::TcpListener as StdTcpListener;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use http_body_util::{BodyExt, Full, Limited};
use hyper::body::{Body, Incoming};
use hyper::{Request, Response};
use hyper_rustls::ConfigBuilderExt;
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;
use tokio::net::TcpListener;

use http_middleware_kernel::{Handler, MiddlewareChain, call_into_response};
pub use http_middleware_kernel::{HttpRequest, HttpResponse};
use http_router_kernel::{HttpMethod, Router};

/// The runtime calls the chain + handler on a tokio worker thread.
///
/// Per ADR-0094: prefer implementing `http_middleware_kernel::Handler`
/// on a typed service struct and wrap with `handler_to_sync(...)` at
/// registration. The closure alias remains for ergonomics on trivial routes.
pub type SyncHandler = Arc<dyn Fn(HttpRequest) -> HttpResponse + Send + Sync>;

mod protocol;
mod serving;
mod tls;
pub use protocol::*;
pub use serving::*;
pub use tls::*;

#[cfg(test)]
mod tests;
