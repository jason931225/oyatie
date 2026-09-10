use std::collections::BTreeSet;
use std::sync::Arc;

use policy_bundle_content::{ContentIdentityError, content_digest};
use policy_pdp_bundle_file::{BundlePublishError, FilePolicyBundleStore};
use policy_pdp_cedar::CedarPdp;
use policy_pdp_kernel::{PdpError, PolicyBundle};
use serde::Serialize;
use shared_audit_event_kernel::ChainSigner;
use shared_platform_contracts_kernel::pdp::PolicyVersion;
use shared_ulid_id_kernel::IdGenerator;

use crate::{DecisionExpectation, PolicyProject};

/// Qualification must decide from the policies alone; a warm entry would let
/// one case's outcome answer another's request.
const QUALIFICATION_CACHE_DISABLED: usize = 0;

#[derive(Debug)]
pub enum QualificationError {
    Encoding {
        detail: String,
    },
    InvalidCases {
        detail: String,
    },
    Admission(PdpError),
    CaseRefused {
        name: String,
        error: PdpError,
    },
    CaseMismatch {
        name: String,
        expected: Box<DecisionExpectation>,
        actual: Box<DecisionExpectation>,
    },
}

#[derive(Clone, Debug, Serialize)]
pub struct QualificationReport {
    pub policy_version: PolicyVersion,
    /// Identity of both source and authored cases for this qualifier format.
    pub qualification_digest: String,
    pub passed_cases: usize,
}

/// Can only be constructed by successful real-engine qualification.
#[derive(Debug)]
pub struct PreparedPolicy {
    bundle: PolicyBundle,
    report: QualificationReport,
}

impl PolicyProject {
    /// A refusal is not a deny and cannot satisfy a deny expectation.
    ///
    /// # Errors
    /// Refuses empty/ambiguous case sets, invalid source, failed evaluation or mismatches.
    pub fn prepare(
        &self,
        id_gen: Arc<dyn IdGenerator>,
    ) -> Result<PreparedPolicy, QualificationError> {
        if self.cases.is_empty() {
            return Err(QualificationError::InvalidCases {
                detail: "at least one decision case is required".into(),
            });
        }
        let mut names = BTreeSet::new();
        for case in &self.cases {
            if case.name.trim().is_empty() || !names.insert(&case.name) {
                return Err(QualificationError::InvalidCases {
                    detail: "case names must be nonblank and unique".into(),
                });
            }
        }
        let bundle = self
            .source
            .candidate()
            .map_err(map_content_identity_error)?;
        let engine = CedarPdp::load(&bundle, id_gen, QUALIFICATION_CACHE_DISABLED)
            .map_err(QualificationError::Admission)?;
        for case in &self.cases {
            let outcome = engine
                .authorize_for_qualification(&case.request, &case.entities)
                .map_err(|error| QualificationError::CaseRefused {
                    name: case.name.clone(),
                    error,
                })?;
            let actual = DecisionExpectation {
                decision: outcome.response.decision,
                determining_policy_ids: outcome.response.determining_policy_ids,
                obligations: outcome.response.obligations,
            };
            if actual != case.expected {
                return Err(QualificationError::CaseMismatch {
                    name: case.name.clone(),
                    expected: Box::new(case.expected.clone()),
                    actual: Box::new(actual),
                });
            }
        }
        let report = QualificationReport {
            policy_version: bundle.version.clone(),
            qualification_digest: content_digest(b"oyatie-policy-qualification/v1\0", self)
                .map_err(map_content_identity_error)?,
            passed_cases: self.cases.len(),
        };
        Ok(PreparedPolicy { bundle, report })
    }
}

fn map_content_identity_error(error: ContentIdentityError) -> QualificationError {
    match error {
        ContentIdentityError::Encoding { detail } => QualificationError::Encoding { detail },
    }
}

impl PreparedPolicy {
    #[must_use]
    pub fn bundle(&self) -> &PolicyBundle {
        &self.bundle
    }

    #[must_use]
    pub fn report(&self) -> &QualificationReport {
        &self.report
    }

    /// Publish only the qualified candidate through configured trust and injected custody.
    /// No trust anchors or signing keys are created by this operation.
    ///
    /// # Errors
    /// Preserves transport errors, including committed-but-durability-unknown.
    pub fn publish(
        &self,
        store: &FilePolicyBundleStore,
        signer: &dyn ChainSigner,
        public_key: &[u8],
    ) -> Result<(), BundlePublishError> {
        store.write_signed_bundle(&self.bundle, signer, public_key)
    }
}
