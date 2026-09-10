#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use application_app::{
    AutonomyTier, DataClass, Foundation, FoundationError, ObjectEntityUpsert, ObjectPropertyInput,
    OutboxPublish, PrivacyDataClass, PropertyTier, RegionalPackRegistration, TenantRegistration,
};

#[test]
fn foundation_publishes_regional_pack_object_graph_and_idempotent_outbox_contracts() {
    let mut foundation = Foundation::default();
    let tenant = foundation
        .onboard_tenant(TenantRegistration {
            tenant_id: "ten_gamma".into(),
            legal_name: "Gamma Corporate".into(),
            home_region: "region-home".into(),
            residency_class: "strict_home_region".into(),
            regulatory_packs: vec!["pack-alpha".into()],
            autonomy_ceiling: AutonomyTier::T2Advisory,
        })
        .expect("tenant can be onboarded");

    let pack = foundation
        .register_regional_pack(RegionalPackRegistration {
            pack_id: "pack-alpha".into(),
            region: "region-home".into(),
            residency_class: "strict_home_region".into(),
            controls: vec!["PIPA".into(), "K-ISMS-P".into(), "KCMVP".into()],
        })
        .expect("regional pack registration is valid");
    assert_eq!(
        pack.residency_class.value.label(),
        Some("strict_home_region")
    );

    let entity = foundation
        .upsert_object_entity(ObjectEntityUpsert {
            tenant_id: tenant.id.clone(),
            entity_id: "ent_employee_001".into(),
            entity_type: "employee".into(),
            properties: vec![
                ObjectPropertyInput::new(
                    "email".into(),
                    "worker@gamma.example".into(),
                    PropertyTier::Scalar,
                    privacy_data_class(DataClass::PiiIdentifying),
                ),
                ObjectPropertyInput::new(
                    "salary_band".into(),
                    "PRIMARY-4".into(),
                    PropertyTier::Struct,
                    privacy_data_class(DataClass::FinancialRegulatedCredit),
                ),
            ],
        })
        .expect("object graph upsert is valid");
    assert_eq!(entity.properties.len(), 2);
    let property_input = ObjectPropertyInput::new(
        "projection".into(),
        "value".into(),
        PropertyTier::Scalar,
        privacy_data_class(DataClass::PiiIdentifying),
    );
    assert_eq!(
        property_input.legacy_data_class(),
        DataClass::PiiIdentifying
    );
    #[allow(deprecated)]
    {
        assert_eq!(
            property_input.data_class(),
            property_input.legacy_data_class()
        );
    }
    assert!(
        entity.properties.values().all(|property| property
            .value
            .data_class
            .compatibility_data_class()
            != DataClass::Public)
    );

    let first = foundation
        .publish_outbox(OutboxPublish {
            tenant_id: tenant.id.clone(),
            topic: "oya.object-graph.entity.upserted.v1".into(),
            idempotency_key: "idem-entity-001".into(),
            payload_ref: entity.id.clone(),
        })
        .expect("outbox publish is valid");
    let duplicate = foundation
        .publish_outbox(OutboxPublish {
            tenant_id: tenant.id.clone(),
            topic: "oya.object-graph.entity.upserted.v1".into(),
            idempotency_key: "idem-entity-001".into(),
            payload_ref: entity.id,
        })
        .expect("duplicate publish is idempotent");

    assert_eq!(first.sequence, duplicate.sequence);
    assert!(!first.published);
    let published = foundation
        .mark_outbox_published(&tenant.id, first.sequence)
        .expect("dispatch worker can mark an outbox record published");
    assert!(published.published);
    let republished = foundation
        .mark_outbox_published(&tenant.id, first.sequence)
        .expect("marking the same outbox record is idempotent");
    assert_eq!(published, republished);
    assert_eq!(
        foundation.mark_outbox_published(&tenant.id, 9_999),
        Err(FoundationError::OutboxRecordNotFound)
    );
    assert_eq!(foundation.outbox_records().len(), 1);
    assert!(foundation.audit_chain().verify());
    assert!(
        foundation
            .audit_chain()
            .events()
            .iter()
            .any(|event| event.surface == "eventing.outbox.publish" && event.decision == "ALLOW")
    );
}

#[test]
fn object_graph_upsert_rejects_non_privacy_property_markers() {
    let mut foundation = Foundation::default();
    foundation
        .onboard_tenant(TenantRegistration {
            tenant_id: "ten_object_markers".into(),
            legal_name: "Object Marker Tenant".into(),
            home_region: "region-home".into(),
            residency_class: "strict_home_region".into(),
            regulatory_packs: vec!["pack-alpha".into()],
            autonomy_ceiling: AutonomyTier::T2Advisory,
        })
        .expect("tenant can be onboarded");
    let upsert_events_before = foundation
        .audit_chain()
        .events()
        .iter()
        .filter(|event| event.surface == "object-graph.entity.upsert")
        .count();

    for data_class in [DataClass::Audit, DataClass::Secret, DataClass::Children] {
        assert_eq!(
            ObjectPropertyInput::try_from_legacy_data_class(
                "marker".into(),
                "not a privacy class".into(),
                PropertyTier::Scalar,
                data_class,
            ),
            Err(FoundationError::InvalidInput)
        );
    }

    let upsert_events_after = foundation
        .audit_chain()
        .events()
        .iter()
        .filter(|event| event.surface == "object-graph.entity.upsert")
        .count();
    assert_eq!(upsert_events_after, upsert_events_before);
}

fn privacy_data_class(data_class: DataClass) -> PrivacyDataClass {
    PrivacyDataClass::try_from(data_class).expect("test fixture uses privacy data class")
}
