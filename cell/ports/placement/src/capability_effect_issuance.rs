//! The producing side of the capability effect boundary.
//!
//! Without these ports the grant is consumed and never produced: the contract
//! states that a Tenancy owner issues it, and states nothing about what that
//! owner is allowed to sign.
//!
//! The two issuing roles are separate traits rather than two methods on one,
//! because revision 2 requires that a preparation issuer signing Write and a
//! local serving issuer signing a control preparation both refuse. With one
//! trait that refusal can only be a runtime check every implementer must
//! remember; with two, a control-owner preparation issuer has no method that
//! accepts an installed issuance at all, and the cross-role case stops
//! compiling.

use crate::{
    BoxCellFuture, CapabilityEffectErrorV1, CapabilityEffectGrantPayloadV1,
    SignedCapabilityEffectGrantV1, VerifiedCapabilityAdapterAcceptanceV1,
};

/// An assembled authorization to sign exactly one grant under installed
/// serving authority.
///
/// The port takes this and never a bare [`CapabilityEffectGrantPayloadV1`]: a
/// signer that accepted the payload directly would be a signing oracle, able
/// to mint any action, under any context, at any scope, for whoever asked.
///
/// Assembly is what constrains the signature. It checks that the payload's
/// action is one of Activate, Write, Fence, Release or Transfer; that the
/// payload carries an Installed context; that the action appears in the
/// acceptance's `supported_actions`; and that the payload's
/// `acceptance_digest` is the digest of that same verified acceptance.
#[derive(Debug, Eq, PartialEq)]
pub struct AuthorizedInstalledEffectIssuanceV1 {
    payload: CapabilityEffectGrantPayloadV1,
    acceptance: VerifiedCapabilityAdapterAcceptanceV1,
}

impl AuthorizedInstalledEffectIssuanceV1 {
    pub fn assemble(
        _payload: CapabilityEffectGrantPayloadV1,
        _acceptance: VerifiedCapabilityAdapterAcceptanceV1,
    ) -> Result<Self, CapabilityEffectErrorV1> {
        Err(CapabilityEffectErrorV1::NotImplemented)
    }

    #[must_use]
    pub fn payload(&self) -> &CapabilityEffectGrantPayloadV1 {
        &self.payload
    }

    #[must_use]
    pub fn acceptance(&self) -> &VerifiedCapabilityAdapterAcceptanceV1 {
        &self.acceptance
    }
}

/// An assembled authorization to sign exactly one grant under preparation
/// authority.
///
/// Assembly checks that the payload's action is one of Prepare,
/// PreparationCleanup, or Transfer into non-serving staged data; that the
/// payload carries a Preparation context; that the action appears in the
/// acceptance's `supported_actions`; and that the payload's
/// `acceptance_digest` is the digest of that same verified acceptance.
///
/// No preparation issuance can name Activate, Write, Fence or Release, so a
/// preparation grant for those actions cannot be assembled, let alone signed.
#[derive(Debug, Eq, PartialEq)]
pub struct AuthorizedPreparationEffectIssuanceV1 {
    payload: CapabilityEffectGrantPayloadV1,
    acceptance: VerifiedCapabilityAdapterAcceptanceV1,
}

impl AuthorizedPreparationEffectIssuanceV1 {
    pub fn assemble(
        _payload: CapabilityEffectGrantPayloadV1,
        _acceptance: VerifiedCapabilityAdapterAcceptanceV1,
    ) -> Result<Self, CapabilityEffectErrorV1> {
        Err(CapabilityEffectErrorV1::NotImplemented)
    }

    #[must_use]
    pub fn payload(&self) -> &CapabilityEffectGrantPayloadV1 {
        &self.payload
    }

    #[must_use]
    pub fn acceptance(&self) -> &VerifiedCapabilityAdapterAcceptanceV1 {
        &self.acceptance
    }
}

/// Implemented by the Tenancy-owned issuer that runs inside the serving cell,
/// composed with the local write-authority service, token ledger and serving
/// authority lease renewal.
///
/// It consumes its installed serving instance, the current local lease and
/// token issuance and a local authorization decision. No synchronous global
/// control lookup, central grant signer, manifest fetch or global epoch
/// allocation is admitted on this path, which is what lets a normal local
/// write survive a global outage. Missing local authority, or an inability to
/// establish the current local fence or clock, fails closed.
pub trait CapabilityInstalledEffectGrantIssuerV1: Send + Sync {
    /// Returns the raw signed grant, never a verified wrapper. The intended
    /// implementer is an out-of-crate Tenancy adapter, which cannot construct
    /// a private-field type; and a signer that returned its own evidence
    /// would be attesting to its own output.
    fn sign_installed<'a>(
        &'a self,
        issuance: &'a AuthorizedInstalledEffectIssuanceV1,
    ) -> BoxCellFuture<'a, Result<SignedCapabilityEffectGrantV1, CapabilityEffectErrorV1>>;
}

/// Implemented by the Tenancy control owner that issues preparation grants
/// from a binding reservation attempt, a verified placement decision and
/// reservation permit, and the complete initial or migration participant
/// manifest.
///
/// Its proof producer authorization is distinct from the local installed
/// serving issuer's. Neither role's producer may sign for the other, and the
/// separation is carried here by the input type, not by convention.
pub trait CapabilityPreparationEffectGrantIssuerV1: Send + Sync {
    /// Returns the raw signed grant, for the same two reasons as
    /// [`CapabilityInstalledEffectGrantIssuerV1::sign_installed`].
    fn sign_preparation<'a>(
        &'a self,
        issuance: &'a AuthorizedPreparationEffectIssuanceV1,
    ) -> BoxCellFuture<'a, Result<SignedCapabilityEffectGrantV1, CapabilityEffectErrorV1>>;
}
