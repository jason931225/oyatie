use super::*;
use data_boundary_kernel::DataClass;

fn ddos_create() -> DdosProtectionCreate {
    DdosProtectionCreate {
        resource_id: "oyatie:cloud:region-alpha1:ten_alpha:ddos-protection:frontdoor".to_string(),
        tenant_id: "ten_alpha".to_string(),
        region: "region-alpha1".to_string(),
        protected_resource_ids: vec![
            "oyatie:cloud:region-alpha1:ten_alpha:lb-v7:frontdoor".to_string(),
            "oyatie:cloud:region-alpha1:ten_alpha:cdn-distribution:console".to_string(),
        ],
        scrubbing_regions: vec!["region-alpha1".to_string(), "region-beta1".to_string()],
        line_rate_scrubbing: true,
        always_on: true,
        mitigation_runbook_ref: "runbook/network/ddos/frontdoor".to_string(),
        oncall_group_ref: "oncall/network-sre".to_string(),
        state: DdosProtectionState::Creating,
        data_class: DataClass::Public,
        created_at_epoch_seconds: 1_700_000_070,
    }
}

#[test]
fn creates_ddos_protection_for_known_edge_resources() {
    let mut catalog = seeded_catalog();
    catalog
        .create_cdn_distribution(cdn_create())
        .expect("cdn distribution");
    let protection = catalog
        .create_ddos_protection(ddos_create())
        .expect("ddos protection is valid");

    assert_eq!(
        protection.resource_id.value.kind_label().unwrap(),
        "ddos-protection"
    );
    assert_eq!(protection.protected_resources.value.len(), 2);
    assert!(protection.line_rate_scrubbing.value);
    assert!(protection.always_on.value);
}

#[test]
fn rejects_ddos_without_known_resources_scrubbing_or_always_on_posture() {
    let mut catalog = seeded_catalog();
    catalog
        .create_cdn_distribution(cdn_create())
        .expect("cdn distribution");

    let resource_error = catalog
        .create_ddos_protection(DdosProtectionCreate {
            protected_resource_ids: vec![
                "oyatie:cloud:region-alpha1:ten_alpha:cdn-distribution:missing".to_string(),
            ],
            ..ddos_create()
        })
        .expect_err("ddos binds only known protected resources");
    assert_eq!(resource_error, CloudNetworkError::UnknownProtectedResource);

    let always_on_error = catalog
        .create_ddos_protection(DdosProtectionCreate {
            always_on: false,
            ..ddos_create()
        })
        .expect_err("ddos protection must be always-on");
    assert_eq!(always_on_error, CloudNetworkError::DdosAlwaysOnRequired);

    let scrubbing_error = catalog
        .create_ddos_protection(DdosProtectionCreate {
            scrubbing_regions: vec!["region-beta1".to_string()],
            ..ddos_create()
        })
        .expect_err("home region must be in scrubbing set");
    assert_eq!(scrubbing_error, CloudNetworkError::ScrubbingRegionRequired);
}
