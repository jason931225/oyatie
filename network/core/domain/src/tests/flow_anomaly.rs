use super::*;

#[test]
fn records_flow_anomaly_only_for_flow_logged_vpc() {
    let mut catalog = CloudNetworkCatalog::default();
    catalog.create_vpc(vpc_create()).expect("vpc create");
    let event = catalog
        .record_flow_anomaly(
            "flowanom_001".to_string(),
            "oyatie:cloud:region-alpha1:ten_alpha:vpc:prod".to_string(),
            FlowAnomalySeverity::High,
            "egress spike to undeclared cidr".to_string(),
            1_700_000_040,
        )
        .expect("flow anomaly event is valid");
    assert_eq!(event.severity.value, FlowAnomalySeverity::High);
    assert_eq!(catalog.anomalies().count(), 1);

    let duplicate = catalog
        .record_flow_anomaly(
            "flowanom_001".to_string(),
            "oyatie:cloud:region-alpha1:ten_alpha:vpc:prod".to_string(),
            FlowAnomalySeverity::Critical,
            "duplicate evidence id".to_string(),
            1_700_000_041,
        )
        .expect_err("flow anomaly IDs are immutable evidence keys");
    assert_eq!(duplicate, CloudNetworkError::DuplicateFlowAnomaly);
}
