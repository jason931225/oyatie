use iam_identity_workload_svid_operator_app::{
    DEFAULT_NAMESPACE, DEFAULT_ROTATION_WINDOW_SECS, DEFAULT_TTL_SECS, ENV_CELL_ID, ENV_NAMESPACE,
    ENV_ROTATION_WINDOW_SECS, ENV_TTL_SECS, OperatorConfig, OperatorStartupConfigError,
    PDP_SVID_SECRET_NAME,
};

fn cfg(pairs: &[(&str, &str)]) -> Result<OperatorConfig, OperatorStartupConfigError> {
    OperatorConfig::from_env_pairs(pairs.iter().map(|(k, v)| (*k, *v)))
}

#[test]
fn builds_desired_spec_from_cell_id_with_defaults() {
    let config = cfg(&[(ENV_CELL_ID, "7")]).expect("valid config");
    assert_eq!(
        config.desired.spiffe_id,
        "spiffe://oyatie.cell-7/platform/cloud-iam-pdp"
    );
    assert_eq!(config.desired.secret_name, PDP_SVID_SECRET_NAME);
    assert_eq!(config.desired.secret_namespace, DEFAULT_NAMESPACE);
    assert_eq!(config.desired.ttl_secs, DEFAULT_TTL_SECS);
    assert_eq!(
        config.desired.rotation_window_secs,
        DEFAULT_ROTATION_WINDOW_SECS
    );
}

#[test]
fn honors_explicit_namespace_ttl_and_rotation_window() {
    let config = cfg(&[
        (ENV_CELL_ID, "east-1a"),
        (ENV_NAMESPACE, "cloud-iam-prod"),
        (ENV_TTL_SECS, "7200"),
        (ENV_ROTATION_WINDOW_SECS, "1800"),
    ])
    .expect("valid config");
    assert_eq!(
        config.desired.spiffe_id,
        "spiffe://oyatie.cell-east-1a/platform/cloud-iam-pdp"
    );
    assert_eq!(config.desired.secret_namespace, "cloud-iam-prod");
    assert_eq!(config.desired.ttl_secs, 7_200);
    assert_eq!(config.desired.rotation_window_secs, 1_800);
}

#[test]
fn missing_cell_id_is_a_startup_refusal() {
    assert_eq!(
        cfg(&[(ENV_NAMESPACE, "cloud-iam")]),
        Err(OperatorStartupConfigError::MissingCellId)
    );
    assert_eq!(
        cfg(&[(ENV_CELL_ID, "   ")]),
        Err(OperatorStartupConfigError::MissingCellId)
    );
}

#[test]
fn malformed_cell_id_is_a_startup_refusal() {
    let err = cfg(&[(ENV_CELL_ID, "has/slash")]).unwrap_err();
    assert!(matches!(
        err,
        OperatorStartupConfigError::MalformedCellId(_)
    ));
}

#[test]
fn non_numeric_ttl_is_a_startup_refusal() {
    let err = cfg(&[(ENV_CELL_ID, "7"), (ENV_TTL_SECS, "soon")]).unwrap_err();
    assert!(matches!(
        err,
        OperatorStartupConfigError::InvalidNumber { .. }
    ));
}

#[test]
fn zero_ttl_is_a_startup_refusal() {
    assert_eq!(
        cfg(&[(ENV_CELL_ID, "7"), (ENV_TTL_SECS, "0")]),
        Err(OperatorStartupConfigError::ZeroTtl)
    );
}

#[test]
fn rotation_window_at_or_above_ttl_is_a_startup_refusal() {
    let err = cfg(&[
        (ENV_CELL_ID, "7"),
        (ENV_TTL_SECS, "600"),
        (ENV_ROTATION_WINDOW_SECS, "600"),
    ])
    .unwrap_err();
    assert!(matches!(
        err,
        OperatorStartupConfigError::RotationWindowNotBelowTtl { .. }
    ));
}
