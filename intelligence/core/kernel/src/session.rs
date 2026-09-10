//! Session pinning (pure kernel) for prompt-cache preservation.
//!
//! Rotating the upstream seat mid-conversation makes the provider rebuild its
//! prompt cache from scratch, so the gateway pins a logical session to one
//! seat and rebinds only on failover.
//!
//! Every derivation here is deterministic and side-effect free, so two gateway
//! replicas derive identical keys for identical requests.

use std::fmt::Write as _;

use sha2::{Digest, Sha256};

use crate::{Provider, TenantId};

/// Namespace prefix for keys derived from a client-supplied wire session id.
const WIRE_SESSION_PREFIX: &str = "wsid:";
/// Namespace prefix for keys derived from a first-user-message fingerprint.
const MESSAGE_PREFIX: &str = "sticky:";

/// Width in lowercase hex chars, so 64 bits of the digest survive.
const FINGERPRINT_HEX_LEN: usize = 16;
const _: () = assert!(FINGERPRINT_HEX_LEN.is_multiple_of(2));

/// Derive a sticky-affinity key from whatever session signal the request
/// carries; `None` means the caller leases without pinning.
///
/// A non-blank wire session id wins because it is stable across every turn,
/// where the fingerprint only matches a byte-identical first message.
pub fn derive_sticky_key(
    wire_session_id: Option<&str>,
    first_user_message: Option<&str>,
) -> Option<String> {
    if let Some(raw) = wire_session_id {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return Some(format!(
                "{WIRE_SESSION_PREFIX}{}",
                message_fingerprint(trimmed)
            ));
        }
    }
    first_user_message.map(message_sticky_key)
}

/// Raw prompt content never appears in the output.
pub(crate) fn message_sticky_key(first_user_message: &str) -> String {
    format!(
        "{MESSAGE_PREFIX}{}",
        message_fingerprint(first_user_message)
    )
}

fn message_fingerprint(first_user_message: &str) -> String {
    let digest = Sha256::digest(first_user_message.as_bytes());
    let mut hex = String::with_capacity(FINGERPRINT_HEX_LEN);
    for byte in digest.iter() {
        if hex.len() >= FINGERPRINT_HEX_LEN {
            break;
        }
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// Address the upstream prompt cache for a pinned session. The tenant is part
/// of the key, so shared cache infrastructure cannot serve one tenant's cached
/// prefix to another.
pub fn prompt_cache_key(
    tenant_id: &TenantId,
    provider: Provider,
    sticky_key: &str,
    model: &str,
) -> String {
    let mut key = String::with_capacity(
        "v1:".len()
            + tenant_id.as_str().len()
            + provider.to_string().len()
            + sticky_key.len()
            + model.len()
            + 32,
    );
    key.push_str("v1:");
    push_segment(&mut key, 't', tenant_id.as_str());
    push_segment(&mut key, 'p', &provider.to_string());
    push_segment(&mut key, 's', sticky_key);
    push_segment(&mut key, 'm', model);
    key
}

/// Length-prefixes each segment, so a value containing the delimiter cannot
/// forge a different key.
fn push_segment(key: &mut String, tag: char, value: &str) {
    let _ = write!(key, "{tag}{}:{value}", value.len());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_session_id_takes_precedence_over_message() {
        let key = derive_sticky_key(Some("conv-123"), Some("hello")).expect("a signal is present");
        assert!(key.starts_with("wsid:"));
        assert!(!key.contains("conv-123"));
        assert_eq!(key.len(), "wsid:".len() + FINGERPRINT_HEX_LEN);
    }

    #[test]
    fn falls_back_to_message_fingerprint_when_no_wire_id() {
        let key = derive_sticky_key(None, Some("the user prompt")).expect("message present");
        assert!(key.starts_with("sticky:"));
        assert!(!key.contains("the user prompt"));
        assert_eq!(key.len(), "sticky:".len() + FINGERPRINT_HEX_LEN);
    }

    #[test]
    fn blank_wire_id_is_ignored_in_favor_of_message() {
        let key = derive_sticky_key(Some("   "), Some("prompt")).expect("message present");
        assert!(key.starts_with("sticky:"));
    }

    #[test]
    fn returns_none_when_no_signal_present() {
        assert_eq!(derive_sticky_key(None, None), None);
        assert_eq!(derive_sticky_key(Some(""), None), None);
    }

    #[test]
    fn message_key_is_deterministic_and_collision_distinct() {
        assert_eq!(message_sticky_key("a"), message_sticky_key("a"));
        assert_ne!(message_sticky_key("a"), message_sticky_key("b"));
    }

    #[test]
    fn cache_key_is_tenant_provider_session_model() {
        let tenant = TenantId::new("tenant-a").unwrap();
        let sticky_key = derive_sticky_key(Some("conv-1"), None).unwrap();
        let key = prompt_cache_key(&tenant, Provider::Anthropic, &sticky_key, "claude-opus-4");
        assert_eq!(
            key,
            format!(
                "v1:t8:tenant-ap9:anthropics{}:{}m13:claude-opus-4",
                sticky_key.len(),
                sticky_key
            )
        );
    }

    #[test]
    fn cache_key_is_unambiguous_when_segments_contain_delimiters() {
        let tenant = TenantId::new("a::b").unwrap();
        let other_tenant = TenantId::new("a").unwrap();
        assert_ne!(
            prompt_cache_key(&tenant, Provider::Anthropic, "c", "d"),
            prompt_cache_key(&other_tenant, Provider::Anthropic, "b::c", "d")
        );
    }
}
