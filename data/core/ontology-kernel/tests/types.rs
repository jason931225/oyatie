#![allow(clippy::expect_used, clippy::panic)]

use data_boundary_kernel::{DataClass, PrivacyDataClass};
use data_ontology_kernel::{
    ObjectEntity, ObjectEntityUpsertOutcome, ObjectGraph, ObjectProperty, PropertyTier,
};

#[test]
fn test_register_type() {
    let mut graph = ObjectGraph::default();
    let patient = ObjectEntity::new(
        "tenant_health".into(),
        "ent_patient_001".into(),
        "Patient".into(),
        vec![ObjectProperty::new(
            "email".into(),
            "patient@example.com".into(),
            PropertyTier::Scalar,
            PrivacyDataClass::try_from(DataClass::PiiIdentifying)
                .expect("PII label is a privacy data class"),
        )],
    )
    .expect("valid typed entity registers");

    assert_eq!(
        graph.upsert_entity(patient),
        Ok(ObjectEntityUpsertOutcome::Created)
    );
    let registered = graph
        .get("tenant_health", "ent_patient_001")
        .expect("registered entity is queryable");
    assert_eq!(registered.entity_type.value, "Patient");
}
