//! Seat enrollment via PKCE OAuth flow.
//!
//! Binding the loopback listener and reading the redirect back are the
//! caller's, so everything here stays hermetically testable.
// data_class: INTERNAL_ONLY throughout this module.

use intelligence_account_domain::ProviderFamily;
use intelligence_oauth_subscription_kernel::{
    FlowKind, OAuthLoopbackServer, PkceVerifier, SubscriptionOAuthFlow,
    SubscriptionTokenCaptureRequest, build_authorization_url, capture_subscription_token,
};

use crate::oauth_client::OAuthTokenClient;
use crate::ports::{CredentialStorePort, SeatId, TokenBytes};
use crate::token_state::SeatTokenState;

#[derive(Debug)]
pub enum EnrollmentError {
    PkceError(intelligence_oauth_subscription_kernel::OAuthError),
    OAuthClientError(crate::oauth_client::OAuthClientError),
    CredentialStoreError(String),
    CallbackParseError(String),
    StateMismatch { expected: String, got: String },
}

impl std::fmt::Display for EnrollmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PkceError(e) => write!(f, "PKCE error: {e}"),
            Self::OAuthClientError(e) => write!(f, "OAuth client error: {e:?}"),
            Self::CredentialStoreError(e) => write!(f, "credential store error: {e}"),
            Self::CallbackParseError(e) => write!(f, "callback parse error: {e}"),
            Self::StateMismatch { expected, got } => {
                write!(f, "state mismatch: expected {expected}, got {got}")
            }
        }
    }
}

// data_class: INTERNAL_ONLY
#[derive(Debug, Clone)]
pub struct CallbackParams {
    pub code: String,  // data_class: INTERNAL_ONLY
    pub state: String, // data_class: INTERNAL_ONLY
}

/// Accepts either a full redirect URL or a bare query string.
pub fn parse_callback(url_or_query: &str) -> Result<CallbackParams, EnrollmentError> {
    let query = if let Some(pos) = url_or_query.find('?') {
        &url_or_query[pos + 1..]
    } else {
        url_or_query
    };

    let mut code = None;
    let mut state = None;

    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            match k {
                "code" => code = Some(percent_decode(v)),
                "state" => state = Some(percent_decode(v)),
                _ => {}
            }
        }
    }

    let code = code.ok_or_else(|| EnrollmentError::CallbackParseError("missing code".into()))?;
    let state = state.ok_or_else(|| EnrollmentError::CallbackParseError("missing state".into()))?;

    if code.is_empty() {
        return Err(EnrollmentError::CallbackParseError("empty code".into()));
    }
    Ok(CallbackParams { code, state })
}

/// Build the enrollment flow from a pre-generated PKCE verifier and state nonce.
/// Returns the `(flow, authorization_url)` pair for the caller to present to the browser.
///
/// Borrows the verifier rather than consuming it: the flow carries only the
/// challenge, and `complete_enrollment` needs the verifier again in step 2.
pub fn build_enrollment_flow(
    verifier: &PkceVerifier,
    state_nonce: String,
    loopback: OAuthLoopbackServer,
) -> Result<(SubscriptionOAuthFlow, String), EnrollmentError> {
    let req = SubscriptionTokenCaptureRequest {
        flow_kind: FlowKind::AnthropicSubscriptionOAuth,
        provider: ProviderFamily::Claude,
        verifier: verifier.clone(),
        state_nonce,
        loopback,
    };
    let flow = capture_subscription_token(&req).map_err(EnrollmentError::PkceError)?;
    let url = build_authorization_url(&flow).map_err(EnrollmentError::PkceError)?;
    Ok((flow, url))
}

/// Complete enrollment: exchange the authorization code for tokens, persist them.
/// Returns the `SeatTokenState` on success.
///
/// `verifier` MUST be the same `PkceVerifier` whose challenge is carried by
/// `flow`. It is not derivable from the challenge — SHA-256 is one-way — so
/// the caller that generated it in step 1 has to hand it back here.
pub async fn complete_enrollment(
    seat_id: &SeatId,
    flow: &SubscriptionOAuthFlow,
    verifier: &PkceVerifier,
    callback_url_or_query: &str,
    client: &OAuthTokenClient,
    store: &dyn CredentialStorePort,
    now_secs: u64,
) -> Result<SeatTokenState, EnrollmentError> {
    let params = parse_callback(callback_url_or_query)?;

    // Verify state nonce to prevent CSRF.
    if params.state != flow.state_nonce {
        return Err(EnrollmentError::StateMismatch {
            expected: flow.state_nonce.clone(),
            got: params.state,
        });
    }

    let state = client
        .exchange(
            &params.code,
            verifier.expose_secret(),
            &flow.loopback.redirect_uri(),
            now_secs,
        )
        .await
        .map_err(EnrollmentError::OAuthClientError)?;

    // Persist BEFORE returning (persist-before-mutate invariant).
    let bytes = state
        .to_storage_bytes()
        .map_err(EnrollmentError::CredentialStoreError)?;
    store
        .store(seat_id, TokenBytes(bytes))
        .map_err(EnrollmentError::CredentialStoreError)?;

    Ok(state)
}

/// Minimal percent-decode for callback query values.
fn percent_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(h) = std::str::from_utf8(&bytes[i + 1..i + 3])
                && let Ok(b) = u8::from_str_radix(h, 16)
            {
                out.push(b as char);
                i += 3;
                continue;
            }
            out.push(bytes[i] as char);
            i += 1;
        } else if bytes[i] == b'+' {
            out.push(' ');
            i += 1;
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_callback_full_url() {
        let url = "http://localhost:35593/callback?code=AUTH_CODE&state=nonce42";
        let p = parse_callback(url).unwrap();
        assert_eq!(p.code, "AUTH_CODE");
        assert_eq!(p.state, "nonce42");
    }

    #[test]
    fn parse_callback_query_only() {
        let p = parse_callback("code=XYZ&state=ABC").unwrap();
        assert_eq!(p.code, "XYZ");
        assert_eq!(p.state, "ABC");
    }

    #[test]
    fn parse_callback_missing_code_errors() {
        assert!(parse_callback("state=s").is_err());
    }

    #[test]
    fn parse_callback_missing_state_errors() {
        assert!(parse_callback("code=c").is_err());
    }

    #[test]
    fn parse_callback_empty_code_errors() {
        assert!(parse_callback("code=&state=s").is_err());
    }

    #[test]
    fn percent_decode_basic() {
        assert_eq!(percent_decode("hello%20world"), "hello world");
        assert_eq!(percent_decode("a+b"), "a b");
        assert_eq!(percent_decode("abc"), "abc");
    }

    #[test]
    fn build_enrollment_flow_produces_valid_url() {
        let verifier =
            PkceVerifier::new("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk".to_owned()).unwrap();
        let loopback = OAuthLoopbackServer::default_claude();
        let (flow, url) = build_enrollment_flow(&verifier, "nonce42".into(), loopback).unwrap();
        assert!(!url.is_empty());
        assert!(url.contains("code_challenge"));
        assert!(url.contains("nonce42"));
        assert_eq!(flow.state_nonce, "nonce42");
    }

    #[test]
    fn state_mismatch_detected() {
        let verifier =
            PkceVerifier::new("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk".to_owned()).unwrap();
        let loopback = OAuthLoopbackServer::default_claude();
        let (flow, _) = build_enrollment_flow(&verifier, "correct-nonce".into(), loopback).unwrap();
        // Simulate wrong state in callback.
        let params = parse_callback("code=c&state=wrong-nonce").unwrap();
        assert_ne!(params.state, flow.state_nonce);
    }
}
