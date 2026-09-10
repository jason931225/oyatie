//! OAuth token client. No raw token value reaches tracing output.
// data_class: INTERNAL_ONLY throughout this module.

use std::sync::Arc;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
pub use http_runtime_hyper_adapter::HyperHttpsClient;
use http_runtime_hyper_adapter::{
    build_loopback_http_or_pqc_hybrid_https_client_for_tests, build_pqc_hybrid_https_client,
};
use hyper::Request;
use tracing::{debug, warn};

use crate::token_state::{RefreshFailureKind, SeatTokenState, classify_oauth_error};

pub const ANTHROPIC_TOKEN_ENDPOINT: &str =
    intelligence_oauth_subscription_kernel::ANTHROPIC_TOKEN_ENDPOINT;

pub const ANTHROPIC_VERSION: &str = "2023-06-01";

pub const ANTHROPIC_BETA: &str = "oauth-2025-04-20";

pub const ANTHROPIC_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";

/// Only [`OAuthClientError::OAuthError`] can be terminal; every other variant
/// is worth retrying.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OAuthClientError {
    Transport(String),
    /// Non-200 whose body carried no parseable OAuth error field.
    HttpError {
        status: u16,
    },
    OAuthError {
        error: String,
        kind: RefreshFailureKind,
    },
    ParseError(String),
    /// Refused pre-flight: empty PKCE verifier, i.e. no proof of possession.
    MissingPkceVerifier,
    /// A 200 carried no `refresh_token` and none was held to carry forward.
    /// Refusing keeps the stored credential intact.
    MissingRefreshToken,
}

impl OAuthClientError {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::OAuthError {
                kind: RefreshFailureKind::Terminal(_),
                ..
            }
        )
    }
}

// data_class: INTERNAL_ONLY
#[derive(serde::Deserialize)]
struct TokenResponse {
    access_token: String,          // data_class: INTERNAL_ONLY
    refresh_token: Option<String>, // data_class: INTERNAL_ONLY
    expires_in: Option<u64>,       // seconds until expiry
}

// data_class: INTERNAL_ONLY
#[derive(serde::Deserialize)]
struct ErrorResponse {
    error: String, // data_class: INTERNAL_ONLY
}

/// TLS via the canonical PQC-hybrid policy.
pub fn build_https_client() -> HyperHttpsClient {
    build_pqc_hybrid_https_client()
}

/// Admits plaintext HTTP on loopback. Never reach for this off loopback.
#[doc(hidden)]
pub fn build_loopback_http_or_https_test_client() -> HyperHttpsClient {
    build_loopback_http_or_pqc_hybrid_https_client_for_tests()
}

pub struct OAuthTokenClient {
    http: Arc<HyperHttpsClient>,
    token_endpoint: String,
    client_id: String,
}

impl OAuthTokenClient {
    pub fn new(http: Arc<HyperHttpsClient>) -> Self {
        Self {
            http,
            token_endpoint: ANTHROPIC_TOKEN_ENDPOINT.to_owned(),
            client_id: ANTHROPIC_CLIENT_ID.to_owned(),
        }
    }

    pub fn with_token_endpoint(mut self, url: impl Into<String>) -> Self {
        self.token_endpoint = url.into();
        self
    }

    pub fn with_client_id(mut self, client_id: impl Into<String>) -> Self {
        self.client_id = client_id.into();
        self
    }

    pub async fn exchange(
        &self,
        code: &str,
        pkce_verifier: &str,
        redirect_uri: &str,
        now_secs: u64,
    ) -> Result<SeatTokenState, OAuthClientError> {
        // PKCE binds the code to the client that requested it. An empty
        // verifier is PKCE silently disabled — refuse before the code ships.
        if pkce_verifier.is_empty() {
            warn!("refusing authorization-code exchange with an empty PKCE verifier");
            return Err(OAuthClientError::MissingPkceVerifier);
        }
        let body = format!(
            "grant_type=authorization_code&code={}&code_verifier={}&redirect_uri={}&client_id={}",
            url_encode(code),
            url_encode(pkce_verifier),
            url_encode(redirect_uri),
            url_encode(&self.client_id),
        );
        debug!(endpoint = %self.token_endpoint, "exchanging authorization code for tokens");
        // Initial exchange holds no previous token to fall back on.
        self.post_token_request(body.into_bytes(), now_secs, None)
            .await
    }

    /// The provider may rotate the refresh token in its response.
    pub async fn refresh(
        &self,
        current_state: &SeatTokenState,
        now_secs: u64,
    ) -> Result<SeatTokenState, OAuthClientError> {
        let body = format!(
            "grant_type=refresh_token&refresh_token={}&client_id={}",
            url_encode(&current_state.refresh_token),
            url_encode(&self.client_id),
        );
        debug!(endpoint = %self.token_endpoint, "refreshing OAuth token");
        self.post_token_request(
            body.into_bytes(),
            now_secs,
            Some(current_state.refresh_token.as_str()),
        )
        .await
    }

    async fn post_token_request(
        &self,
        body: Vec<u8>,
        now_secs: u64,
        previous_refresh_token: Option<&str>,
    ) -> Result<SeatTokenState, OAuthClientError> {
        let req = Request::builder()
            .method("POST")
            .uri(&self.token_endpoint)
            .header("content-type", "application/x-www-form-urlencoded")
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("anthropic-beta", ANTHROPIC_BETA)
            .body(Full::new(Bytes::from(body)))
            .map_err(|e| OAuthClientError::Transport(e.to_string()))?;

        let resp = self
            .http
            .request(req)
            .await
            .map_err(|e| OAuthClientError::Transport(e.to_string()))?;

        let status = resp.status().as_u16();
        let body_bytes = resp
            .into_body()
            .collect()
            .await
            .map_err(|e| OAuthClientError::Transport(e.to_string()))?
            .to_bytes();

        if status == 200 {
            let tr: TokenResponse = serde_json::from_slice(&body_bytes)
                .map_err(|e| OAuthClientError::ParseError(e.to_string()))?;

            let expires_in = tr.expires_in.unwrap_or(3600);
            let expires_at = now_secs.saturating_add(expires_in);
            // A provider that does not rotate omits `refresh_token` from a 200.
            // Carry the previous one forward; defaulting to "" here would store
            // an empty credential and leave the next refresh with nothing to
            // present. If neither exists, refuse rather than persist a blank.
            let rotated = tr.refresh_token.filter(|t| !t.is_empty());
            let carried = previous_refresh_token
                .filter(|t| !t.is_empty())
                .map(str::to_owned);
            let Some(refresh_token) = rotated.or(carried) else {
                warn!("token response carried no refresh_token and none was held");
                return Err(OAuthClientError::MissingRefreshToken);
            };

            Ok(SeatTokenState::new(
                tr.access_token,
                refresh_token,
                expires_at,
                now_secs,
            ))
        } else {
            // A parseable error field is what separates terminal from transient.
            if let Ok(err) = serde_json::from_slice::<ErrorResponse>(&body_bytes) {
                let kind = classify_oauth_error(&err.error);
                if matches!(kind, RefreshFailureKind::Terminal(_)) {
                    warn!(error = %err.error, "terminal OAuth error from token endpoint");
                }
                Err(OAuthClientError::OAuthError {
                    error: err.error,
                    kind,
                })
            } else {
                Err(OAuthClientError::HttpError { status })
            }
        }
    }
}

/// Percent-encodes everything outside the RFC 3986 unreserved set.
fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.as_bytes() {
        let c = *b as char;
        if c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '~') {
            out.push(c);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

pub fn outbound_auth_headers(access_token: &str) -> Vec<(String, String)> {
    vec![
        ("authorization".to_owned(), format!("Bearer {access_token}")),
        ("anthropic-version".to_owned(), ANTHROPIC_VERSION.to_owned()),
        ("anthropic-beta".to_owned(), ANTHROPIC_BETA.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_encode_encodes_special_chars() {
        assert_eq!(url_encode("a b/c"), "a%20b%2Fc");
        assert_eq!(url_encode("abc-123.~_"), "abc-123.~_");
        assert_eq!(url_encode("foo=bar&baz"), "foo%3Dbar%26baz");
    }

    #[test]
    fn outbound_headers_contain_bearer_and_version() {
        let hdrs = outbound_auth_headers("test-tok");
        let map: std::collections::BTreeMap<_, _> = hdrs.into_iter().collect();
        assert_eq!(map["authorization"], "Bearer test-tok");
        assert!(map.contains_key("anthropic-version"));
        assert!(map.contains_key("anthropic-beta"));
    }

    #[test]
    fn oauth_client_error_terminal_detection() {
        let e = OAuthClientError::OAuthError {
            error: "refresh_token_expired".into(),
            kind: RefreshFailureKind::Terminal(crate::ports::AlertKind::RefreshTokenExpired),
        };
        assert!(e.is_terminal());

        let e2 = OAuthClientError::HttpError { status: 503 };
        assert!(!e2.is_terminal());
    }
}
