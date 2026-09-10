//! Pure OAuth subscription flow construction.

use super::{
    FlowKind, OAuthError, OAuthLoopbackServer, PkceChallenge, ProviderFamily, SecretReference,
    SubscriptionOAuthFlow, SubscriptionTokenCaptureRequest, SubscriptionTokenCaptureResponse,
    url_encode,
};

/// Canonical scope set for subscription OAuth.
pub fn anthropic_subscription_scopes() -> Vec<String> {
    vec![
        "org:create_api_key".into(),
        "user:profile".into(),
        "user:inference".into(),
    ]
}

pub const ANTHROPIC_AUTHORIZATION_ENDPOINT: &str = "https://claude.ai/oauth/authorize";
pub const ANTHROPIC_TOKEN_ENDPOINT: &str = "https://console.anthropic.com/v1/oauth/token";

/// Builds the flow record only. Opening the browser, listening on the
/// loopback, exchanging the code and minting the `SecretReference` are the
/// runtime adapter's, so no raw token is ever in scope here.
pub fn capture_subscription_token(
    req: &SubscriptionTokenCaptureRequest,
) -> Result<SubscriptionOAuthFlow, OAuthError> {
    if req.state_nonce.is_empty() {
        return Err(OAuthError::EmptyStateNonce);
    }
    let loopback = &req.loopback;
    // Ports below 1024 are privileged and cannot be bound unprivileged.
    if loopback.port < 1024 {
        return Err(OAuthError::LoopbackPortReserved);
    }
    let challenge = PkceChallenge::derive_s256(&req.verifier);
    let (auth_ep, token_ep, scopes) = match req.flow_kind {
        FlowKind::AnthropicSubscriptionOAuth => (
            ANTHROPIC_AUTHORIZATION_ENDPOINT.to_owned(),
            ANTHROPIC_TOKEN_ENDPOINT.to_owned(),
            anthropic_subscription_scopes(),
        ),
        FlowKind::OpenAiOAuth => (
            "https://auth.openai.com/oauth/authorize".to_owned(),
            "https://auth.openai.com/oauth/token".to_owned(),
            vec!["openid".into(), "profile".into(), "offline_access".into()],
        ),
        // No upstream endpoint: the operator supplies the key, so the empty
        // endpoints below are the intended value and skip the checks after.
        FlowKind::ApiKeyImport => (String::new(), String::new(), vec!["import".into()]),
    };
    if req.flow_kind != FlowKind::ApiKeyImport {
        if auth_ep.is_empty() {
            return Err(OAuthError::EmptyAuthorizationEndpoint);
        }
        if token_ep.is_empty() {
            return Err(OAuthError::EmptyTokenEndpoint);
        }
        if scopes.is_empty() {
            return Err(OAuthError::EmptyScopes);
        }
    }
    Ok(SubscriptionOAuthFlow {
        flow_kind: req.flow_kind,
        provider: req.provider,
        authorization_endpoint: auth_ep,
        token_endpoint: token_ep,
        scopes,
        challenge,
        loopback: loopback.clone(),
        state_nonce: req.state_nonce.clone(),
    })
}

/// Build the authorization URL the browser will navigate to.
pub fn build_authorization_url(flow: &SubscriptionOAuthFlow) -> Result<String, OAuthError> {
    if flow.flow_kind == FlowKind::ApiKeyImport {
        // No browser step, so there is no URL to navigate to.
        return Ok(String::new());
    }
    if flow.authorization_endpoint.is_empty() {
        return Err(OAuthError::EmptyAuthorizationEndpoint);
    }
    let scope_list = flow.scopes.join(" ");
    let url = format!(
        "{}?response_type=code&code_challenge={}&code_challenge_method=S256&state={}&redirect_uri={}&scope={}",
        flow.authorization_endpoint,
        url_encode(flow.challenge.as_str()),
        url_encode(&flow.state_nonce),
        url_encode(&flow.loopback.redirect_uri()),
        url_encode(&scope_list),
    );
    Ok(url)
}

/// The caller supplies an already-minted `SecretReference`; a raw token has
/// no parameter to arrive through.
pub fn record_capture(
    sref: SecretReference,
    flow_kind: FlowKind,
    provider: ProviderFamily,
    captured_unix_secs: u64,
) -> SubscriptionTokenCaptureResponse {
    SubscriptionTokenCaptureResponse {
        sref,
        flow_kind,
        provider,
        captured_unix_secs,
    }
}
