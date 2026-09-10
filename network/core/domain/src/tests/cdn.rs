use super::*;
use data_boundary_kernel::DataClass;

#[test]
fn creates_cdn_distribution_with_tls_waf_and_known_origin() {
    let mut catalog = seeded_catalog();
    let distribution = catalog
        .create_cdn_distribution(cdn_create())
        .expect("cdn distribution is valid");

    assert_eq!(
        distribution.resource_id.value.kind_label().unwrap(),
        "cdn-distribution"
    );
    assert_eq!(
        distribution.hostnames.value[0].value,
        "console.oyatie.example"
    );
    assert_eq!(
        distribution.origins.value[0].kind,
        CdnOriginKind::LoadBalancer
    );
    assert_eq!(
        distribution.origins.data_class.compatibility_data_class(),
        DataClass::InternalOnly
    );

    let duplicate_hostname = catalog
        .create_cdn_distribution(CdnDistributionCreate {
            resource_id: "oyatie:cloud:region-alpha1:ten_alpha:cdn-distribution:dup-host"
                .to_string(),
            hostnames: vec![
                "console.oyatie.example".to_string(),
                "Console.Oyatie.Example.".to_string(),
            ],
            ..cdn_create()
        })
        .expect_err("hostnames must be unique after normalization");
    assert_eq!(duplicate_hostname, CloudNetworkError::DuplicateCdnHostname);
}

#[test]
fn rejects_cdn_without_waf_tls_known_origin_or_create_state() {
    let mut catalog = seeded_catalog();

    let waf_error = catalog
        .create_cdn_distribution(CdnDistributionCreate {
            waf_policy: String::new(),
            ..cdn_create()
        })
        .expect_err("cdn must bind a WAF policy");
    assert_eq!(waf_error, CloudNetworkError::CdnWafRequired);

    let tls_error = catalog
        .create_cdn_distribution(CdnDistributionCreate {
            tls_certificate: String::new(),
            ..cdn_create()
        })
        .expect_err("cdn must bind an edge certificate");
    assert_eq!(tls_error, CloudNetworkError::CdnTlsRequired);

    let origin_error = catalog
        .create_cdn_distribution(CdnDistributionCreate {
            origins: vec![CdnOriginCreate {
                resource_id: "oyatie:cloud:region-alpha1:ten_alpha:lb-v7:missing".to_string(),
                kind: CdnOriginKind::LoadBalancer,
            }],
            ..cdn_create()
        })
        .expect_err("cdn origins must be known network resources");
    assert_eq!(origin_error, CloudNetworkError::UnknownLoadBalancer);

    let state_error = catalog
        .create_cdn_distribution(CdnDistributionCreate {
            state: CdnState::Active,
            ..cdn_create()
        })
        .expect_err("cdn create begins in Creating");
    assert_eq!(state_error, CloudNetworkError::InvalidCdnState);
}
