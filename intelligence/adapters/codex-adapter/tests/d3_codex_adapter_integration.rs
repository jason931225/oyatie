//! D3 integration tests — CodexAdapter against a scripted HTTP server.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use std::collections::BTreeMap;
use std::sync::Arc;

use intelligence_codex_adapter::{CodexAdapter, CodexAdapterError, CodexProxyRequest, HOP_BY_HOP};
use scripted_http_server::{Chunk, ScriptedResponse, ScriptedServer};

fn make_client() -> Arc<reqwest::Client> {
    Arc::new(
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap(),
    )
}

fn empty_request() -> CodexProxyRequest {
    CodexProxyRequest {
        body: br#"{"model":"codex","messages":[]}"#.to_vec(),
        extra_headers: BTreeMap::new(),
    }
}

#[tokio::test]
async fn token_refresh_post_shape_and_response_parsed() {
    let server = ScriptedServer::start(vec![
        ScriptedResponse::ok()
            .header("content-type", "application/json")
            .body(r#"{"accessToken":"new-access-tok","user":{"email":"test@example.com"}}"#),
    ]);

    let adapter = CodexAdapter::with_base_url(make_client(), server.base_url().to_owned());
    let result = adapter.refresh_token("my-refresh-tok").await;

    assert!(result.is_ok(), "expected Ok, got: {result:?}");
    let tokens = result.unwrap();
    assert_eq!(tokens.access_token, "new-access-tok");

    let requests = server.requests();
    assert_eq!(server.request_lines(), vec!["POST /api/auth/session"]);
    assert_eq!(
        requests[0].header("cookie"),
        Some("__Secure-next-auth.session-token=my-refresh-tok"),
        "the refresh token must be carried in the session cookie"
    );
    assert!(
        requests[0].has_header("user-agent"),
        "the session refresh must identify itself with a User-Agent"
    );
}

#[tokio::test]
async fn proxy_sets_bearer_and_cli_version_user_agent() {
    let server = ScriptedServer::start(vec![
        ScriptedResponse::ok()
            .header("content-type", "application/json")
            .body(r#"{"id":"resp-ok","object":"codex.response"}"#),
    ]);

    let adapter = CodexAdapter::with_base_url(make_client(), server.base_url().to_owned());
    let result = adapter.proxy("test-access-token", empty_request()).await;

    assert!(result.is_ok(), "expected Ok, got: {result:?}");
    let resp = result.unwrap();
    assert_eq!(resp.status, 200);

    let requests = server.requests();
    assert_eq!(
        server.request_lines(),
        vec!["POST /backend-api/codex/responses"]
    );
    assert_eq!(
        requests[0].header("authorization"),
        Some("Bearer test-access-token")
    );
    assert_eq!(requests[0].header("user-agent"), Some("cli/0.27.0"));
    assert_eq!(requests[0].header("x-openai-beta"), Some("codex-runs"));
    assert_eq!(
        requests[0].body,
        br#"{"model":"codex","messages":[]}"#.to_vec()
    );
}

#[tokio::test]
async fn proxy_429_with_retry_after_returns_rate_limited_error() {
    let server = ScriptedServer::start(vec![
        ScriptedResponse::status(429)
            .header("retry-after", "60")
            .header("content-type", "application/json")
            .body(r#"{"error":"rate_limit_exceeded"}"#),
    ]);

    let adapter = CodexAdapter::with_base_url(make_client(), server.base_url().to_owned());
    let result = adapter.proxy("some-token", empty_request()).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        CodexAdapterError::RateLimited { retry_after_secs } => {
            assert_eq!(
                retry_after_secs,
                Some(60),
                "expected Retry-After 60 seconds"
            );
        }
        other => panic!("expected RateLimited, got: {other:?}"),
    }
    assert_eq!(
        server.request_lines(),
        vec!["POST /backend-api/codex/responses"]
    );
}

#[tokio::test]
async fn refresh_429_with_retry_after_returns_rate_limited_error() {
    let server = ScriptedServer::start(vec![
        ScriptedResponse::status(429)
            .header("retry-after", "30")
            .text("rate limited"),
    ]);

    let adapter = CodexAdapter::with_base_url(make_client(), server.base_url().to_owned());
    let result = adapter.refresh_token("some-refresh-tok").await;

    assert!(result.is_err());
    match result.unwrap_err() {
        CodexAdapterError::RateLimited { retry_after_secs } => {
            assert_eq!(retry_after_secs, Some(30));
        }
        other => panic!("expected RateLimited, got: {other:?}"),
    }
    assert_eq!(server.request_lines(), vec!["POST /api/auth/session"]);
}

#[tokio::test]
async fn refresh_401_invalid_grant_returns_refresh_failed() {
    let server = ScriptedServer::start(vec![
        ScriptedResponse::status(401)
            .header("content-type", "application/json")
            .body(r#"{"error":"invalid_grant","error_description":"refresh token revoked"}"#),
    ]);

    let adapter = CodexAdapter::with_base_url(make_client(), server.base_url().to_owned());
    let result = adapter.refresh_token("revoked-rt").await;

    assert!(result.is_err());
    match result.unwrap_err() {
        CodexAdapterError::RefreshFailed(msg) => {
            assert!(
                msg.contains("401") || msg.contains("session refresh failed"),
                "error message should describe refresh failure: {msg}"
            );
        }
        other => panic!("expected RefreshFailed, got: {other:?}"),
    }
    assert_eq!(server.request_lines(), vec!["POST /api/auth/session"]);
}

#[tokio::test]
async fn proxy_stream_200_returns_upstream_bytes() {
    use futures_util::StreamExt;

    let delta_event = "data: {\"delta\":\"hello\"}\n\n";
    let done_event = "data: [DONE]\n\n";
    let sse_body = format!("{delta_event}{done_event}");
    let server = ScriptedServer::start(vec![ScriptedResponse::ok().sse(vec![
        Chunk::new(delta_event),
        Chunk::after(std::time::Duration::from_millis(30), done_event),
    ])]);

    let adapter = CodexAdapter::with_base_url(make_client(), server.base_url().to_owned());
    let stream_result = adapter.proxy_stream("stream-token", empty_request()).await;

    assert!(stream_result.is_ok(), "expected Ok from proxy_stream");
    let (status, _headers, mut stream) = stream_result.unwrap();
    assert_eq!(status, 200);

    let mut collected = Vec::new();
    let mut frames = 0usize;
    while let Some(chunk) = stream.next().await {
        let bytes = chunk.expect("chunk read error");
        collected.extend_from_slice(&bytes);
        frames += 1;
    }
    assert_eq!(
        String::from_utf8_lossy(&collected),
        sse_body,
        "streamed bytes must match upstream body"
    );
    assert!(
        frames >= 2,
        "the two upstream SSE frames were coalesced into {frames} chunk(s); streaming \
         pass-through must forward each frame as it arrives"
    );
    assert_eq!(
        server.request_lines(),
        vec!["POST /backend-api/codex/responses"]
    );
}

#[tokio::test]
async fn response_hop_by_hop_headers_stripped() {
    let server = ScriptedServer::start(vec![
        ScriptedResponse::ok()
            .header("content-type", "application/json")
            .header("connection", "keep-alive")
            .header("keep-alive", "timeout=60")
            .header("x-custom-response", "present")
            .chunks(vec![Chunk::new(r#"{"id":"resp-hop"}"#)]),
    ]);

    let adapter = CodexAdapter::with_base_url(make_client(), server.base_url().to_owned());
    let result = adapter.proxy("hop-token", empty_request()).await;

    assert!(result.is_ok());
    let resp = result.unwrap();

    for h in HOP_BY_HOP {
        assert!(
            !resp.headers.contains_key(*h),
            "hop-by-hop header '{h}' should have been stripped from response"
        );
    }

    assert!(
        resp.headers.contains_key("content-type"),
        "content-type should pass through the response filter"
    );
    assert!(
        resp.headers.contains_key("x-custom-response"),
        "custom non-hop header should pass through"
    );
    assert_eq!(resp.body, br#"{"id":"resp-hop"}"#.to_vec());
}

#[tokio::test]
async fn request_hop_by_hop_headers_not_forwarded_upstream() {
    let server = ScriptedServer::start(vec![
        ScriptedResponse::ok()
            .header("content-type", "application/json")
            .body(r#"{"id":"resp-hop-req"}"#),
    ]);

    let adapter = CodexAdapter::with_base_url(make_client(), server.base_url().to_owned());

    let mut headers = BTreeMap::new();
    headers.insert("x-safe-header".to_string(), "keep-me".to_string());
    for h in HOP_BY_HOP {
        headers.insert(h.to_string(), "drop-me".to_string());
    }

    let req = CodexProxyRequest {
        body: b"{}".to_vec(),
        extra_headers: headers,
    };

    let result = adapter.proxy("hop-req-token", req).await;
    assert!(result.is_ok(), "proxy should succeed: {result:?}");

    let requests = server.requests();
    assert_eq!(
        server.request_lines(),
        vec!["POST /backend-api/codex/responses"]
    );
    // `connection` is excluded: reqwest's own HTTP/1.1 client sets it, so only the inbound
    // sentinel VALUE is asserted against instead.
    for header in HOP_BY_HOP {
        if *header == "connection" {
            continue;
        }
        assert!(
            !requests[0].has_header(header),
            "hop-by-hop header '{header}' was forwarded upstream — must be stripped \
             (upstream saw: {:?})",
            requests[0].headers
        );
    }
    assert!(
        !requests[0]
            .header_values("connection")
            .iter()
            .any(|value| value.contains("drop-me")),
        "the inbound Connection value leaked upstream: {:?}",
        requests[0].header_values("connection")
    );
    assert_eq!(requests[0].header("x-safe-header"), Some("keep-me"));
}
