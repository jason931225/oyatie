//! OAuth subscription capture value types.

use std::fmt;

use super::{ProviderFamily, SecretReference, base64url_no_pad, sha256};

/// data_class: INTERNAL_ONLY — high-entropy secret. Held in plain `String`
/// and not zeroized on drop, so a copy can outlive the flow.
#[derive(Clone, Eq, PartialEq)]
pub struct PkceVerifier(String);

impl PkceVerifier {
    pub fn new(raw: String) -> Result<Self, OAuthError> {
        if raw.len() < 43 || raw.len() > 128 {
            return Err(OAuthError::InvalidVerifierLength);
        }
        if !raw.chars().all(is_unreserved) {
            return Err(OAuthError::InvalidVerifierCharset);
        }
        Ok(Self(raw))
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for PkceVerifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PkceVerifier([REDACTED])")
    }
}

fn is_unreserved(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '~')
}

/// data_class: INTERNAL_ONLY — base64url-encoded SHA-256 of the verifier.
/// Safe to send over the wire.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PkceChallenge(String);

impl PkceChallenge {
    pub fn derive_s256(verifier: &PkceVerifier) -> Self {
        let digest = sha256(verifier.as_str().as_bytes());
        let encoded = base64url_no_pad(&digest);
        Self(encoded)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// data_class: INTERNAL_ONLY
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlowKind {
    AnthropicSubscriptionOAuth,
    OpenAiOAuth,
    ApiKeyImport,
}

impl FlowKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::AnthropicSubscriptionOAuth => "anthropic_subscription_oauth",
            Self::OpenAiOAuth => "openai_oauth",
            Self::ApiKeyImport => "api_key_import",
        }
    }
}

/// Describes the loopback only; the runtime adapter binds the listener.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OAuthLoopbackServer {
    pub port: u16,             // data_class: INTERNAL_ONLY
    pub callback_path: String, // data_class: INTERNAL_ONLY
}

impl OAuthLoopbackServer {
    pub const DEFAULT_PORT: u16 = 35593;
    pub const DEFAULT_PATH: &'static str = "/callback";

    pub fn default_claude() -> Self {
        Self {
            port: Self::DEFAULT_PORT,
            callback_path: Self::DEFAULT_PATH.to_owned(),
        }
    }

    pub fn redirect_uri(&self) -> String {
        format!("http://localhost:{}{}", self.port, self.callback_path)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscriptionOAuthFlow {
    pub flow_kind: FlowKind,            // data_class: INTERNAL_ONLY
    pub provider: ProviderFamily,       // data_class: INTERNAL_ONLY
    pub authorization_endpoint: String, // data_class: INTERNAL_ONLY
    pub token_endpoint: String,         // data_class: INTERNAL_ONLY
    pub scopes: Vec<String>,            // data_class: INTERNAL_ONLY
    pub challenge: PkceChallenge,       // data_class: INTERNAL_ONLY
    pub loopback: OAuthLoopbackServer,  // data_class: INTERNAL_ONLY
    pub state_nonce: String,            // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscriptionTokenCaptureRequest {
    pub flow_kind: FlowKind,           // data_class: INTERNAL_ONLY
    pub provider: ProviderFamily,      // data_class: INTERNAL_ONLY
    pub verifier: PkceVerifier,        // data_class: INTERNAL_ONLY
    pub state_nonce: String,           // data_class: INTERNAL_ONLY
    pub loopback: OAuthLoopbackServer, // data_class: INTERNAL_ONLY
}

/// There is no raw-token field, so a raw token cannot travel in a capture
/// response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscriptionTokenCaptureResponse {
    pub sref: SecretReference,    // data_class: INTERNAL_ONLY
    pub flow_kind: FlowKind,      // data_class: INTERNAL_ONLY
    pub provider: ProviderFamily, // data_class: INTERNAL_ONLY
    pub captured_unix_secs: u64,  // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OAuthError {
    InvalidVerifierLength,
    InvalidVerifierCharset,
    EmptyAuthorizationEndpoint,
    EmptyTokenEndpoint,
    EmptyScopes,
    EmptyStateNonce,
    ProviderRejected(String),
    LoopbackPortReserved,
}

impl fmt::Display for OAuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidVerifierLength => {
                write!(f, "PKCE verifier must be 43..=128 chars (RFC 7636)")
            }
            Self::InvalidVerifierCharset => {
                write!(
                    f,
                    "PKCE verifier must use unreserved chars [A-Za-z0-9\\-._~]"
                )
            }
            Self::EmptyAuthorizationEndpoint => write!(f, "authorization endpoint is empty"),
            Self::EmptyTokenEndpoint => write!(f, "token endpoint is empty"),
            Self::EmptyScopes => write!(f, "scope list is empty"),
            Self::EmptyStateNonce => write!(f, "state nonce is empty"),
            Self::ProviderRejected(s) => write!(f, "provider rejected token exchange: {s}"),
            Self::LoopbackPortReserved => write!(f, "loopback port is reserved"),
        }
    }
}
