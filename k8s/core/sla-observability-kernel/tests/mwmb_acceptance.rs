//! Acceptance tests for `summarize_burn_rate_alert`.

use k8s_sla_observability_kernel::{
    BurnRatePolicy, ObservedControlPlaneStatus, SlaKernelError, SlaObservation, SlaPolicy,
    summarize_burn_rate_alert,
};

fn valid_obs(tenant_id: &str, cluster_name: &str) -> SlaObservation {
    SlaObservation::new(
        tenant_id,
        cluster_name,
        ObservedControlPlaneStatus::Active,
        1_000,
        1_000,
        None,
    )
}

fn good() -> SlaObservation {
    valid_obs("ten_acme", "prod-a")
}

#[test]
fn blank_tenant_id_in_fast_window_fails_closed() {
    let fast = valid_obs("", "prod-a");
    let slow = good();
    assert_eq!(
        summarize_burn_rate_alert(
            &fast,
            &slow,
            SlaPolicy::default(),
            BurnRatePolicy::default()
        )
        .unwrap_err(),
        SlaKernelError::InvalidClusterIdentity,
    );
}

#[test]
fn blank_cluster_name_in_slow_window_fails_closed() {
    let fast = good();
    let slow = valid_obs("ten_acme", "");
    assert_eq!(
        summarize_burn_rate_alert(
            &fast,
            &slow,
            SlaPolicy::default(),
            BurnRatePolicy::default()
        )
        .unwrap_err(),
        SlaKernelError::InvalidClusterIdentity,
    );
}

#[test]
fn whitespace_only_tenant_id_in_fast_window_fails_closed() {
    let fast = valid_obs("   ", "prod-a");
    let slow = good();
    assert_eq!(
        summarize_burn_rate_alert(
            &fast,
            &slow,
            SlaPolicy::default(),
            BurnRatePolicy::default()
        )
        .unwrap_err(),
        SlaKernelError::InvalidClusterIdentity,
    );
}

#[test]
fn blank_identity_in_both_windows_returns_invalid_cluster_identity() {
    let fast = valid_obs("", "");
    let slow = valid_obs("", "");
    assert_eq!(
        summarize_burn_rate_alert(
            &fast,
            &slow,
            SlaPolicy::default(),
            BurnRatePolicy::default()
        )
        .unwrap_err(),
        SlaKernelError::InvalidClusterIdentity,
    );
}

#[test]
fn blank_tenant_id_in_slow_window_fails_closed() {
    let fast = good();
    let slow = valid_obs("", "prod-a");
    assert_eq!(
        summarize_burn_rate_alert(
            &fast,
            &slow,
            SlaPolicy::default(),
            BurnRatePolicy::default()
        )
        .unwrap_err(),
        SlaKernelError::InvalidClusterIdentity,
    );
}

#[test]
fn whitespace_only_cluster_name_in_slow_window_fails_closed() {
    let fast = good();
    let slow = valid_obs("ten_acme", "\t");
    assert_eq!(
        summarize_burn_rate_alert(
            &fast,
            &slow,
            SlaPolicy::default(),
            BurnRatePolicy::default()
        )
        .unwrap_err(),
        SlaKernelError::InvalidClusterIdentity,
    );
}
