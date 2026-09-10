use data_boundary_kernel::DataClass;
use network_residency::ResidencyClass;

use super::*;
mod cdn;
mod cidr;
mod create_contract;
mod ddos;
mod dns;
mod flow_anomaly;
mod interconnect;
mod load_balancer;
mod mesh;
mod provider;
mod route_table;
mod security_group;
mod shadowed_rules;
mod vpc;

use network_residency::{
    PerPackResidency, PerPackResidencyCreate, RegulatorOverlay, RegulatorOverlayCreate,
};

fn residency_class() -> ResidencyClass {
    ResidencyClass::PerPack(Box::new(
        PerPackResidency::new(PerPackResidencyCreate {
            allowed_primary_regions: vec!["region-alpha1".to_string()],
            allowed_replica_regions: vec!["region-beta1".to_string()],
            forbidden_regions: vec!["region-gamma1".to_string()],
            regulator_overlay: RegulatorOverlay::new(RegulatorOverlayCreate {
                regulator_refs: vec!["regulator/cloud-network".to_string()],
                evidence_ref: "evidence/residency/cloud-network".to_string(),
            })
            .expect("regulator overlay fixture is valid"),
        })
        .expect("per-pack residency fixture is valid"),
    ))
}

fn route_table_create() -> RouteTableCreate {
    RouteTableCreate {
        id: "rtb_main".to_string(),
        routes: vec![
            RouteCreate {
                destination: "10.42.0.0/16".to_string(),
                next_hop: RouteNextHopKind::Local,
                target_ref: None,
            },
            RouteCreate {
                destination: "2001:db8:42::/56".to_string(),
                next_hop: RouteNextHopKind::Local,
                target_ref: None,
            },
        ],
    }
}

fn security_group_create() -> SecurityGroupCreate {
    SecurityGroupCreate {
        id: "sg_web".to_string(),
        rules: vec![SecurityRule {
            direction: RuleDirection::Ingress,
            protocol: IpProtocol::Tcp,
            port_range: Some((443, 443)),
            cidr: RouteDestination::Ipv4(Ipv4Cidr::new("10.42.0.0/16").unwrap()),
            description: "tenant https ingress".to_string(),
        }],
    }
}

fn vpc_create() -> VpcCreate {
    VpcCreate {
        resource_id: "oyatie:cloud:region-alpha1:ten_alpha:vpc:prod".to_string(),
        tenant_id: "ten_alpha".to_string(),
        region: "region-alpha1".to_string(),
        cidr_v4: "10.42.0.0/16".to_string(),
        cidr_v6: "2001:db8:42::/56".to_string(),
        flow_logs_enabled: true,
        route_table: route_table_create(),
        security_groups: vec![security_group_create()],
        residency: residency_class(),
        state: VpcState::Creating,
        data_class: DataClass::Public,
        created_at_epoch_seconds: 1_700_000_000,
    }
}

fn subnet_create() -> SubnetCreate {
    SubnetCreate {
        resource_id: "oyatie:cloud:region-alpha1:ten_alpha:subnet:prod-a".to_string(),
        tenant_id: "ten_alpha".to_string(),
        vpc_id: "oyatie:cloud:region-alpha1:ten_alpha:vpc:prod".to_string(),
        region: "region-alpha1".to_string(),
        az: "region-alpha1-a".to_string(),
        cidr_v4: "10.42.1.0/24".to_string(),
        cidr_v6: "2001:db8:42:1::/64".to_string(),
        public_ip_on_launch: false,
        state: SubnetState::Creating,
        data_class: DataClass::Public,
        created_at_epoch_seconds: 1_700_000_010,
    }
}

fn lb_create() -> LoadBalancerCreate {
    LoadBalancerCreate {
        resource_id: "oyatie:cloud:region-alpha1:ten_alpha:lb-v7:frontdoor".to_string(),
        tenant_id: "ten_alpha".to_string(),
        vpc_id: "oyatie:cloud:region-alpha1:ten_alpha:vpc:prod".to_string(),
        region: "region-alpha1".to_string(),
        kind: LbKind::L7Grpc,
        listeners: vec![ListenerCreate {
            port: 443,
            target_group_id: "tg_api".to_string(),
            tls_certificate: Some("cert/region-alpha1/ten_alpha/frontdoor".to_string()),
        }],
        target_groups: vec![TargetGroupCreate {
            id: "tg_api".to_string(),
            subnet_ids: vec!["oyatie:cloud:region-alpha1:ten_alpha:subnet:prod-a".to_string()],
            health_check_path: Some("/healthz".to_string()),
        }],
        mtls: Some(MtlsConfigCreate {
            ca_bundle_ref: "cert/region-alpha1/ten_alpha/mesh-ca".to_string(),
            client_policy: MtlsClientPolicy::RequireVerifiedClientCert,
        }),
        waf_policy: Some("waf_cloud_frontdoor".to_string()),
        state: LbState::Creating,
        data_class: DataClass::Public,
        created_at_epoch_seconds: 1_700_000_020,
    }
}

fn public_dns_create() -> DnsZoneCreate {
    DnsZoneCreate {
        resource_id: "oyatie:cloud:region-alpha1:ten_alpha:dns-zone:example-com".to_string(),
        tenant_id: "ten_alpha".to_string(),
        region: "region-alpha1".to_string(),
        name: "example.com".to_string(),
        kind: DnsZoneKind::Public,
        vpc_id: None,
        dnssec_key_ref: Some("dnssec/region-alpha1/ten_alpha/example-com".to_string()),
        state: DnsZoneState::Creating,
        data_class: DataClass::Public,
        created_at_epoch_seconds: 1_700_000_030,
    }
}

fn seeded_catalog() -> CloudNetworkCatalog {
    let mut catalog = CloudNetworkCatalog::default();
    catalog.create_vpc(vpc_create()).expect("vpc create");
    catalog.add_subnet(subnet_create()).expect("subnet create");
    catalog
        .create_load_balancer(lb_create())
        .expect("load balancer create");
    catalog
        .create_dns_zone(public_dns_create())
        .expect("public dns create");
    catalog
}

fn cdn_create() -> CdnDistributionCreate {
    CdnDistributionCreate {
        resource_id: "oyatie:cloud:region-alpha1:ten_alpha:cdn-distribution:console".to_string(),
        tenant_id: "ten_alpha".to_string(),
        region: "region-alpha1".to_string(),
        hostnames: vec!["console.oyatie.example".to_string()],
        origins: vec![CdnOriginCreate {
            resource_id: "oyatie:cloud:region-alpha1:ten_alpha:lb-v7:frontdoor".to_string(),
            kind: CdnOriginKind::LoadBalancer,
        }],
        tls_certificate: "cert/region-alpha1/ten_alpha/console-edge".to_string(),
        waf_policy: "waf_console_edge".to_string(),
        cache_mode: CdnCacheMode::ConsoleAssets,
        state: CdnState::Creating,
        data_class: DataClass::Public,
        created_at_epoch_seconds: 1_700_000_050,
    }
}

fn interconnect_partner_create(id: &str, location: &str) -> InterconnectPartnerCreate {
    InterconnectPartnerCreate {
        id: id.to_string(),
        name: id.trim_start_matches("ixp_").to_ascii_uppercase(),
        region: "region-alpha1".to_string(),
        peering_locations: vec![location.to_string()],
        per_link_sla_basis_points: 9_999,
    }
}

fn bgp_session_create(id: &str, local: &str, peer: &str) -> BgpSessionCreate {
    BgpSessionCreate {
        id: id.to_string(),
        local_asn: 64_512,
        peer_asn: 64_520,
        local_address: local.to_string(),
        peer_address: peer.to_string(),
    }
}

fn direct_interconnect_create() -> DirectInterconnectCreate {
    DirectInterconnectCreate {
        resource_id: "oyatie:cloud:region-alpha1:ten_alpha:direct-interconnect:fabric-a-primary"
            .to_string(),
        tenant_id: "ten_alpha".to_string(),
        region: "region-alpha1".to_string(),
        partner_id: "ixp_alpha".to_string(),
        peering_location: "region-alpha1-fabric-a".to_string(),
        physical_port_id: "icp_alpha_001".to_string(),
        vlan_tag: 101,
        bandwidth_mbps: 10_000,
        redundant_port_count: 2,
        bgp_sessions: vec![
            bgp_session_create("bgp_alpha_1", "169.254.10.1", "169.254.10.2"),
            bgp_session_create("bgp_alpha_2", "169.254.10.5", "169.254.10.6"),
        ],
        advertised_prefixes: vec!["10.42.0.0/16".to_string(), "2001:db8:42::/56".to_string()],
        per_link_sla_basis_points: 9_999,
        state: DirectInterconnectState::Creating,
        data_class: DataClass::Public,
        created_at_epoch_seconds: 1_700_000_060,
    }
}

fn interconnect_partner_creates() -> Vec<InterconnectPartnerCreate> {
    vec![
        interconnect_partner_create("ixp_alpha", "region-alpha1-fabric-a"),
        interconnect_partner_create("ixp_beta", "region-alpha1-fabric-b"),
    ]
}

// ── ST2: SecurityGroup::evaluate ─────────────────────────────────────────

fn ingress_tcp_rule(cidr: &str, port_range: Option<(u16, u16)>) -> SecurityRule {
    SecurityRule {
        direction: RuleDirection::Ingress,
        protocol: IpProtocol::Tcp,
        port_range,
        cidr: RouteDestination::Ipv4(Ipv4Cidr::new(cidr).unwrap()),
        description: "test ingress tcp rule".to_string(),
    }
}

fn egress_tcp_rule(cidr: &str, port_range: Option<(u16, u16)>) -> SecurityRule {
    SecurityRule {
        direction: RuleDirection::Egress,
        protocol: IpProtocol::Tcp,
        port_range,
        cidr: RouteDestination::Ipv4(Ipv4Cidr::new(cidr).unwrap()),
        description: "test egress tcp rule".to_string(),
    }
}

fn ingress_any_rule(cidr: &str) -> SecurityRule {
    SecurityRule {
        direction: RuleDirection::Ingress,
        protocol: IpProtocol::Any,
        port_range: None,
        cidr: RouteDestination::Ipv4(Ipv4Cidr::new(cidr).unwrap()),
        description: "test ingress any rule".to_string(),
    }
}

fn sg_with_rules(rules: Vec<SecurityRule>) -> SecurityGroup {
    SecurityGroup {
        id: SecurityGroupId {
            value: "sg_test".to_string(),
        },
        rules,
    }
}
