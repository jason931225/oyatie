use super::*;
use data_boundary_kernel::DataClass;

fn mesh_create() -> ServiceMeshCellCreate {
    ServiceMeshCellCreate {
        mesh_id: "mesh_prod_alpha".to_string(),
        tenant_id: "ten_alpha".to_string(),
        region: "region-alpha1".to_string(),
        cell_id: "cell-region-alpha1-a-001".to_string(),
        vpc_id: "oyatie:cloud:region-alpha1:ten_alpha:vpc:prod".to_string(),
        namespace: "mesh-cell-region-alpha1-a-001".to_string(),
        mode: ServiceMeshMode::IstioAmbient,
        edge_gateway: MeshGatewayKind::Envoy,
        mtls_everywhere: true,
        ext_authz_enabled: true,
        cross_cell_policy_ref: "cedar/network/cross-cell".to_string(),
        audit_stream_ref: "audit/network/mesh".to_string(),
        health_alarm_ref: "alarm/network/mesh-control-plane".to_string(),
        control_plane_replicas: 3,
        quarterly_upgrade_drill: true,
        state: ServiceMeshState::Creating,
        data_class: DataClass::Public,
        created_at_epoch_seconds: 1_700_000_080,
    }
}

#[test]
fn creates_service_mesh_cell_with_ambient_envoy_policy_and_audit_chain() {
    let mut catalog = CloudNetworkCatalog::default();
    catalog.create_vpc(vpc_create()).expect("vpc create");
    let mesh = catalog
        .create_service_mesh_cell(mesh_create())
        .expect("service mesh cell is valid");

    assert_eq!(mesh.mode.value, ServiceMeshMode::IstioAmbient);
    assert_eq!(mesh.edge_gateway.value, MeshGatewayKind::Envoy);
    assert!(mesh.mtls_everywhere.value);
    assert!(mesh.ext_authz_enabled.value);
    assert_eq!(mesh.control_plane_replicas.value, 3);
    assert_eq!(
        mesh.cross_cell_policy_ref
            .data_class
            .compatibility_data_class(),
        DataClass::InternalOnly
    );
}

#[test]
fn rejects_service_mesh_without_ambient_envoy_mtls_policy_or_cell_scope() {
    let mut catalog = CloudNetworkCatalog::default();
    catalog.create_vpc(vpc_create()).expect("vpc create");

    let mode_error = catalog
        .create_service_mesh_cell(ServiceMeshCellCreate {
            mode: ServiceMeshMode::Sidecar,
            ..mesh_create()
        })
        .expect_err("mesh mode is fixed to ambient");
    assert_eq!(mode_error, CloudNetworkError::InvalidMeshMode);

    let gateway_error = catalog
        .create_service_mesh_cell(ServiceMeshCellCreate {
            edge_gateway: MeshGatewayKind::Nginx,
            ..mesh_create()
        })
        .expect_err("edge gateway is fixed to Envoy");
    assert_eq!(gateway_error, CloudNetworkError::InvalidMeshGateway);

    let mtls_error = catalog
        .create_service_mesh_cell(ServiceMeshCellCreate {
            mtls_everywhere: false,
            ..mesh_create()
        })
        .expect_err("mesh requires mTLS everywhere");
    assert_eq!(mtls_error, CloudNetworkError::MeshMtlsRequired);

    let cell_error = catalog
        .create_service_mesh_cell(ServiceMeshCellCreate {
            cell_id: "cell-region-gamma1-a-001".to_string(),
            namespace: "mesh-cell-region-gamma1-a-001".to_string(),
            ..mesh_create()
        })
        .expect_err("mesh cell must belong to region");
    assert_eq!(cell_error, CloudNetworkError::InvalidCellId);
}
