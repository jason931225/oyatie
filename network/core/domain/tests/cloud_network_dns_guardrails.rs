#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use network_domain::{
    CloudNetworkError, CoreDnsPodMode, EdgeGatewayProvider, NetworkCniProvider,
    NetworkDnsCellGuardrail, NetworkDnsCellGuardrailCreate,
};

fn guardrail() -> NetworkDnsCellGuardrailCreate {
    NetworkDnsCellGuardrailCreate {
        tenant_id: "ten_alpha".to_string(),
        region: "region-alpha1".to_string(),
        cell_id: "cell-region-alpha1-a-001".to_string(),
        namespace: "mesh-cell-region-alpha1-a-001-network".to_string(),
        cni_provider: NetworkCniProvider::Cilium,
        edge_gateway: EdgeGatewayProvider::Envoy,
        default_deny_ingress: true,
        default_deny_egress: true,
        dns_egress_explicitly_allowed: true,
        cross_cell_default_traffic_allowed: false,
        envoy_external_authorization: true,
        envoy_failure_mode_allow: false,
        mtls_required: true,
        coredns_pod_mode: CoreDnsPodMode::Verified,
        evidence_ref: "evidence://cloud-network-dns/cell-guardrail/region-alpha1/a-001".to_string(),
    }
}

#[test]
fn cell_guardrail_requires_cilium_envoy_coredns_and_denies_cross_cell_by_default() {
    let policy = NetworkDnsCellGuardrail::new(guardrail()).expect("guardrail is valid");

    assert_eq!(policy.tenant_id(), "ten_alpha");
    assert_eq!(policy.region(), "region-alpha1");
    assert_eq!(policy.cell_id(), "cell-region-alpha1-a-001");
    assert_eq!(policy.namespace(), "mesh-cell-region-alpha1-a-001-network");
    assert_eq!(policy.cni_provider(), NetworkCniProvider::Cilium);
    assert_eq!(policy.edge_gateway(), EdgeGatewayProvider::Envoy);
    assert!(policy.default_deny_ingress());
    assert!(policy.default_deny_egress());
    assert!(policy.dns_egress_explicitly_allowed());
    assert!(!policy.cross_cell_default_traffic_allowed());
    assert!(policy.envoy_external_authorization());
    assert!(!policy.envoy_failure_mode_allow());
    assert!(policy.mtls_required());
    assert_eq!(policy.coredns_pod_mode(), CoreDnsPodMode::Verified);
    assert_eq!(
        policy.evidence_ref(),
        "evidence://cloud-network-dns/cell-guardrail/region-alpha1/a-001"
    );
}

#[test]
fn cell_guardrail_rejects_default_allow_and_missing_dns_exception() {
    let mut cross_cell_open = guardrail();
    cross_cell_open.cross_cell_default_traffic_allowed = true;
    assert_eq!(
        NetworkDnsCellGuardrail::new(cross_cell_open).unwrap_err(),
        CloudNetworkError::CrossCellDefaultTrafficForbidden
    );

    let mut ingress_open = guardrail();
    ingress_open.default_deny_ingress = false;
    assert_eq!(
        NetworkDnsCellGuardrail::new(ingress_open).unwrap_err(),
        CloudNetworkError::DefaultDenyIngressRequired
    );

    let mut egress_open = guardrail();
    egress_open.default_deny_egress = false;
    assert_eq!(
        NetworkDnsCellGuardrail::new(egress_open).unwrap_err(),
        CloudNetworkError::DefaultDenyEgressRequired
    );

    let mut dns_blocked = guardrail();
    dns_blocked.dns_egress_explicitly_allowed = false;
    assert_eq!(
        NetworkDnsCellGuardrail::new(dns_blocked).unwrap_err(),
        CloudNetworkError::DnsEgressExceptionRequired
    );
}

#[test]
fn cell_guardrail_rejects_insecure_envoy_and_coredns_modes() {
    let mut no_ext_authz = guardrail();
    no_ext_authz.envoy_external_authorization = false;
    assert_eq!(
        NetworkDnsCellGuardrail::new(no_ext_authz).unwrap_err(),
        CloudNetworkError::EnvoyExtAuthzRequired
    );

    let mut fail_open = guardrail();
    fail_open.envoy_failure_mode_allow = true;
    assert_eq!(
        NetworkDnsCellGuardrail::new(fail_open).unwrap_err(),
        CloudNetworkError::EnvoyFailClosedRequired
    );

    let mut no_mtls = guardrail();
    no_mtls.mtls_required = false;
    assert_eq!(
        NetworkDnsCellGuardrail::new(no_mtls).unwrap_err(),
        CloudNetworkError::MeshMtlsRequired
    );

    let mut insecure_dns = guardrail();
    insecure_dns.coredns_pod_mode = CoreDnsPodMode::Insecure;
    assert_eq!(
        NetworkDnsCellGuardrail::new(insecure_dns).unwrap_err(),
        CloudNetworkError::CoreDnsInsecurePodModeForbidden
    );
}

#[test]
fn cell_guardrail_validates_scope_and_evidence_without_secret_material() {
    let mut wrong_region_cell = guardrail();
    wrong_region_cell.cell_id = "cell-region-beta1-a-001".to_string();
    assert_eq!(
        NetworkDnsCellGuardrail::new(wrong_region_cell).unwrap_err(),
        CloudNetworkError::InvalidCellId
    );

    let mut namespace_drift = guardrail();
    namespace_drift.namespace = "mesh-cell-region-alpha1-b-002-network".to_string();
    assert_eq!(
        NetworkDnsCellGuardrail::new(namespace_drift).unwrap_err(),
        CloudNetworkError::InvalidMeshNamespace
    );

    let mut path_tenant = guardrail();
    path_tenant.tenant_id = "ten_alpha/../../root".to_string();
    assert_eq!(
        NetworkDnsCellGuardrail::new(path_tenant).unwrap_err(),
        CloudNetworkError::InvalidTenantId
    );

    let mut secret_evidence = guardrail();
    secret_evidence.evidence_ref = "evidence://cloud-network/token=secret".to_string();
    assert_eq!(
        NetworkDnsCellGuardrail::new(secret_evidence).unwrap_err(),
        CloudNetworkError::EvidenceRefLooksSecretLike
    );
}
