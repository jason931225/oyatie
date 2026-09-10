use super::*;
use data_boundary_kernel::DataClass;

#[test]
fn creates_dns_zones_with_dnssec_and_private_zone_vpc_binding() {
    let mut catalog = CloudNetworkCatalog::default();
    catalog.create_vpc(vpc_create()).expect("vpc create");
    let public = catalog
        .create_dns_zone(DnsZoneCreate {
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
        })
        .expect("public zone is valid");
    assert_eq!(public.name.value.value, "example.com");

    let private = catalog
        .create_dns_zone(DnsZoneCreate {
            resource_id: "oyatie:cloud:region-alpha1:ten_alpha:dns-zone:internal-example"
                .to_string(),
            tenant_id: "ten_alpha".to_string(),
            region: "region-alpha1".to_string(),
            name: "internal.example".to_string(),
            kind: DnsZoneKind::Private,
            vpc_id: Some("oyatie:cloud:region-alpha1:ten_alpha:vpc:prod".to_string()),
            dnssec_key_ref: None,
            state: DnsZoneState::Creating,
            data_class: DataClass::Public,
            created_at_epoch_seconds: 1_700_000_031,
        })
        .expect("private zone binds known vpc");
    assert!(private.vpc_id.value.is_some());
}

#[test]
fn rejects_public_dns_without_dnssec_and_private_dns_without_vpc() {
    let dnssec_error = DnsZone::new(
        None,
        DnsZoneCreate {
            resource_id: "oyatie:cloud:region-alpha1:ten_alpha:dns-zone:example-com".to_string(),
            tenant_id: "ten_alpha".to_string(),
            region: "region-alpha1".to_string(),
            name: "example.com".to_string(),
            kind: DnsZoneKind::Public,
            vpc_id: None,
            dnssec_key_ref: None,
            state: DnsZoneState::Creating,
            data_class: DataClass::Public,
            created_at_epoch_seconds: 1_700_000_030,
        },
    )
    .expect_err("public zones require DNSSEC");
    assert_eq!(dnssec_error, CloudNetworkError::DnssecRequired);

    let private_error = DnsZone::new(
        None,
        DnsZoneCreate {
            resource_id: "oyatie:cloud:region-alpha1:ten_alpha:dns-zone:internal".to_string(),
            tenant_id: "ten_alpha".to_string(),
            region: "region-alpha1".to_string(),
            name: "internal.example".to_string(),
            kind: DnsZoneKind::Private,
            vpc_id: None,
            dnssec_key_ref: None,
            state: DnsZoneState::Creating,
            data_class: DataClass::Public,
            created_at_epoch_seconds: 1_700_000_031,
        },
    )
    .expect_err("private zones require VPC binding");
    assert_eq!(private_error, CloudNetworkError::PrivateZoneRequiresVpc);
}
