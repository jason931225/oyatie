use super::*;

fn provider_direct_interconnect_create_request() -> NetworkProviderDirectInterconnectCreateRequest {
    NetworkProviderDirectInterconnectCreateRequest {
        request_id: "networkprov_req_interconnect_create_001".to_string(),
        provider_virtual_circuit_ref: "oci-fast-connect://ocid1.compartment.oc1..cloud/ap-chuncheon-1/oyatie:cloud:region-alpha1:ten_alpha:direct-interconnect:fabric-a-primary".to_string(),
        interconnect_partners: interconnect_partner_creates(),
        direct_interconnect: direct_interconnect_create(),
        actor: "sp_network".to_string(),
        idempotency_key: "idem-network-interconnect-create".to_string(),
        requested_at_epoch_seconds: 1_700_000_060,
    }
}

fn provider_vpc_create_request() -> NetworkProviderVpcCreateRequest {
    NetworkProviderVpcCreateRequest {
        request_id: "networkprov_req_vpc_create_001".to_string(),
        provider_vcn_ref: "oci-vcn://ocid1.compartment.oc1..cloud/ap-chuncheon-1/oyatie:cloud:region-alpha1:ten_alpha:vpc:prod".to_string(),
        vpc: vpc_create(),
        actor: "sp_network".to_string(),
        idempotency_key: "idem-network-vpc-create".to_string(),
        requested_at_epoch_seconds: 1_700_000_010,
    }
}

fn provider_load_balancer_create_request() -> NetworkProviderLoadBalancerCreateRequest {
    NetworkProviderLoadBalancerCreateRequest {
        request_id: "networkprov_req_lb_create_001".to_string(),
        provider_load_balancer_ref: "oci-lb://ocid1.compartment.oc1..cloud/ap-chuncheon-1/oyatie:cloud:region-alpha1:ten_alpha:lb-v7:frontdoor".to_string(),
        vpc: vpc_create(),
        subnets: vec![subnet_create()],
        load_balancer: lb_create(),
        actor: "sp_network".to_string(),
        idempotency_key: "idem-network-lb-create".to_string(),
        requested_at_epoch_seconds: 1_700_000_030,
    }
}

fn provider_dns_zone_create_request() -> NetworkProviderDnsZoneCreateRequest {
    NetworkProviderDnsZoneCreateRequest {
        request_id: "networkprov_req_dns_create_001".to_string(),
        provider_dns_zone_ref: "oci-dns-zone://ocid1.compartment.oc1..cloud/ap-chuncheon-1/oyatie:cloud:region-alpha1:ten_alpha:dns-zone:example-com".to_string(),
        vpc: None,
        dns_zone: public_dns_create(),
        actor: "sp_network".to_string(),
        idempotency_key: "idem-network-dns-zone-create".to_string(),
        requested_at_epoch_seconds: 1_700_000_040,
    }
}

#[test]
fn network_provider_vpc_requests_validate_refs_shape_and_actor() {
    provider_vpc_create_request()
        .validate()
        .expect("provider VPC request is valid");

    let mut bad_provider_ref = provider_vpc_create_request();
    bad_provider_ref.provider_vcn_ref = " ".to_string();
    assert_eq!(
        bad_provider_ref.validate(),
        Err(NetworkProviderVpcError::InvalidProviderVcnRef)
    );

    let mut bad_vpc_shape = provider_vpc_create_request();
    bad_vpc_shape.vpc.flow_logs_enabled = false;
    assert_eq!(
        bad_vpc_shape.validate(),
        Err(NetworkProviderVpcError::InvalidRequestShape(
            CloudNetworkError::FlowLogsRequired,
        ))
    );

    let mut bad_actor = provider_vpc_create_request();
    bad_actor.actor = "network".to_string();
    assert_eq!(
        bad_actor.validate(),
        Err(NetworkProviderVpcError::InvalidActorRef)
    );
}

#[test]
fn network_provider_vpc_receipts_keep_refs_without_provider_credentials() {
    let receipt = NetworkProviderVpcReceipt::create_vpc(
        NetworkProviderKind::OciVcn,
        provider_vpc_create_request(),
        "oci-vcn-1700000000-networkprov_req_vpc_create_001",
        "oci-vcn://ocid1.compartment.oc1..cloud/ap-chuncheon-1/oyatie:cloud:region-alpha1:ten_alpha:vpc:prod/networkprov_req_vpc_create_001",
    )
    .expect("VPC receipt keeps provider references only");

    assert_eq!(receipt.provider.label(), "oci_vcn");
    assert_eq!(receipt.operation.label(), "create_vpc");
    assert_eq!(
        receipt.resource_id,
        "oyatie:cloud:region-alpha1:ten_alpha:vpc:prod"
    );
    assert_eq!(receipt.cidr_v4, "10.42.0.0/16");
    assert_eq!(receipt.cidr_v6, "2001:db8:42::/56");
    assert!(receipt.flow_logs_enabled);
    assert_eq!(receipt.actor, "sp_network");
    assert_eq!(receipt.schema_version, NETWORK_SCHEMA_VERSION);
}

#[test]
fn network_provider_load_balancer_requests_validate_context_shape_and_actor() {
    provider_load_balancer_create_request()
        .validate()
        .expect("provider load balancer request is valid");

    let mut bad_provider_ref = provider_load_balancer_create_request();
    bad_provider_ref.provider_load_balancer_ref = " ".to_string();
    assert_eq!(
        bad_provider_ref.validate(),
        Err(NetworkProviderLoadBalancerError::InvalidProviderLoadBalancerRef)
    );

    let mut bad_lb_shape = provider_load_balancer_create_request();
    bad_lb_shape.load_balancer.mtls = None;
    assert_eq!(
        bad_lb_shape.validate(),
        Err(NetworkProviderLoadBalancerError::InvalidRequestShape(
            CloudNetworkError::GrpcRequiresMtls,
        ))
    );

    let mut bad_actor = provider_load_balancer_create_request();
    bad_actor.actor = "network".to_string();
    assert_eq!(
        bad_actor.validate(),
        Err(NetworkProviderLoadBalancerError::InvalidActorRef)
    );
}

#[test]
fn network_provider_load_balancer_receipts_keep_refs_without_provider_credentials() {
    let receipt = NetworkProviderLoadBalancerReceipt::create_load_balancer(
        NetworkProviderKind::OciLoadBalancer,
        provider_load_balancer_create_request(),
        "oci-lb-1700000000-networkprov_req_lb_create_001",
        "oci-lb://ocid1.compartment.oc1..cloud/ap-chuncheon-1/oyatie:cloud:region-alpha1:ten_alpha:lb-v7:frontdoor/networkprov_req_lb_create_001",
    )
    .expect("load balancer receipt keeps provider references only");

    assert_eq!(receipt.provider.label(), "oci_load_balancer");
    assert_eq!(receipt.operation.label(), "create_load_balancer");
    assert_eq!(
        receipt.resource_id,
        "oyatie:cloud:region-alpha1:ten_alpha:lb-v7:frontdoor"
    );
    assert_eq!(
        receipt.vpc_id,
        "oyatie:cloud:region-alpha1:ten_alpha:vpc:prod"
    );
    assert_eq!(receipt.kind, LbKind::L7Grpc);
    assert_eq!(receipt.listener_count, 1);
    assert_eq!(receipt.target_group_count, 1);
    assert!(receipt.mtls_enabled);
    assert_eq!(receipt.actor, "sp_network");
    assert_eq!(receipt.schema_version, NETWORK_SCHEMA_VERSION);
}

#[test]
fn network_provider_dns_zone_requests_validate_context_shape_and_actor() {
    provider_dns_zone_create_request()
        .validate()
        .expect("provider DNS zone request is valid");

    let mut bad_provider_ref = provider_dns_zone_create_request();
    bad_provider_ref.provider_dns_zone_ref = " ".to_string();
    assert_eq!(
        bad_provider_ref.validate(),
        Err(NetworkProviderDnsZoneError::InvalidProviderDnsZoneRef)
    );

    let mut bad_zone_shape = provider_dns_zone_create_request();
    bad_zone_shape.dns_zone.dnssec_key_ref = None;
    assert_eq!(
        bad_zone_shape.validate(),
        Err(NetworkProviderDnsZoneError::InvalidRequestShape(
            CloudNetworkError::DnssecRequired,
        ))
    );

    let mut bad_actor = provider_dns_zone_create_request();
    bad_actor.actor = "network".to_string();
    assert_eq!(
        bad_actor.validate(),
        Err(NetworkProviderDnsZoneError::InvalidActorRef)
    );
}

#[test]
fn network_provider_dns_zone_receipts_keep_refs_without_provider_credentials() {
    let receipt = NetworkProviderDnsZoneReceipt::create_dns_zone(
        NetworkProviderKind::OciDnsZone,
        provider_dns_zone_create_request(),
        "oci-dns-zone-1700000000-networkprov_req_dns_create_001",
        "oci-dns-zone://ocid1.compartment.oc1..cloud/ap-chuncheon-1/oyatie:cloud:region-alpha1:ten_alpha:dns-zone:example-com/networkprov_req_dns_create_001",
    )
    .expect("DNS zone receipt keeps provider references only");

    assert_eq!(receipt.provider.label(), "oci_dns_zone");
    assert_eq!(receipt.operation.label(), "create_dns_zone");
    assert_eq!(
        receipt.resource_id,
        "oyatie:cloud:region-alpha1:ten_alpha:dns-zone:example-com"
    );
    assert_eq!(receipt.name, "example.com");
    assert_eq!(receipt.kind, DnsZoneKind::Public);
    assert_eq!(receipt.vpc_id, None);
    assert!(receipt.dnssec_enabled);
    assert_eq!(receipt.actor, "sp_network");
    assert_eq!(receipt.schema_version, NETWORK_SCHEMA_VERSION);
}

#[test]
fn network_provider_direct_interconnect_requests_validate_context_shape_and_actor() {
    provider_direct_interconnect_create_request()
        .validate()
        .expect("provider direct interconnect request is valid");

    let mut bad_provider_ref = provider_direct_interconnect_create_request();
    bad_provider_ref.provider_virtual_circuit_ref = " ".to_string();
    assert_eq!(
        bad_provider_ref.validate(),
        Err(NetworkProviderDirectInterconnectError::InvalidProviderVirtualCircuitRef)
    );

    let mut bad_shape = provider_direct_interconnect_create_request();
    bad_shape.direct_interconnect.redundant_port_count = 1;
    assert_eq!(
        bad_shape.validate(),
        Err(NetworkProviderDirectInterconnectError::InvalidRequestShape(
            CloudNetworkError::InterconnectRedundancyRequired,
        ))
    );

    let mut bad_actor = provider_direct_interconnect_create_request();
    bad_actor.actor = "network".to_string();
    assert_eq!(
        bad_actor.validate(),
        Err(NetworkProviderDirectInterconnectError::InvalidActorRef)
    );
}

#[test]
fn network_provider_direct_interconnect_receipts_keep_refs_without_provider_credentials() {
    let receipt = NetworkProviderDirectInterconnectReceipt::create_direct_interconnect(
        NetworkProviderKind::OciFastConnect,
        provider_direct_interconnect_create_request(),
        "oci-fast-connect-1700000000-networkprov_req_interconnect_create_001",
        "oci-fast-connect://ocid1.compartment.oc1..cloud/ap-chuncheon-1/oyatie:cloud:region-alpha1:ten_alpha:direct-interconnect:fabric-a-primary/networkprov_req_interconnect_create_001",
    )
    .expect("direct interconnect receipt keeps provider references only");

    assert_eq!(receipt.provider.label(), "oci_fast_connect");
    assert_eq!(receipt.operation.label(), "create_direct_interconnect");
    assert_eq!(
        receipt.resource_id,
        "oyatie:cloud:region-alpha1:ten_alpha:direct-interconnect:fabric-a-primary"
    );
    assert_eq!(receipt.partner_id, "ixp_alpha");
    assert_eq!(receipt.peering_location, "region-alpha1-fabric-a");
    assert_eq!(receipt.physical_port_id, "icp_alpha_001");
    assert_eq!(receipt.vlan_tag, 101);
    assert_eq!(receipt.bandwidth_mbps, 10_000);
    assert_eq!(receipt.redundant_port_count, 2);
    assert_eq!(receipt.bgp_session_count, 2);
    assert_eq!(receipt.advertised_prefix_count, 2);
    assert_eq!(receipt.actor, "sp_network");
    assert_eq!(receipt.schema_version, NETWORK_SCHEMA_VERSION);
}
