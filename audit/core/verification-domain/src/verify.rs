use audit_chain_domain::{Ed25519VerificationKey, Sha256Hash};
use audit_verification_api::{VerificationFailureReason, VerificationVerdict};

use crate::ports::{KeyResolver, MerkleVerifier, RedactionRegistry, RootRegistry};
use crate::request::{PriorRootClaim, VerificationRequest};

/// The exact canonical bytes a `record_pack` / `record_tenant_partition` /
/// `record_period_id` / `merkle_root` combination must be signed over for
/// [`verify`] to accept the signature. A real signer (whatever composes
/// `audit_sealing_domain::build_seal_record`'s output with a `SignerPort`
/// adapter) MUST sign exactly this construction — a different framing,
/// field order, or byte value produces a signature `verify` will reject as
/// [`VerificationFailureReason::SignatureInvalid`], never as a pass.
///
/// Deliberately built from the record's OWN identity fields, not the
/// verification request's `context_*` identity: the signature attests to
/// what the record itself is (its `(pack, tenant_partition, period_id,
/// merkle_root)`), independent of which context a caller later asks to
/// verify it under. That separation is what lets [`verify`] tell a
/// [`VerificationFailureReason::SignatureInvalid`] apart from a
/// [`VerificationFailureReason::PackMismatch`] — see [`crate::request`]'s
/// module doc.
pub fn verification_signing_payload(
    record_pack: &str,
    record_tenant_partition: &str,
    record_period_id: &str,
    merkle_root: &Sha256Hash,
) -> Vec<u8> {
    const DOMAIN_TAG: &[u8] = b"audit-verification-domain-v2";
    let mut out = Vec::with_capacity(
        DOMAIN_TAG.len()
            + 8 * 3
            + record_pack.len()
            + record_tenant_partition.len()
            + record_period_id.len()
            + merkle_root.len(),
    );
    out.extend_from_slice(DOMAIN_TAG);
    push_length_prefixed(&mut out, record_pack.as_bytes());
    push_length_prefixed(&mut out, record_tenant_partition.as_bytes());
    push_length_prefixed(&mut out, record_period_id.as_bytes());
    out.extend_from_slice(merkle_root); // fixed-size: no length prefix needed
    out
}

fn push_length_prefixed(out: &mut Vec<u8>, field: &[u8]) {
    out.extend_from_slice(&(field.len() as u64).to_be_bytes());
    out.extend_from_slice(field);
}

/// Verify `request` against the trust material `key_resolver`,
/// `root_registry`, `merkle_verifier`, and `redaction_registry` supply, and
/// return the resulting [`VerificationVerdict`]. Never panics, never
/// mutates `request` or anything reachable through the four ports.
pub fn verify<KR, RR, MV, RG>(
    request: &VerificationRequest,
    key_resolver: &KR,
    root_registry: &RR,
    merkle_verifier: &MV,
    redaction_registry: &RG,
) -> VerificationVerdict
where
    KR: KeyResolver,
    RR: RootRegistry,
    MV: MerkleVerifier,
    RG: RedactionRegistry,
{
    let Some(key) = resolve_context_trusted_key(request, key_resolver) else {
        return VerificationVerdict::Failed(VerificationFailureReason::KeyEpochMismatch);
    };
    if !record_signature_is_valid(request, &key) {
        return VerificationVerdict::Failed(VerificationFailureReason::SignatureInvalid);
    }
    if !leaf_inclusion_is_proven(request, merkle_verifier) {
        return VerificationVerdict::Failed(VerificationFailureReason::ProofInvalid);
    }
    if !prior_root_claim_is_confirmed(request, root_registry) {
        return VerificationVerdict::Failed(VerificationFailureReason::PriorRootMissing);
    }
    if !record_identity_matches_context(request) {
        return VerificationVerdict::Failed(VerificationFailureReason::PackMismatch);
    }
    if !redaction_is_confirmed_clean(request, redaction_registry) {
        return VerificationVerdict::Failed(VerificationFailureReason::RedactedEvent);
    }
    VerificationVerdict::Verified
}

fn resolve_context_trusted_key<KR: KeyResolver>(
    request: &VerificationRequest,
    key_resolver: &KR,
) -> Option<Ed25519VerificationKey> {
    key_resolver
        .resolve_key(
            &request.context_pack,
            &request.context_tenant_partition,
            &request.context_period_id,
        )
        .ok()
}

fn record_signature_is_valid(request: &VerificationRequest, key: &Ed25519VerificationKey) -> bool {
    let payload = verification_signing_payload(
        &request.record_pack,
        &request.record_tenant_partition,
        &request.record_period_id,
        &request.merkle_root,
    );
    request
        .signature
        .verify_with_trusted_key(&payload, key)
        .is_ok()
}

fn leaf_inclusion_is_proven<MV: MerkleVerifier>(
    request: &VerificationRequest,
    merkle_verifier: &MV,
) -> bool {
    merkle_verifier.verify(&request.leaf, &request.proof, &request.merkle_root)
}

fn prior_root_claim_is_confirmed<RR: RootRegistry>(
    request: &VerificationRequest,
    root_registry: &RR,
) -> bool {
    match &request.prior_root {
        PriorRootClaim::First => matches!(
            root_registry.is_first_period(
                &request.context_pack,
                &request.context_tenant_partition,
                &request.context_period_id,
            ),
            Ok(true)
        ),
        PriorRootClaim::Preceding { root } => matches!(
            root_registry.resolve_root(
                &request.context_pack,
                &request.context_tenant_partition,
                &request.context_period_id,
            ),
            Ok(resolved) if resolved == *root
        ),
    }
}

fn record_identity_matches_context(request: &VerificationRequest) -> bool {
    request.record_pack == request.context_pack
        && request.record_tenant_partition == request.context_tenant_partition
        && request.record_period_id == request.context_period_id
}

fn redaction_is_confirmed_clean<RG: RedactionRegistry>(
    request: &VerificationRequest,
    redaction_registry: &RG,
) -> bool {
    let registry_confirms_clean = matches!(
        redaction_registry.is_redacted(
            &request.context_pack,
            &request.context_tenant_partition,
            &request.context_period_id,
            &request.leaf,
        ),
        Ok(false)
    );
    !request.redacted && registry_confirms_clean
}
