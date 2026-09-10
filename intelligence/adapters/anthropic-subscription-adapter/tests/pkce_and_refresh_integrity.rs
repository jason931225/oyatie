//! PKCE proof and refresh-token preservation on the OAuth credential path.
//!
//! Two defects these tests pin, both of which shipped behind a comment that
//! described the opposite behaviour:
//!
//!   1. `complete_enrollment` derived the PKCE verifier from the challenge —
//!      impossible, SHA-256 is one-way — via a stub that returned `""`. Every
//!      production enrollment exchanged its authorization code with no proof
//!      of possession.
//!   2. `post_token_request` resolved a missing `refresh_token` with
//!      `unwrap_or_default()`, storing an empty credential where the comment
//!      promised the previous one was retained.
//!
//! ADR-0083 Tier 3: test-only unwrap/expect/panic are allowed.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use intelligence_anthropic_subscription_adapter::{
    InMemoryCredentialStore, OAuthClientError, OAuthTokenClient, SeatId, SeatTokenState,
    build_enrollment_flow, build_loopback_http_or_https_test_client, complete_enrollment,
};
use intelligence_oauth_subscription_kernel::{OAuthLoopbackServer, PkceVerifier};

/// A real 43-char RFC 7636 verifier. Distinctive enough to find in a form body.
const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";

async fn bind_mock_server() -> (TcpListener, SocketAddr) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    (listener, addr)
}

/// Serve exactly one request, recording the form body the client sent.
async fn serve_one(
    listener: TcpListener,
    status: u16,
    response_json: &'static str,
    seen: Arc<tokio::sync::Mutex<String>>,
) {
    let (stream, _) = listener.accept().await.unwrap();
    let io = TokioIo::new(stream);
    http1::Builder::new()
        .serve_connection(
            io,
            service_fn(move |req: Request<hyper::body::Incoming>| {
                let seen = Arc::clone(&seen);
                async move {
                    let raw = req.into_body().collect().await.unwrap().to_bytes();
                    *seen.lock().await = String::from_utf8_lossy(&raw).to_string();
                    Ok::<_, hyper::Error>(
                        Response::builder()
                            .status(status)
                            .header("content-type", "application/json")
                            .body(Full::new(Bytes::from(response_json)))
                            .unwrap(),
                    )
                }
            }),
        )
        .await
        .unwrap();
}

// ── Defect 1: PKCE proof reaches the token endpoint ──────────────────────────

/// RED on the unfixed base: `complete_enrollment` had no `verifier` parameter,
/// so this call does not compile there; once the arity is patched in, the body
/// carries `code_verifier=` with nothing after it.
#[tokio::test]
async fn complete_enrollment_sends_the_real_pkce_verifier() {
    let (listener, addr) = bind_mock_server().await;
    let endpoint = format!("http://{addr}/oauth/token");
    let seen = Arc::new(tokio::sync::Mutex::new(String::new()));

    let body_store = Arc::clone(&seen);
    tokio::spawn(async move {
        serve_one(
            listener,
            200,
            r#"{"access_token":"a","refresh_token":"r","expires_in":3600}"#,
            body_store,
        )
        .await;
    });

    let verifier = PkceVerifier::new(VERIFIER.to_owned()).unwrap();
    let (flow, _url) = build_enrollment_flow(
        &verifier,
        "nonce-1".to_owned(),
        OAuthLoopbackServer::default_claude(),
    )
    .unwrap();

    let http = Arc::new(build_loopback_http_or_https_test_client());
    let client = OAuthTokenClient::new(http).with_token_endpoint(&endpoint);
    let store = InMemoryCredentialStore::new();

    complete_enrollment(
        &SeatId("pkce-seat".into()),
        &flow,
        &verifier,
        "code=auth-code-1&state=nonce-1",
        &client,
        &store,
        1_000_000,
    )
    .await
    .expect("enrollment must succeed");

    let body = seen.lock().await;
    let mut missing: Vec<&str> = Vec::new();
    if !body.contains(&format!("code_verifier={VERIFIER}")) {
        missing.push("code_verifier=<the generated verifier>");
    }
    if body.contains("code_verifier=&") || body.ends_with("code_verifier=") {
        missing.push("non-empty code_verifier");
    }
    assert!(
        missing.is_empty(),
        "token request missing PKCE proof {missing:?}; body was: {body}"
    );
}

/// RED on the unfixed base: this compiles there and reaches the mock server,
/// which answers 200, so `exchange` returns `Ok` instead of refusing.
#[tokio::test]
async fn exchange_refuses_an_empty_verifier_before_any_network_call() {
    let (listener, addr) = bind_mock_server().await;
    let endpoint = format!("http://{addr}/oauth/token");
    let seen = Arc::new(tokio::sync::Mutex::new(String::new()));

    let body_store = Arc::clone(&seen);
    tokio::spawn(async move {
        serve_one(
            listener,
            200,
            r#"{"access_token":"a","refresh_token":"r","expires_in":3600}"#,
            body_store,
        )
        .await;
    });

    let http = Arc::new(build_loopback_http_or_https_test_client());
    let client = OAuthTokenClient::new(http).with_token_endpoint(&endpoint);

    let result = client
        .exchange("auth-code-1", "", "http://localhost:35593/callback", 1_000)
        .await;

    assert_eq!(
        result.err(),
        Some(OAuthClientError::MissingPkceVerifier),
        "an empty verifier is PKCE disabled and must be refused",
    );
    assert!(
        seen.lock().await.is_empty(),
        "the authorization code must never leave the process without PKCE proof",
    );
}

// ── Defect 2: a non-rotating refresh must not erase the credential ───────────

/// RED on the unfixed base: `unwrap_or_default()` yields `""`, so the returned
/// (and then persisted) state carries an empty refresh token.
#[tokio::test]
async fn refresh_without_rotation_retains_the_previous_refresh_token() {
    let (listener, addr) = bind_mock_server().await;
    let endpoint = format!("http://{addr}/oauth/token");
    let seen = Arc::new(tokio::sync::Mutex::new(String::new()));

    // A provider that does not rotate omits `refresh_token` entirely.
    let body_store = Arc::clone(&seen);
    tokio::spawn(async move {
        serve_one(
            listener,
            200,
            r#"{"access_token":"rotated-access","expires_in":1800}"#,
            body_store,
        )
        .await;
    });

    let http = Arc::new(build_loopback_http_or_https_test_client());
    let client = OAuthTokenClient::new(http).with_token_endpoint(&endpoint);

    let now = 2_000_000u64;
    let current = SeatTokenState::new("old-access".into(), "keep-me".into(), now + 100, now - 3500);
    let next = client.refresh(&current, now).await.expect("refresh ok");

    assert_eq!(
        next.refresh_token, "keep-me",
        "a 200 without rotation must carry the previous refresh token forward",
    );
    assert_eq!(next.access_token, "rotated-access");
}

/// RED on the unfixed base: an empty string is stored and returned as `Ok`.
/// The Err keeps the caller's persist-before-mutate path from writing a blank.
#[tokio::test]
async fn refresh_with_no_token_anywhere_refuses_rather_than_storing_blank() {
    let (listener, addr) = bind_mock_server().await;
    let endpoint = format!("http://{addr}/oauth/token");
    let seen = Arc::new(tokio::sync::Mutex::new(String::new()));

    let body_store = Arc::clone(&seen);
    tokio::spawn(async move {
        serve_one(
            listener,
            200,
            r#"{"access_token":"a","refresh_token":"","expires_in":1800}"#,
            body_store,
        )
        .await;
    });

    let http = Arc::new(build_loopback_http_or_https_test_client());
    let client = OAuthTokenClient::new(http).with_token_endpoint(&endpoint);

    let now = 2_000_000u64;
    let current = SeatTokenState::new("old".into(), String::new(), now + 100, now);
    let result = client.refresh(&current, now).await;

    assert_eq!(
        result.err(),
        Some(OAuthClientError::MissingRefreshToken),
        "with no token in the response and none held, refusing beats persisting \"\"",
    );
}
