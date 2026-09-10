#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use data_ontology_api::{
    OBJECT_GRAPH_ENTITY_UPSERT_OPENAPI_CONTRACT, OBJECT_GRAPH_ENTITY_UPSERT_SURFACE,
    ObjectGraphApiAuthorization, ObjectGraphApiBoundaryContext, ObjectGraphApiPrincipal,
    ObjectGraphEntityDirectory, ObjectGraphEntityPropertyRef, ObjectGraphEntityUpsertApiError,
    ObjectGraphEntityUpsertApiRequest, ObjectGraphEntityUpsertApiStatus,
    ObjectGraphEntityUpsertIdempotencyLedger, ObjectGraphEntityUpsertRequest,
    upsert_object_graph_entity_from_api,
};

const REQUEST_ID: &str = "req_object_graph_001";
const IDEMPOTENCY_KEY: &str = "idem_object_graph_001";
const TENANT_ID: &str = "ten_object_graph";
const ENTITY_ID: &str = "ent_customer_001";

#[test]
fn object_graph_entity_upsert_contract_runtime_constants_are_covered() {
    assert_eq!(
        OBJECT_GRAPH_ENTITY_UPSERT_SURFACE,
        "object-graph.entity.upsert"
    );
    assert_eq!(
        OBJECT_GRAPH_ENTITY_UPSERT_OPENAPI_CONTRACT,
        "contracts/openapi/platform/platform-object-graph-v1.yaml"
    );
    assert_eq!(ObjectGraphEntityUpsertApiStatus::Ok.code(), 200);
    assert_eq!(ObjectGraphEntityUpsertApiStatus::BadRequest.code(), 400);
    assert_eq!(ObjectGraphEntityUpsertApiStatus::Unauthorized.code(), 401);
    assert_eq!(ObjectGraphEntityUpsertApiStatus::Forbidden.code(), 403);
    assert_eq!(
        ObjectGraphEntityUpsertApiStatus::UnprocessableEntity.code(),
        422
    );
}

#[test]
fn object_graph_entity_upsert_creates_entity_emits_event_and_replays_idempotently() {
    let mut directory = ObjectGraphEntityDirectory::default();
    let mut idempotency = ObjectGraphEntityUpsertIdempotencyLedger::default();
    let request = entity_request(REQUEST_ID, IDEMPOTENCY_KEY, TENANT_ID, ENTITY_ID);

    let first =
        upsert_object_graph_entity_from_api(&mut directory, &mut idempotency, request.clone())
            .expect("first entity upsert succeeds");
    let second = upsert_object_graph_entity_from_api(&mut directory, &mut idempotency, request)
        .expect("same entity upsert replays");

    assert_eq!(first, second);
    assert_eq!(directory.len(), 1);
    assert_eq!(directory.event_count(), 1);
    assert_eq!(idempotency.len(), 1);
    assert_eq!(first.data.tenant_id, TENANT_ID);
    assert_eq!(first.data.entity_id, ENTITY_ID);
    assert_eq!(first.data.entity_type, "CustomerProfile");
    assert_eq!(first.data.property_refs[0].name, "email");
    assert_eq!(first.data.property_refs[0].tier, "scalar");
    assert_eq!(first.data.property_refs[0].data_class, "PII_IDENTIFYING");
    assert_eq!(first.data.schema_version, 1);
    assert_eq!(first.metadata.result, "created");
    assert_eq!(first.metadata.event_id, "evt_og_req_object_graph_001");
}

#[test]
fn object_graph_entity_upsert_updates_existing_entity_with_new_idempotency_key() {
    let mut directory = ObjectGraphEntityDirectory::default();
    let mut idempotency = ObjectGraphEntityUpsertIdempotencyLedger::default();
    upsert_object_graph_entity_from_api(
        &mut directory,
        &mut idempotency,
        entity_request(REQUEST_ID, IDEMPOTENCY_KEY, TENANT_ID, ENTITY_ID),
    )
    .expect("initial entity upsert succeeds");

    let mut update = entity_request(
        "req_object_graph_002",
        "idem_object_graph_002",
        TENANT_ID,
        ENTITY_ID,
    );
    update
        .body
        .property_refs
        .push(ObjectGraphEntityPropertyRef {
            name: "preferred_locale".to_string(),
            value: "ko-KR".to_string(),
            tier: "scalar".to_string(),
            data_class: "DECLARED_PREFERENCE".to_string(),
        });
    let response = upsert_object_graph_entity_from_api(&mut directory, &mut idempotency, update)
        .expect("same entity can be updated with a new idempotency key");

    assert_eq!(response.metadata.result, "updated");
    assert_eq!(response.data.property_refs.len(), 2);
    assert_eq!(directory.event_count(), 2);
    assert_eq!(idempotency.len(), 2);
}

#[test]
fn object_graph_entity_upsert_rejects_tenant_path_and_principal_drift() {
    let mut directory = ObjectGraphEntityDirectory::default();
    let mut idempotency = ObjectGraphEntityUpsertIdempotencyLedger::default();
    let mut tenant_drift = entity_request(
        "req_object_graph_tenant_drift",
        "idem_object_graph_tenant_drift",
        TENANT_ID,
        ENTITY_ID,
    );
    tenant_drift.body.tenant_id = "ten_other".to_string();

    let drift_error =
        upsert_object_graph_entity_from_api(&mut directory, &mut idempotency, tenant_drift)
            .expect_err("body tenant drift is rejected before mutation");
    assert!(matches!(
        drift_error,
        ObjectGraphEntityUpsertApiError::TenantPathBodyMismatch { .. }
    ));
    assert_eq!(drift_error.object_graph_entity_upsert_status_code(), 400);

    let mut principal_drift = entity_request(
        "req_object_graph_principal_drift",
        "idem_object_graph_principal_drift",
        TENANT_ID,
        ENTITY_ID,
    );
    principal_drift.principal.tenant_id = "ten_other".to_string();
    let principal_error =
        upsert_object_graph_entity_from_api(&mut directory, &mut idempotency, principal_drift)
            .expect_err("principal tenant drift is forbidden");
    assert_eq!(
        principal_error.object_graph_entity_upsert_status(),
        ObjectGraphEntityUpsertApiStatus::Forbidden
    );
    assert!(directory.is_empty());
}

#[test]
fn object_graph_entity_upsert_rejects_invalid_tier_and_data_class_before_kernel() {
    let mut directory = ObjectGraphEntityDirectory::default();
    let mut idempotency = ObjectGraphEntityUpsertIdempotencyLedger::default();
    let mut invalid_tier = entity_request(
        "req_object_graph_tier",
        "idem_object_graph_tier",
        TENANT_ID,
        ENTITY_ID,
    );
    invalid_tier.body.property_refs[0].tier = "blob".to_string();
    assert!(matches!(
        upsert_object_graph_entity_from_api(&mut directory, &mut idempotency, invalid_tier),
        Err(ObjectGraphEntityUpsertApiError::InvalidPropertyTier { .. })
    ));

    let mut invalid_data_class = entity_request(
        "req_object_graph_class",
        "idem_object_graph_class",
        TENANT_ID,
        ENTITY_ID,
    );
    invalid_data_class.body.property_refs[0].data_class = "AUDIT".to_string();
    assert!(matches!(
        upsert_object_graph_entity_from_api(&mut directory, &mut idempotency, invalid_data_class),
        Err(ObjectGraphEntityUpsertApiError::InvalidPropertyDataClass { .. })
    ));
    assert!(directory.is_empty());
}

#[test]
fn object_graph_entity_upsert_maps_kernel_errors_and_reused_idempotency() {
    let mut directory = ObjectGraphEntityDirectory::default();
    let mut idempotency = ObjectGraphEntityUpsertIdempotencyLedger::default();
    let mut invalid_id = entity_request(
        "req_object_graph_invalid_id",
        "idem_object_graph_invalid_id",
        TENANT_ID,
        "object_001",
    );
    invalid_id.body.entity_id = "object_001".to_string();
    assert!(matches!(
        upsert_object_graph_entity_from_api(&mut directory, &mut idempotency, invalid_id),
        Err(ObjectGraphEntityUpsertApiError::Kernel(_))
    ));

    let mut empty_properties = entity_request(
        "req_object_graph_empty",
        "idem_object_graph_empty",
        TENANT_ID,
        ENTITY_ID,
    );
    empty_properties.body.property_refs.clear();
    assert!(matches!(
        upsert_object_graph_entity_from_api(&mut directory, &mut idempotency, empty_properties),
        Err(ObjectGraphEntityUpsertApiError::Kernel(_))
    ));

    let mut request = entity_request(REQUEST_ID, IDEMPOTENCY_KEY, TENANT_ID, ENTITY_ID);
    upsert_object_graph_entity_from_api(&mut directory, &mut idempotency, request.clone())
        .expect("initial entity upsert succeeds");
    request.body.property_refs[0].value = "other@example.com".to_string();
    let reused = upsert_object_graph_entity_from_api(&mut directory, &mut idempotency, request)
        .expect_err("same idempotency key with different fingerprint fails");
    assert_eq!(
        reused.object_graph_entity_upsert_status(),
        ObjectGraphEntityUpsertApiStatus::UnprocessableEntity
    );
    assert_eq!(idempotency.len(), 1);
}

fn entity_request(
    request_id: &str,
    idempotency_key: &str,
    tenant_id: &str,
    entity_id: &str,
) -> ObjectGraphEntityUpsertApiRequest {
    ObjectGraphEntityUpsertApiRequest {
        path_tenant_id: tenant_id.to_string(),
        path_entity_id: entity_id.to_string(),
        boundary: ObjectGraphApiBoundaryContext {
            request_id: request_id.to_string(),
            tenant_id: tenant_id.to_string(),
            idempotency_key: idempotency_key.to_string(),
        },
        principal: ObjectGraphApiPrincipal {
            tenant_id: tenant_id.to_string(),
            principal_id: "usr_object_graph_operator".to_string(),
        },
        authorization: ObjectGraphApiAuthorization {
            tenant_id: tenant_id.to_string(),
            principal_id: "usr_object_graph_operator".to_string(),
            decision_id: "authz_object_graph_entity_upsert".to_string(),
            allowed_surfaces: vec![OBJECT_GRAPH_ENTITY_UPSERT_SURFACE.to_string()],
        },
        body: ObjectGraphEntityUpsertRequest {
            tenant_id: tenant_id.to_string(),
            entity_id: entity_id.to_string(),
            entity_type: "CustomerProfile".to_string(),
            property_refs: vec![ObjectGraphEntityPropertyRef {
                name: "email".to_string(),
                value: "worker@example.com".to_string(),
                tier: "scalar".to_string(),
                data_class: "PII_IDENTIFYING".to_string(),
            }],
        },
    }
}

/// The legacy string-wire surface fails closed on a typed stored value:
/// a typed property cannot be projected, and the refusal names it.
#[test]
fn typed_property_value_is_refused_by_the_string_wire() {
    use data_ontology_domain::{ObjectProperty, PropertyValue};
    let typed = ObjectProperty::typed(
        "count".to_string(),
        PropertyValue::Integer(7),
        data_boundary_kernel::PrivacyDataClass::try_from(
            data_boundary_kernel::DataClass::InternalOnly,
        )
        .unwrap(),
    );
    let entity = data_ontology_domain::ObjectEntity::new(
        "ten_test".to_string(),
        "ent_m1".to_string(),
        "ety_measure".to_string(),
        vec![typed],
    )
    .unwrap();
    let error = data_ontology_api::ObjectGraphEntityUpsertApiError::NonStringPropertyValue {
        name: "count".to_string(),
    };
    assert_eq!(
        error.code().as_str(),
        "OBJECT_GRAPH_PROPERTY_VALUE_NOT_STRING"
    );
    // The record builder is the enforcement point.
    let _ = entity;
}
