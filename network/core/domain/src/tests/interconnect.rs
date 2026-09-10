use super::*;

#[test]
fn creates_direct_interconnect_with_multi_ixp_bgp_and_sla() {
    let mut catalog = CloudNetworkCatalog::default();
    catalog
        .add_interconnect_partner(interconnect_partner_create(
            "ixp_alpha",
            "region-alpha1-fabric-a",
        ))
        .expect("primary interconnect partner");
    let diversity_error = catalog
        .create_direct_interconnect(direct_interconnect_create())
        .expect_err("major regions require at least two interconnect partners");
    assert_eq!(
        diversity_error,
        CloudNetworkError::RegionalInterconnectDiversityRequired
    );

    catalog
        .add_interconnect_partner(interconnect_partner_create(
            "ixp_beta",
            "region-alpha1-fabric-b",
        ))
        .expect("second interconnect partner");
    let interconnect = catalog
        .create_direct_interconnect(direct_interconnect_create())
        .expect("direct interconnect is valid");

    assert_eq!(
        interconnect.resource_id.value.kind_label().unwrap(),
        "direct-interconnect"
    );
    assert_eq!(interconnect.bgp_sessions.value.len(), 2);
    assert_eq!(interconnect.per_link_sla_basis_points.value, 9_999);
}

#[test]
fn rejects_direct_interconnect_without_redundant_bgp_vlan_or_sla() {
    let mut catalog = CloudNetworkCatalog::default();
    catalog
        .add_interconnect_partner(interconnect_partner_create(
            "ixp_alpha",
            "region-alpha1-fabric-a",
        ))
        .expect("primary interconnect partner");
    catalog
        .add_interconnect_partner(interconnect_partner_create(
            "ixp_beta",
            "region-alpha1-fabric-b",
        ))
        .expect("second interconnect partner");

    let bgp_error = catalog
        .create_direct_interconnect(DirectInterconnectCreate {
            bgp_sessions: vec![bgp_session_create(
                "bgp_alpha_1",
                "169.254.10.1",
                "169.254.10.2",
            )],
            ..direct_interconnect_create()
        })
        .expect_err("interconnect needs redundant BGP sessions");
    assert_eq!(bgp_error, CloudNetworkError::InterconnectRedundancyRequired);

    let vlan_error = catalog
        .create_direct_interconnect(DirectInterconnectCreate {
            vlan_tag: 0,
            ..direct_interconnect_create()
        })
        .expect_err("802.1Q VLAN tag must be in range");
    assert_eq!(vlan_error, CloudNetworkError::InvalidVlanTag);

    let sla_error = catalog
        .create_direct_interconnect(DirectInterconnectCreate {
            per_link_sla_basis_points: 9_990,
            ..direct_interconnect_create()
        })
        .expect_err("direct interconnect requires 99.99% per-link SLA");
    assert_eq!(sla_error, CloudNetworkError::InterconnectSlaRequired);
}
