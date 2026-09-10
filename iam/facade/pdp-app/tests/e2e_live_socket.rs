//! Boots the real service on ephemeral loopback ports through the same
//! `server::start` that `main` calls, then drives both delivery surfaces over
//! real sockets.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use iam_pdp_app::grpc::proto;
use iam_pdp_app::server::{self, StartError};
use iam_pdp_kernel::PdpConfig;

use common::{
    entity_ref, entity_slice, request, seed_bundle, signed_bundle_doc, temp_bundle_file, trust_dir,
};

fn config_for(bundle_path: &std::path::Path) -> PdpConfig {
    PdpConfig {
        bundle_path: bundle_path.to_string_lossy().into_owned(),
        bundle_trust_dir: trust_dir("e2e").to_string_lossy().into_owned(),
        rest_addr: "127.0.0.1:0".to_owned(),
        grpc_addr: "127.0.0.1:0".to_owned(),
        decision_cache_capacity: 64,
        mtls_cert_dir: "/etc/cloud-iam-pdp/tls".to_owned(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn serves_decisions_over_live_rest_and_grpc_sockets() {
    let bundle = seed_bundle(common::SEED_VERSION, vec![]);
    let inner = serde_json::to_string(&bundle).unwrap();
    let path = temp_bundle_file("green", &signed_bundle_doc(&inner));
    let handle = server::start(&config_for(&path)).await.expect("boots");
    let base = format!("http://{}", handle.rest_addr);
    let client = reqwest::Client::new();

    let health: serde_json::Value = client
        .get(format!("{base}/healthz"))
        .send()
        .await
        .expect("healthz reachable")
        .json()
        .await
        .expect("healthz json");
    assert_eq!(health["status"], "ok");
    let ready: serde_json::Value = client
        .get(format!("{base}/readyz"))
        .send()
        .await
        .expect("readyz reachable")
        .json()
        .await
        .expect("readyz json");
    assert_eq!(ready["status"], "ready");
    assert_eq!(ready["policy_version"], common::SEED_VERSION);

    let allow_body = serde_json::json!({
        "request": request(
            "req-e2e-allow",
            "acme",
            entity_ref("OyaPlatform::Principal", "alice"),
            "tenant.administer",
            entity_ref("OyaPlatform::Tenant", "acme"),
        ),
        "entities": entity_slice().entities,
    });
    let response = client
        .post(format!("{base}/v1/authorize"))
        .json(&allow_body)
        .send()
        .await
        .expect("authorize reachable");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let json: serde_json::Value = response.json().await.expect("decision json");
    assert_eq!(json["decision"], "allow");
    assert_eq!(json["policy_version"], common::SEED_VERSION);

    let deny_body = serde_json::json!({
        "request": request(
            "req-e2e-deny",
            "acme",
            entity_ref("OyaPlatform::Principal", "bob"),
            "resource.read",
            entity_ref("OyaPlatform::TenantResource", "acme-doc-1"),
        ),
        "entities": entity_slice().entities,
    });
    let response = client
        .post(format!("{base}/v1/authorize"))
        .json(&deny_body)
        .send()
        .await
        .expect("authorize reachable");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let json: serde_json::Value = response.json().await.expect("decision json");
    assert_eq!(json["decision"], "deny");

    let response = client
        .get(format!("{base}/v1/bundles"))
        .send()
        .await
        .expect("reachable");
    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);

    let endpoint = format!("http://{}", handle.grpc_addr);
    let channel = tonic::transport::Endpoint::from_shared(endpoint)
        .expect("endpoint")
        .connect()
        .await
        .expect("grpc connects");
    let mut grpc = proto::cloud_iam_pdp_client::CloudIamPdpClient::new(channel);
    let version = grpc
        .get_loaded_policy_version(proto::GetLoadedPolicyVersionRequest {})
        .await
        .expect("version probe")
        .into_inner();
    assert_eq!(version.policy_version, common::SEED_VERSION);

    handle.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn missing_bundle_file_refuses_boot() {
    let config = config_for(std::path::Path::new("/nonexistent/iam-pdp/bundle.json"));
    let err = server::start(&config)
        .await
        .err()
        .expect("boot must refuse");
    assert!(matches!(err, StartError::Bundle(_)), "{err}");
}

#[tokio::test(flavor = "multi_thread")]
async fn malformed_bundle_json_refuses_boot() {
    let path = temp_bundle_file("red-garbage", "{ not a bundle");
    let err = server::start(&config_for(&path))
        .await
        .err()
        .expect("boot must refuse");
    assert!(matches!(err, StartError::Bundle(_)), "{err}");
}

#[tokio::test(flavor = "multi_thread")]
async fn unsigned_bundle_file_refuses_boot() {
    let bundle = seed_bundle(common::SEED_VERSION, vec![]);
    let inner = serde_json::to_string(&bundle).unwrap();
    let doc = serde_json::json!({ "bundle": inner, "signatures": [] });
    let path = temp_bundle_file("red-unsigned", &doc.to_string());
    let err = server::start(&config_for(&path))
        .await
        .err()
        .expect("unsigned bundle must refuse boot");
    assert!(matches!(err, StartError::Bundle(_)), "{err}");
    assert!(err.to_string().contains("refusing to boot"), "{err}");
}

#[tokio::test(flavor = "multi_thread")]
async fn tampered_signed_bundle_file_refuses_boot() {
    let bundle = seed_bundle(common::SEED_VERSION, vec![]);
    let inner = serde_json::to_string(&bundle).unwrap();
    let signed = signed_bundle_doc(&inner);
    let mut doc: serde_json::Value = serde_json::from_str(&signed).unwrap();
    let mut tampered = doc["bundle"].as_str().unwrap().to_owned().into_bytes();
    let idx = tampered.len() / 2;
    tampered[idx] = if tampered[idx] == b'x' { b'y' } else { b'x' };
    doc["bundle"] = serde_json::json!(String::from_utf8(tampered).unwrap());
    let path = temp_bundle_file("red-tampered", &doc.to_string());
    let err = server::start(&config_for(&path))
        .await
        .err()
        .expect("tampered bundle must refuse boot");
    assert!(matches!(err, StartError::Bundle(_)), "{err}");
}

#[tokio::test(flavor = "multi_thread")]
async fn empty_trust_anchor_refuses_boot() {
    let bundle = seed_bundle(common::SEED_VERSION, vec![]);
    let inner = serde_json::to_string(&bundle).unwrap();
    let path = temp_bundle_file("red-empty-trust", &signed_bundle_doc(&inner));
    let empty_trust =
        std::env::temp_dir().join(format!("iam-pdp-empty-trust-{}", std::process::id()));
    std::fs::create_dir_all(&empty_trust).unwrap();
    let config = PdpConfig {
        bundle_path: path.to_string_lossy().into_owned(),
        bundle_trust_dir: empty_trust.to_string_lossy().into_owned(),
        rest_addr: "127.0.0.1:0".to_owned(),
        grpc_addr: "127.0.0.1:0".to_owned(),
        decision_cache_capacity: 64,
        mtls_cert_dir: "/etc/cloud-iam-pdp/tls".to_owned(),
    };
    let err = server::start(&config)
        .await
        .err()
        .expect("empty trust anchor must refuse boot");
    assert!(matches!(err, StartError::Bundle(_)), "{err}");
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_cedar_policy_text_refuses_boot() {
    let mut bundle = seed_bundle(common::SEED_VERSION, vec![]);
    bundle.policies_src = "permit (principal, action".to_owned();
    // Signed, so the asserted PolicyLoad (not Bundle) proves the signature gate
    // does not mask policy-load checks.
    let inner = serde_json::to_string(&bundle).unwrap();
    let path = temp_bundle_file("red-cedar", &signed_bundle_doc(&inner));
    let err = server::start(&config_for(&path))
        .await
        .err()
        .expect("boot must refuse");
    assert!(matches!(err, StartError::PolicyLoad(_)), "{err}");
}
