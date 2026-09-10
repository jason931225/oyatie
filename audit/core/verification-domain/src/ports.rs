//! Port traits this crate calls to verify a sealed period.
//!
//! `audit-verification-domain` depends only on `audit-chain-domain` and
//! `audit-verification-api` — it may NOT depend on
//! `audit_verification_kernel`, the port-only crate that already declares
//! `RootRegistry`, `KeyResolver`, and `MerkleVerifier`. Three of the four
//! traits below are this crate's OWN declarations of those equivalent
//! shapes, written independently so this crate never needs the dependency
//! edge; the fourth, [`RedactionRegistry`], has no `audit_verification_kernel`
//! counterpart at all and is this crate's own addition (see its doc for
//! why).

use audit_chain_domain::{Ed25519VerificationKey, Sha256Hash};

/// Fixed to `Key = Ed25519VerificationKey` (rather than a free associated
/// type) so [`crate::verify`] can call
/// `audit_chain_domain::Ed25519Signature::verify_with_trusted_key` directly
/// on the result — real Ed25519 verification, not a second port this crate
/// would otherwise have to invent to wrap it.
///
/// An `Err` here means "no key epoch this crate can find covers this
/// request" and [`crate::verify`] reports
/// [`VerificationFailureReason::KeyEpochMismatch`](audit_verification_api::VerificationFailureReason::KeyEpochMismatch)
/// for it, never a pass (L4: fail closed).
pub trait KeyResolver {
    type Error;

    fn resolve_key(
        &self,
        pack: &str,
        tenant_partition: &str,
        period_id: &str,
    ) -> Result<Ed25519VerificationKey, Self::Error>;
}

pub trait MerkleVerifier {
    fn verify(
        &self,
        leaf: &Sha256Hash,
        proof: &crate::request::MerkleInclusionProof,
        root: &Sha256Hash,
    ) -> bool;
}

pub trait RootRegistry {
    type Error;

    /// Resolves the trusted, previously published root that
    /// `(pack, tenant_partition, period_id)`'s immediate predecessor
    /// committed. `Err` when no such root can be confirmed (unreachable
    /// registry, no record on file, etc.) — [`crate::verify`] never treats
    /// that as a pass.
    fn resolve_root(
        &self,
        pack: &str,
        tenant_partition: &str,
        period_id: &str,
    ) -> Result<Sha256Hash, Self::Error>;

    /// Returns `Ok(true)` only when the registry can affirmatively confirm
    /// that no period precedes `period_id` for `(pack, tenant_partition)` —
    /// i.e. a [`crate::PriorRootClaim::First`] claim against it is
    /// genuinely true. `Ok(false)` and `Err` are both treated identically
    /// by [`crate::verify`]: neither is a confirmed "yes", so neither is a
    /// pass.
    fn is_first_period(
        &self,
        pack: &str,
        tenant_partition: &str,
        period_id: &str,
    ) -> Result<bool, Self::Error>;
}

/// [`crate::VerificationRequest::redacted`] is a plain `bool` field — every
/// bit as free to construct as [`crate::PriorRootClaim::First`] is free to
/// construct, and for the exact same reason [`RootRegistry::is_first_period`]
/// exists: a caller (or an attacker replaying a genuinely-signed record)
/// could otherwise simply set `redacted: false` and have a truly redacted
/// leaf's `RedactedEvent` verdict silently laundered into `Verified`,
/// because nothing about the Ed25519 signature or the Merkle proof says
/// anything about redaction status — [`crate::verification_signing_payload`]
/// does not cover it, deliberately, because redaction is a fact about
/// retention-cascade state *after* sealing, not something the record itself
/// could have attested to at signing time. [`crate::verify`] therefore never
/// takes `request.redacted` as the whole story: it also asks this registry,
/// and only a confirmed `Ok(false)` answer lets a would-be `Verified`
/// verdict actually pass step 6. `Ok(true)` and `Err` are both treated as
/// "not confirmed clean" and fail closed into
/// [`crate::VerificationFailureReason::RedactedEvent`] (L4) — an
/// unreachable or erroring registry can never be used to launder a
/// genuinely redacted leaf through as verified, and `request.redacted` on
/// its own can never override a registry that says otherwise.
pub trait RedactionRegistry {
    type Error;

    /// Returns `Ok(true)` only when the registry can affirmatively confirm
    /// that the leaf at `(pack, tenant_partition, period_id)` has been
    /// redacted by a retention-cascade action. `Ok(false)` means
    /// affirmatively confirmed clean. `Err` means the registry could not
    /// answer at all — [`crate::verify`] treats that identically to
    /// `Ok(true)`: neither is a confirmed "clean", so neither lets
    /// [`crate::verify`] report [`crate::VerificationVerdict::Verified`].
    fn is_redacted(
        &self,
        pack: &str,
        tenant_partition: &str,
        period_id: &str,
        leaf: &Sha256Hash,
    ) -> Result<bool, Self::Error>;
}
