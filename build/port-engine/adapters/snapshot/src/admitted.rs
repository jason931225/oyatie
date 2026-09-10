//! An admitted snapshot: the model, bound to the fleet pin, with its digests verified.

use port_engine_api::{Declaration, Digest, SourceModel, UnitId};
use port_engine_frontend_go::GoSourceModel;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedSnapshot {
    pub(crate) pin: String,
    pub(crate) artifact_digest: Digest,
    /// Verified against the preimage at admission, not merely copied out of the artifact.
    pub(crate) model_digest: Digest,
    pub(crate) model: GoSourceModel,
}

impl AdmittedSnapshot {
    #[must_use]
    pub fn pin(&self) -> &str {
        &self.pin
    }

    #[must_use]
    pub fn artifact_digest(&self) -> &Digest {
        &self.artifact_digest
    }

    #[must_use]
    pub fn model_digest(&self) -> &Digest {
        &self.model_digest
    }

    #[must_use]
    pub fn as_model(&self) -> &dyn SourceModel {
        self
    }

    #[must_use]
    pub fn producer_for(&self, unit: &UnitId) -> Option<&str> {
        self.model.producer_for(unit)
    }
}

impl SourceModel for AdmittedSnapshot {
    fn language(&self) -> &str {
        self.model.language()
    }

    fn snapshot_digest(&self) -> Digest {
        self.artifact_digest.clone()
    }

    fn units(&self) -> Vec<UnitId> {
        self.model.units()
    }

    fn declarations(&self, unit: &UnitId) -> Option<Vec<Declaration>> {
        self.model.declarations_for(unit)
    }
}
