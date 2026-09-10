//! The write path against the REAL durable adapter —
//! kill the process's handle, reopen the file, refold the replay, and
//! the projection is identical, poisons included.

use std::path::PathBuf;

use data_boundary_kernel::{DataClass, PrivacyDataClass};
use data_ontology_kernel::{
    ActionInvocationRequest, ActionPolicyDecision, ActionTypeDefinition, ActionTypeId,
    AutonomyTier, EntityTypeDefinition, EntityTypeId, EntityTypePropertyDefinition, OntologyEngine,
    PropertyTier,
};
use foundry_edits::{EditSet, OntologyEdit, WireDataClass, WireProperty, WireTier, WireValue};
use foundry_records_draft::{ActionEnvelope, RecordsLog, SealedEnvelope};
use foundry_records_sqlite_draft::SqliteRecordsLog;
use foundry_spine::{
    ActionSubmission, ApplyOutcome, PoisonReason, ProjectionState, apply_sealed, fold_from_scratch,
    submit,
};

fn internal() -> PrivacyDataClass {
    PrivacyDataClass::try_from(DataClass::InternalOnly).unwrap()
}

fn registry() -> OntologyEngine {
    let mut engine = OntologyEngine::default();
    engine
        .register_entity_type(
            EntityTypeDefinition::new(
                "ten_test",
                EntityTypeId::new("ety_reading").unwrap(),
                "Reading",
                vec![
                    EntityTypePropertyDefinition::new(
                        "name",
                        PropertyTier::Scalar,
                        internal(),
                        true,
                    )
                    .unwrap(),
                ],
                1,
            )
            .unwrap(),
        )
        .unwrap();
    engine
        .register_action_type(
            ActionTypeDefinition::new(
                "ten_test",
                ActionTypeId::new("aty_calibrate").unwrap(),
                EntityTypeId::new("ety_reading").unwrap(),
                "ops-console",
                AutonomyTier::T1Assist,
                "reading.calibrated",
            )
            .unwrap(),
        )
        .unwrap();
    engine
}

fn submission(object: &str, key: &str, name: &str) -> ActionSubmission {
    ActionSubmission {
        request: ActionInvocationRequest {
            tenant_id: "ten_test".into(),
            principal_id: "prn_alice".into(),
            action_id: ActionTypeId::new("aty_calibrate").unwrap(),
            entity_id: object.into(),
            idempotency_key: key.into(),
            requested_at_epoch_seconds: 1_700_000_000,
        },
        decision: ActionPolicyDecision {
            decision_id: "dec_1".into(),
            tenant_id: "ten_test".into(),
            principal_id: "prn_alice".into(),
            allowed_surfaces: vec!["ops-console".into()],
            autonomy_tier: AutonomyTier::T1Assist,
        },
        parameters: vec![],
        edits: EditSet::new(vec![
            OntologyEdit::create_object(
                "ety_reading",
                vec![
                    WireProperty::new(
                        "name",
                        WireTier::Scalar,
                        WireDataClass::InternalOnly,
                        WireValue::String(name.into()),
                    )
                    .unwrap(),
                ],
            )
            .unwrap(),
        ])
        .unwrap(),
    }
}

#[derive(Default)]
struct MemoryDenials {
    entries: Vec<SealedEnvelope>,
}

impl RecordsLog for MemoryDenials {
    fn append(
        &mut self,
        envelope: ActionEnvelope,
    ) -> Result<foundry_records_draft::Receipt, foundry_records_draft::RecordsLogError> {
        let receipt = foundry_records_draft::Receipt {
            ordinal: self.entries.len() as u64 + 1,
            object_sequence: 1,
            deduplicated: false,
        };
        self.entries.push(SealedEnvelope {
            envelope,
            receipt: receipt.clone(),
        });
        Ok(receipt)
    }

    fn replay(
        &self,
        _tenant_id: &str,
        _from_ordinal: u64,
    ) -> Result<Vec<SealedEnvelope>, foundry_records_draft::RecordsLogError> {
        Ok(self.entries.clone())
    }

    fn head(&self, _tenant_id: &str) -> Result<u64, foundry_records_draft::RecordsLogError> {
        Ok(self.entries.len() as u64)
    }
}

fn scratch_db(case: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "foundry-spine-{case}-{}-{}.sqlite",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_file(&path);
    path
}

#[test]
fn kill_reopen_refold_is_byte_identical() {
    let registry = registry();
    let path = scratch_db("kill-reopen");
    let live = {
        let mut log = SqliteRecordsLog::open(&path).unwrap();
        let mut denials = MemoryDenials::default();
        let mut projection = ProjectionState::new("ten_test", &registry);
        submit(
            submission("ent_r1", "idem_1", "Ada"),
            &mut log,
            &mut denials,
            &mut projection,
        )
        .unwrap();
        submit(
            submission("ent_r2", "idem_2", "Grace"),
            &mut log,
            &mut denials,
            &mut projection,
        )
        .unwrap();

        // Plant one poisoned entry: corruption written around the writer.
        let corrupt = ActionEnvelope::new(
            "ten_test",
            "ent_r3",
            "aty_calibrate",
            "idem_3",
            1,
            vec![0xFF],
            1_700_000_000_000,
        )
        .unwrap();
        let receipt = log.append(corrupt.clone()).unwrap();
        apply_sealed(
            &mut projection,
            &SealedEnvelope {
                envelope: corrupt,
                receipt,
            },
        );

        submit(
            submission("ent_r4", "idem_4", "Edsger"),
            &mut log,
            &mut denials,
            &mut projection,
        )
        .unwrap();
        projection
    }; // the handle dies here — the "kill".

    let reopened = SqliteRecordsLog::open(&path).unwrap();
    let replayed = reopened.replay("ten_test", 1).unwrap();
    assert_eq!(replayed.len(), 4, "all four entries survive the reopen");
    let refolded = fold_from_scratch("ten_test", &registry, &replayed);
    assert_eq!(refolded, live, "refold after reopen is byte-identical");
    assert_eq!(refolded.poison.len(), 1);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_fold_that_poisoned_never_reports_applied() {
    let registry = registry();
    let path = scratch_db("raced-append");
    let mut log = SqliteRecordsLog::open(&path).unwrap();
    let mut denials = MemoryDenials::default();
    let mut projection = ProjectionState::new("ten_test", &registry);

    // The log races ahead of the projection: this entry is appended around
    // the writer and never folded, so the advisory dry run still probes
    // ordinal 1 while the real append lands at 2.
    let raced = ActionEnvelope::new(
        "ten_test",
        "ent_r9",
        "aty_calibrate",
        "idem_raced",
        1,
        vec![0xFF],
        1_700_000_000_000,
    )
    .unwrap();
    log.append(raced).unwrap();

    let outcome = submit(
        submission("ent_r1", "idem_1", "Ada"),
        &mut log,
        &mut denials,
        &mut projection,
    )
    .expect("the append succeeds; the authoritative fold is what refuses");
    match outcome {
        ApplyOutcome::Poisoned { receipt, reason } => {
            assert!(!receipt.deduplicated);
            assert_eq!(receipt.ordinal, 2);
            assert_eq!(
                reason,
                PoisonReason::NonDenseOrdinal {
                    expected: 1,
                    found: 2
                }
            );
        }
        other => panic!("a poisoned fold must never report success: {other:?}"),
    }
    let _ = std::fs::remove_file(&path);
}
