//! OAuth subscription-token capture as pure value objects. The loopback
//! server, the browser and the upstream exchange all live in a runtime
//! adapter, so no raw token is ever in this crate's scope (ADR-0043).
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use intelligence_account_kernel::{ProviderFamily, SecretReference};

mod crypto;
mod flow;
mod types;

use crypto::{base64url_no_pad, sha256, url_encode};
pub use flow::{
    ANTHROPIC_AUTHORIZATION_ENDPOINT, ANTHROPIC_TOKEN_ENDPOINT, anthropic_subscription_scopes,
    build_authorization_url, capture_subscription_token, record_capture,
};
pub use types::{
    FlowKind, OAuthError, OAuthLoopbackServer, PkceChallenge, PkceVerifier, SubscriptionOAuthFlow,
    SubscriptionTokenCaptureRequest, SubscriptionTokenCaptureResponse,
};

#[cfg(test)]
mod tests {
    use super::*;

    fn good_verifier() -> PkceVerifier {
        // RFC 7636 §4.1 sets 43 characters as the minimum verifier length.
        PkceVerifier::new("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk".to_owned()).unwrap()
    }

    #[test]
    fn pkce_verifier_length_enforced() {
        let too_short = "a".repeat(10);
        assert_eq!(
            PkceVerifier::new(too_short),
            Err(OAuthError::InvalidVerifierLength)
        );
        let too_long = "a".repeat(200);
        assert_eq!(
            PkceVerifier::new(too_long),
            Err(OAuthError::InvalidVerifierLength)
        );
    }

    #[test]
    fn pkce_verifier_charset_enforced() {
        let bad = "!".repeat(50);
        assert_eq!(
            PkceVerifier::new(bad),
            Err(OAuthError::InvalidVerifierCharset)
        );
    }

    #[test]
    fn pkce_verifier_debug_is_redacted() {
        let v = good_verifier();
        let dbg = format!("{v:?}");
        assert!(dbg.contains("[REDACTED]"));
        assert!(!dbg.contains("dBjftJeZ"));
    }

    #[test]
    fn pkce_challenge_matches_rfc7636_appendix_b() {
        // RFC 7636 Appendix B test vector:
        //   verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"
        //   challenge (S256) = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        let v = good_verifier();
        let c = PkceChallenge::derive_s256(&v);
        assert_eq!(c.as_str(), "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn loopback_default_matches_ccproxy_port() {
        let l = OAuthLoopbackServer::default_claude();
        assert_eq!(l.port, 35593);
        assert_eq!(l.callback_path, "/callback");
        assert_eq!(l.redirect_uri(), "http://localhost:35593/callback");
    }

    #[test]
    fn anthropic_scopes_match_ccproxy() {
        let s = anthropic_subscription_scopes();
        assert!(s.contains(&"org:create_api_key".to_owned()));
        assert!(s.contains(&"user:profile".to_owned()));
        assert!(s.contains(&"user:inference".to_owned()));
    }

    #[test]
    fn capture_request_builds_anthropic_flow() {
        let req = SubscriptionTokenCaptureRequest {
            flow_kind: FlowKind::AnthropicSubscriptionOAuth,
            provider: ProviderFamily::Claude,
            verifier: good_verifier(),
            state_nonce: "abc123".into(),
            loopback: OAuthLoopbackServer::default_claude(),
        };
        let flow = capture_subscription_token(&req).unwrap();
        assert_eq!(
            flow.authorization_endpoint,
            ANTHROPIC_AUTHORIZATION_ENDPOINT
        );
        assert_eq!(flow.token_endpoint, ANTHROPIC_TOKEN_ENDPOINT);
        assert_eq!(flow.scopes, anthropic_subscription_scopes());
    }

    #[test]
    fn capture_request_rejects_empty_state_nonce() {
        let req = SubscriptionTokenCaptureRequest {
            flow_kind: FlowKind::AnthropicSubscriptionOAuth,
            provider: ProviderFamily::Claude,
            verifier: good_verifier(),
            state_nonce: String::new(),
            loopback: OAuthLoopbackServer::default_claude(),
        };
        assert_eq!(
            capture_subscription_token(&req),
            Err(OAuthError::EmptyStateNonce)
        );
    }

    #[test]
    fn capture_request_rejects_privileged_port() {
        let req = SubscriptionTokenCaptureRequest {
            flow_kind: FlowKind::AnthropicSubscriptionOAuth,
            provider: ProviderFamily::Claude,
            verifier: good_verifier(),
            state_nonce: "abc".into(),
            loopback: OAuthLoopbackServer {
                port: 80,
                callback_path: "/callback".into(),
            },
        };
        assert_eq!(
            capture_subscription_token(&req),
            Err(OAuthError::LoopbackPortReserved)
        );
    }

    #[test]
    fn api_key_import_flow_degenerates_to_empty_url() {
        let req = SubscriptionTokenCaptureRequest {
            flow_kind: FlowKind::ApiKeyImport,
            provider: ProviderFamily::Claude,
            verifier: good_verifier(),
            state_nonce: "abc".into(),
            loopback: OAuthLoopbackServer::default_claude(),
        };
        let flow = capture_subscription_token(&req).unwrap();
        assert_eq!(flow.flow_kind, FlowKind::ApiKeyImport);
        assert!(flow.authorization_endpoint.is_empty());
        assert_eq!(build_authorization_url(&flow).unwrap(), "");
    }

    #[test]
    fn build_url_contains_pkce_challenge_state_redirect_scope() {
        let req = SubscriptionTokenCaptureRequest {
            flow_kind: FlowKind::AnthropicSubscriptionOAuth,
            provider: ProviderFamily::Claude,
            verifier: good_verifier(),
            state_nonce: "nonce42".into(),
            loopback: OAuthLoopbackServer::default_claude(),
        };
        let flow = capture_subscription_token(&req).unwrap();
        let url = build_authorization_url(&flow).unwrap();
        assert!(url.starts_with(ANTHROPIC_AUTHORIZATION_ENDPOINT));
        assert!(url.contains("code_challenge="));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("state=nonce42"));
        assert!(url.contains("redirect_uri="));
        assert!(url.contains("scope="));
    }

    #[test]
    fn record_capture_carries_sref_only() {
        let sref = SecretReference::new("sref://abc-123-token-id".to_owned()).unwrap();
        let resp = record_capture(
            sref.clone(),
            FlowKind::AnthropicSubscriptionOAuth,
            ProviderFamily::Claude,
            1_700_000_000,
        );
        assert_eq!(resp.sref, sref);
        assert_eq!(resp.flow_kind, FlowKind::AnthropicSubscriptionOAuth);
        let dbg = format!("{:?}", resp.sref);
        assert!(dbg.contains("[REDACTED]"));
    }

    #[test]
    fn openai_flow_uses_openai_endpoints() {
        let req = SubscriptionTokenCaptureRequest {
            flow_kind: FlowKind::OpenAiOAuth,
            provider: ProviderFamily::OpenAiOrCodex,
            verifier: good_verifier(),
            state_nonce: "abc".into(),
            loopback: OAuthLoopbackServer::default_claude(),
        };
        let flow = capture_subscription_token(&req).unwrap();
        assert!(flow.authorization_endpoint.contains("openai.com"));
        assert!(flow.token_endpoint.contains("openai.com"));
    }

    #[test]
    fn flow_kind_names_distinct() {
        let s: std::collections::HashSet<&str> = [
            FlowKind::AnthropicSubscriptionOAuth,
            FlowKind::OpenAiOAuth,
            FlowKind::ApiKeyImport,
        ]
        .iter()
        .map(|f| f.name())
        .collect();
        assert_eq!(s.len(), 3);
    }

    #[test]
    fn oauth_error_display_distinct() {
        let m: Vec<String> = vec![
            format!("{}", OAuthError::InvalidVerifierLength),
            format!("{}", OAuthError::InvalidVerifierCharset),
            format!("{}", OAuthError::EmptyAuthorizationEndpoint),
            format!("{}", OAuthError::EmptyTokenEndpoint),
            format!("{}", OAuthError::EmptyScopes),
            format!("{}", OAuthError::EmptyStateNonce),
            format!("{}", OAuthError::ProviderRejected("x".into())),
            format!("{}", OAuthError::LoopbackPortReserved),
        ];
        let uniq: std::collections::HashSet<_> = m.iter().collect();
        assert_eq!(uniq.len(), m.len());
    }

    #[test]
    fn sha256_known_vector_abc() {
        // SHA-256("abc") = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
        let h = sha256(b"abc");
        let expected: [u8; 32] = [
            0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
            0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
            0xf2, 0x00, 0x15, 0xad,
        ];
        assert_eq!(h, expected);
    }

    #[test]
    fn base64url_known_vector() {
        let s = base64url_no_pad(b"foo");
        assert_eq!(s, "Zm9v");
    }

    #[test]
    fn url_encode_escapes_non_unreserved() {
        let s = url_encode("a b/c");
        assert_eq!(s, "a%20b%2Fc");
    }
}
