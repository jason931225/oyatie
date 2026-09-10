use super::*;
use data_boundary_kernel::DataClass;

#[test]
fn creates_vpc_with_ipv6_flow_logs_route_table_and_security_groups() {
    let vpc = Vpc::new(vpc_create()).expect("vpc is valid");

    assert_eq!(vpc.resource_id.value.kind_label().unwrap(), "vpc");
    assert_eq!(vpc.cidr_v4.value.value, "10.42.0.0/16");
    assert_eq!(vpc.cidr_v6.value.value, "2001:db8:42::/56");
    assert!(vpc.flow_logs_enabled.value);
    assert_eq!(vpc.route_table.value.routes.len(), 2);
    assert_eq!(vpc.security_groups.value.len(), 1);
    assert_eq!(
        vpc.route_table.data_class.compatibility_data_class(),
        DataClass::InternalOnly
    );
    assert_eq!(
        vpc.security_groups.data_class.compatibility_data_class(),
        DataClass::InternalOnly
    );
    assert_eq!(vpc.schema_version.value, NETWORK_SCHEMA_VERSION);
}

#[test]
fn rejects_vpc_without_ipv6_flow_logs_or_public_metadata_class() {
    let ipv6_error = Vpc::new(VpcCreate {
        cidr_v6: "not-a-cidr".to_string(),
        ..vpc_create()
    })
    .expect_err("IPv6 is required from day one");
    assert_eq!(ipv6_error, CloudNetworkError::InvalidIpv6Cidr);

    let flow_error = Vpc::new(VpcCreate {
        flow_logs_enabled: false,
        ..vpc_create()
    })
    .expect_err("audit-grade flow logs are mandatory");
    assert_eq!(flow_error, CloudNetworkError::FlowLogsRequired);

    let class_error = Vpc::new(VpcCreate {
        data_class: DataClass::InternalOnly,
        ..vpc_create()
    })
    .expect_err("network metadata is public-only");
    assert_eq!(class_error, CloudNetworkError::InvalidDataClass);
}

#[test]
fn creates_subnet_only_inside_parent_vpc_and_region_az() {
    let vpc = Vpc::new(vpc_create()).expect("vpc is valid");
    let subnet = Subnet::new(&vpc, subnet_create()).expect("subnet is valid");

    assert_eq!(subnet.resource_id.value.kind_label().unwrap(), "subnet");
    assert_eq!(subnet.az.value.value, "region-alpha1-a");
    assert_eq!(subnet.cidr_v4.value.value, "10.42.1.0/24");

    let outside = Subnet::new(
        &vpc,
        SubnetCreate {
            cidr_v4: "10.99.1.0/24".to_string(),
            ..subnet_create()
        },
    )
    .expect_err("subnet CIDR must be inside VPC CIDR");
    assert_eq!(outside, CloudNetworkError::SubnetOutsideVpc);

    let az_error = Subnet::new(
        &vpc,
        SubnetCreate {
            az: "region-gamma1-a".to_string(),
            ..subnet_create()
        },
    )
    .expect_err("subnet AZ must belong to region");
    assert_eq!(az_error, CloudNetworkError::AzRegionMismatch);
}

#[test]
fn catalog_rejects_overlapping_subnets_per_vpc() {
    let mut catalog = CloudNetworkCatalog::default();
    catalog.create_vpc(vpc_create()).expect("vpc create");
    catalog.add_subnet(subnet_create()).expect("first subnet");

    let overlap = catalog
        .add_subnet(SubnetCreate {
            resource_id: "oyatie:cloud:region-alpha1:ten_alpha:subnet:prod-a-overlap".to_string(),
            cidr_v4: "10.42.1.128/25".to_string(),
            cidr_v6: "2001:db8:42:1::/65".to_string(),
            ..subnet_create()
        })
        .expect_err("same VPC CIDR ranges must not overlap");
    assert_eq!(overlap, CloudNetworkError::OverlappingSubnet);

    let adjacent = catalog
        .add_subnet(SubnetCreate {
            resource_id: "oyatie:cloud:region-alpha1:ten_alpha:subnet:prod-b".to_string(),
            az: "region-alpha1-b".to_string(),
            cidr_v4: "10.42.2.0/24".to_string(),
            cidr_v6: "2001:db8:42:2::/64".to_string(),
            ..subnet_create()
        })
        .expect("non-overlapping same-VPC subnet is valid");
    assert_eq!(adjacent.cidr_v4.value.value, "10.42.2.0/24");
}
