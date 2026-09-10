use super::*;
use data_boundary_kernel::DataClass;
use std::collections::BTreeMap;

#[test]
fn create_contracts_reject_caller_forged_runtime_state() {
    let vpc_error = Vpc::new(VpcCreate {
        state: VpcState::Active,
        ..vpc_create()
    })
    .expect_err("vpc create begins in Creating");
    assert_eq!(vpc_error, CloudNetworkError::InvalidVpcState);

    let vpc = Vpc::new(vpc_create()).expect("vpc fixture is valid");
    let subnet_error = Subnet::new(
        &vpc,
        SubnetCreate {
            state: SubnetState::Active,
            ..subnet_create()
        },
    )
    .expect_err("subnet create begins in Creating");
    assert_eq!(subnet_error, CloudNetworkError::InvalidSubnetState);

    let lb_error = LoadBalancer::new(
        &vpc,
        &BTreeMap::new(),
        LoadBalancerCreate {
            state: LbState::Active,
            ..lb_create()
        },
    )
    .expect_err("load-balancer create begins in Creating");
    assert_eq!(lb_error, CloudNetworkError::InvalidLbState);

    let zone_error = DnsZone::new(
        None,
        DnsZoneCreate {
            resource_id: "oyatie:cloud:region-alpha1:ten_alpha:dns-zone:example-com".to_string(),
            tenant_id: "ten_alpha".to_string(),
            region: "region-alpha1".to_string(),
            name: "example.com".to_string(),
            kind: DnsZoneKind::Public,
            vpc_id: None,
            dnssec_key_ref: Some("dnssec/region-alpha1/ten_alpha/example-com".to_string()),
            state: DnsZoneState::Active,
            data_class: DataClass::Public,
            created_at_epoch_seconds: 1_700_000_030,
        },
    )
    .expect_err("dns-zone create begins in Creating");
    assert_eq!(zone_error, CloudNetworkError::InvalidDnsZoneState);
}
