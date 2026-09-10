use super::*;
use data_boundary_kernel::DataClass;

#[test]
fn creates_l7_grpc_load_balancer_with_tls_mtls_and_known_target_group() {
    let mut catalog = CloudNetworkCatalog::default();
    catalog.create_vpc(vpc_create()).expect("vpc create");
    catalog.add_subnet(subnet_create()).expect("subnet create");
    let lb = catalog
        .create_load_balancer(lb_create())
        .expect("load balancer is valid");

    assert_eq!(lb.resource_id.value.kind_label().unwrap(), "lb-v7");
    assert_eq!(lb.listeners.value[0].port, 443);
    assert!(lb.mtls.value.is_some());
    assert_eq!(lb.target_groups.value[0].subnet_ids.len(), 1);
    assert_eq!(
        lb.listeners.data_class.compatibility_data_class(),
        DataClass::InternalOnly
    );
    assert_eq!(
        lb.target_groups.data_class.compatibility_data_class(),
        DataClass::InternalOnly
    );
    assert_eq!(
        lb.mtls.data_class.compatibility_data_class(),
        DataClass::InternalOnly
    );
}

#[test]
fn rejects_l7_without_tls_grpc_without_mtls_and_unknown_subnet_targets() {
    let mut catalog = CloudNetworkCatalog::default();
    catalog.create_vpc(vpc_create()).expect("vpc create");
    catalog.add_subnet(subnet_create()).expect("subnet create");

    let tls_error = catalog
        .create_load_balancer(LoadBalancerCreate {
            listeners: vec![ListenerCreate {
                port: 443,
                target_group_id: "tg_api".to_string(),
                tls_certificate: None,
            }],
            ..lb_create()
        })
        .expect_err("L7 listeners need TLS certificates");
    assert_eq!(tls_error, CloudNetworkError::L7RequiresTls);

    let mtls_error = catalog
        .create_load_balancer(LoadBalancerCreate {
            mtls: None,
            ..lb_create()
        })
        .expect_err("gRPC front doors require mTLS config");
    assert_eq!(mtls_error, CloudNetworkError::GrpcRequiresMtls);

    let subnet_error = catalog
        .create_load_balancer(LoadBalancerCreate {
            target_groups: vec![TargetGroupCreate {
                id: "tg_api".to_string(),
                subnet_ids: vec!["oyatie:cloud:region-alpha1:ten_alpha:subnet:missing".to_string()],
                health_check_path: Some("/healthz".to_string()),
            }],
            ..lb_create()
        })
        .expect_err("LB target subnets must exist");
    assert_eq!(subnet_error, CloudNetworkError::UnknownSubnet);
}
