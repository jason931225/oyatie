//! Drives the tonic service impl directly, with no TCP socket, over the same
//! fixtures the REST suite uses.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::collections::HashMap;

use tonic::{Code, Request};

use iam_pdp_app::grpc::CloudIamPdpService;
use iam_pdp_app::grpc::proto::{self, cloud_iam_pdp_server::CloudIamPdp as _};

use common::{bob_read_link, seeded_state};

fn proto_entity_ref(entity_type: &str, entity_id: &str) -> proto::EntityRef {
    proto::EntityRef {
        entity_type: entity_type.to_owned(),
        entity_id: entity_id.to_owned(),
    }
}

fn string_attr(value: &str) -> proto::AttributeValue {
    proto::AttributeValue {
        value: Some(proto::attribute_value::Value::StringValue(value.to_owned())),
    }
}

fn proto_record(
    uid: proto::EntityRef,
    attributes: &[(&str, &str)],
    parents: Vec<proto::EntityRef>,
) -> proto::EntityRecord {
    proto::EntityRecord {
        uid: Some(uid),
        attributes: attributes
            .iter()
            .map(|(k, v)| ((*k).to_owned(), string_attr(v)))
            .collect(),
        parents,
    }
}

fn proto_entities() -> Vec<proto::EntityRecord> {
    vec![
        proto_record(
            proto_entity_ref("OyaPlatform::Tenant", "acme"),
            &[
                ("tenant_id", "acme"),
                ("cell_id", "cell-001"),
                ("lifecycle_state", "active"),
            ],
            vec![],
        ),
        proto_record(
            proto_entity_ref("OyaPlatform::Group", "tenant-admins"),
            &[("tenant_id", "acme")],
            vec![proto_entity_ref("OyaPlatform::Tenant", "acme")],
        ),
        proto_record(
            proto_entity_ref("OyaPlatform::Principal", "alice"),
            &[
                ("tenant_id", "acme"),
                ("kind", "human"),
                ("step_up_class", "a"),
            ],
            vec![proto_entity_ref("OyaPlatform::Group", "tenant-admins")],
        ),
        proto_record(
            proto_entity_ref("OyaPlatform::Principal", "bob"),
            &[("tenant_id", "acme"), ("kind", "human")],
            vec![],
        ),
        proto_record(
            proto_entity_ref("OyaPlatform::TenantResource", "acme-doc-1"),
            &[
                ("tenant_id", "acme"),
                ("resource_kind", "document"),
                ("data_class", "restricted"),
                ("cell_id", "cell-001"),
            ],
            vec![proto_entity_ref("OyaPlatform::Tenant", "acme")],
        ),
        proto_record(
            proto_entity_ref("OyaPlatform::TenantResource", "acme-doc-2"),
            &[
                ("tenant_id", "acme"),
                ("resource_kind", "document"),
                ("data_class", "internal"),
                ("cell_id", "cell-001"),
            ],
            vec![proto_entity_ref("OyaPlatform::Tenant", "acme")],
        ),
    ]
}

fn authorize_request(
    request_id: &str,
    principal_id: &str,
    action: &str,
    resource_id: &str,
) -> proto::AuthorizeRequest {
    proto::AuthorizeRequest {
        request_id: request_id.to_owned(),
        tenant_id: "acme".to_owned(),
        principal: Some(proto_entity_ref("OyaPlatform::Principal", principal_id)),
        action: action.to_owned(),
        resource: Some(proto_entity_ref("OyaPlatform::TenantResource", resource_id)),
        context: HashMap::new(),
        min_policy_version: String::new(),
        entities: proto_entities(),
    }
}

#[tokio::test]
async fn abac_allow_round_trips_with_zookie_echo() {
    let (state, sink) = seeded_state(vec![]);
    let service = CloudIamPdpService::new(state);
    let response = service
        .authorize(Request::new(authorize_request(
            "req-grpc-allow",
            "alice",
            "resource.read",
            "acme-doc-1",
        )))
        .await
        .expect("an allow is a response")
        .into_inner();
    assert_eq!(response.decision(), proto::DecisionEffect::Allow);
    assert_eq!(response.request_id, "req-grpc-allow");
    assert_eq!(response.policy_version, "psv-000001");
    assert!(!response.decision_id.is_empty());
    assert!(
        response
            .determining_policy_ids
            .contains(&"abac-step-up-restricted-read".to_owned()),
        "{response:?}"
    );
    assert_eq!(sink.records().len(), 1);
}

#[tokio::test]
async fn deny_is_a_decision_response_not_a_status_error() {
    let (state, sink) = seeded_state(vec![]);
    let service = CloudIamPdpService::new(state);
    let response = service
        .authorize(Request::new(authorize_request(
            "req-grpc-deny",
            "bob",
            "resource.read",
            // Deny-by-default leaves no determining policy: acme-doc-2 matches
            // neither a permit nor a forbid.
            "acme-doc-2",
        )))
        .await
        .expect("a deny is a response, NOT an RPC error")
        .into_inner();
    assert_eq!(response.decision(), proto::DecisionEffect::Deny);
    assert!(response.determining_policy_ids.is_empty());
    assert_eq!(sink.records().len(), 1, "denies are audited too");
}

#[tokio::test]
async fn pbac_link_decides_through_the_same_core() {
    let (state, _sink) = seeded_state(vec![bob_read_link()]);
    let service = CloudIamPdpService::new(state);
    let response = service
        .authorize(Request::new(authorize_request(
            "req-grpc-pbac",
            "bob",
            "resource.read",
            "acme-doc-2",
        )))
        .await
        .expect("PBAC allow")
        .into_inner();
    assert_eq!(response.decision(), proto::DecisionEffect::Allow);
    assert!(
        response
            .determining_policy_ids
            .contains(&"pbac-link-bob-acme-doc-2".to_owned()),
        "{response:?}"
    );
}

#[tokio::test]
async fn missing_principal_fails_closed_invalid_argument() {
    let (state, sink) = seeded_state(vec![]);
    let service = CloudIamPdpService::new(state);
    let mut request = authorize_request(
        "req-grpc-no-principal",
        "alice",
        "resource.read",
        "acme-doc-1",
    );
    request.principal = None;
    let status = service
        .authorize(Request::new(request))
        .await
        .expect_err("missing principal must refuse");
    assert_eq!(status.code(), Code::InvalidArgument);
    assert!(sink.is_empty(), "refusals never enter the decision audit");
}

#[tokio::test]
async fn unset_attribute_oneof_fails_closed() {
    let (state, _sink) = seeded_state(vec![]);
    let service = CloudIamPdpService::new(state);
    let mut request = authorize_request(
        "req-grpc-unset-attr",
        "alice",
        "resource.read",
        "acme-doc-1",
    );
    request
        .context
        .insert("channel".to_owned(), proto::AttributeValue { value: None });
    let status = service
        .authorize(Request::new(request))
        .await
        .expect_err("unset oneof must refuse, never silently default");
    assert_eq!(status.code(), Code::InvalidArgument);
}

#[tokio::test]
async fn unknown_action_is_invalid_argument() {
    let (state, _sink) = seeded_state(vec![]);
    let service = CloudIamPdpService::new(state);
    let status = service
        .authorize(Request::new(authorize_request(
            "req-grpc-unknown-action",
            "alice",
            "resource.delete",
            "acme-doc-1",
        )))
        .await
        .expect_err("unmapped action must refuse");
    assert_eq!(status.code(), Code::InvalidArgument);
}

#[tokio::test]
async fn stale_zookie_pin_is_failed_precondition() {
    let (state, _sink) = seeded_state(vec![]);
    let service = CloudIamPdpService::new(state);
    let mut request = authorize_request("req-grpc-stale", "alice", "resource.read", "acme-doc-1");
    request.min_policy_version = "psv-000099".to_owned();
    let status = service
        .authorize(Request::new(request))
        .await
        .expect_err("stale pin must refuse, never answer stale");
    assert_eq!(status.code(), Code::FailedPrecondition);
}

#[tokio::test]
async fn policy_version_probe_echoes_the_loaded_bundle() {
    let (state, _sink) = seeded_state(vec![]);
    let service = CloudIamPdpService::new(state);
    let response = service
        .get_loaded_policy_version(Request::new(proto::GetLoadedPolicyVersionRequest {}))
        .await
        .expect("version probe")
        .into_inner();
    assert_eq!(response.policy_version, "psv-000001");
}
