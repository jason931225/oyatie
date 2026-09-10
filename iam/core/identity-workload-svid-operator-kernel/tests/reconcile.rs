use iam_identity_workload_svid_operator_kernel::{
    Action, Clock, DesiredState, ObservedState, reconcile,
};

#[derive(Clone, Copy)]
struct FixedClock {
    now: u64,
}

impl Clock for FixedClock {
    fn now_epoch_seconds(&self) -> u64 {
        self.now
    }
}

const NOW: u64 = 1_000;
const TTL_SECS: u64 = 3_600;
const ROTATION_WINDOW_SECS: u64 = 600;

fn desired() -> DesiredState {
    DesiredState {
        spiffe_id: "spiffe://oyatie.cell-7/platform/cloud-iam-pdp".to_owned(),
        ttl_secs: TTL_SECS,
        rotation_window_secs: ROTATION_WINDOW_SECS,
        secret_name: "cloud-iam-pdp-svid".to_owned(),
        secret_namespace: "cloud-iam".to_owned(),
    }
}

#[test]
fn issues_when_no_secret_is_present() {
    let want = desired();
    let action = reconcile(&ObservedState::absent(), &want, &FixedClock { now: NOW });
    assert_eq!(
        action,
        Action::Issue {
            desired: want,
            requested_at_epoch_seconds: NOW,
        }
    );
}

#[test]
fn noops_when_leaf_is_comfortably_fresh() {
    let want = desired();
    let action = reconcile(
        &ObservedState::present(NOW + TTL_SECS),
        &want,
        &FixedClock { now: NOW },
    );
    assert_eq!(action, Action::Noop);
}

#[test]
fn rotates_when_leaf_is_within_the_rotation_window() {
    let want = desired();
    let inside_window = NOW + ROTATION_WINDOW_SECS - 100;
    let action = reconcile(
        &ObservedState::present(inside_window),
        &want,
        &FixedClock { now: NOW },
    );
    assert_eq!(
        action,
        Action::Rotate {
            desired: want,
            observed_leaf_not_after_epoch_seconds: inside_window,
            requested_at_epoch_seconds: NOW,
        }
    );
}

#[test]
fn rotates_exactly_at_the_window_boundary() {
    let want = desired();
    let action = reconcile(
        &ObservedState::present(NOW + ROTATION_WINDOW_SECS),
        &want,
        &FixedClock { now: NOW },
    );
    assert!(matches!(action, Action::Rotate { .. }));
}

#[test]
fn noops_one_second_above_the_window_boundary() {
    let want = desired();
    let action = reconcile(
        &ObservedState::present(NOW + ROTATION_WINDOW_SECS + 1),
        &want,
        &FixedClock { now: NOW },
    );
    assert_eq!(action, Action::Noop);
}

#[test]
fn rotates_an_already_expired_leaf_without_underflow() {
    let want = desired();
    let expired = NOW - 500;
    let action = reconcile(
        &ObservedState::present(expired),
        &want,
        &FixedClock { now: NOW },
    );
    assert_eq!(
        action,
        Action::Rotate {
            desired: want,
            observed_leaf_not_after_epoch_seconds: expired,
            requested_at_epoch_seconds: NOW,
        }
    );
}

#[test]
fn applying_issue_then_observing_the_fresh_leaf_is_idempotent() {
    let want = desired();
    let issue = reconcile(&ObservedState::absent(), &want, &FixedClock { now: NOW });
    assert!(matches!(issue, Action::Issue { .. }));
    let leaf_the_adapter_would_mint = ObservedState::present(NOW + want.ttl_secs);
    assert_eq!(
        reconcile(
            &leaf_the_adapter_would_mint,
            &want,
            &FixedClock { now: NOW }
        ),
        Action::Noop
    );
}

const ISSUED_AT: u64 = 1_700_000_000;

#[test]
fn rotates_when_the_clock_reads_before_the_leaf_could_have_been_issued() {
    let want = desired();
    let leaf_not_after = ISSUED_AT + TTL_SECS;
    let action = reconcile(
        &ObservedState::present(leaf_not_after),
        &want,
        &FixedClock { now: 0 },
    );
    assert_eq!(
        action,
        Action::Rotate {
            desired: want,
            observed_leaf_not_after_epoch_seconds: leaf_not_after,
            requested_at_epoch_seconds: 0,
        }
    );
}

#[test]
fn no_clock_reading_below_the_leafs_issuance_yields_noop() {
    let want = desired();
    let leaf_not_after = ISSUED_AT + TTL_SECS;
    let suppressed: Vec<u64> = [0, 1, ISSUED_AT - 1, ISSUED_AT - TTL_SECS]
        .into_iter()
        .filter(|now| {
            reconcile(
                &ObservedState::present(leaf_not_after),
                &want,
                &FixedClock { now: *now },
            ) == Action::Noop
        })
        .collect();
    assert!(
        suppressed.is_empty(),
        "clock readings that suppressed rotation: {suppressed:?}"
    );
}

/// A shortened `ttl_secs` leaves a longer-lived leaf in place, so the
/// `remaining > ttl_secs` disjunct also fires on a HEALTHY clock. Deriving the
/// issuance instant from the leaf instead of the clock dates the replacement
/// into the future there, and a not-yet-valid leaf is refused by every peer
/// the moment it is applied.
#[test]
fn a_shortened_ttl_never_dates_the_replacement_after_the_clock() {
    let want = desired();
    let issuance = match reconcile(
        &ObservedState::present(ISSUED_AT + 20 * TTL_SECS),
        &want,
        &FixedClock { now: ISSUED_AT },
    ) {
        Action::Issue {
            requested_at_epoch_seconds,
            ..
        }
        | Action::Rotate {
            requested_at_epoch_seconds,
            ..
        } => Some(requested_at_epoch_seconds),
        Action::Noop => None,
    };
    assert!(
        issuance.is_some_and(|at| at <= ISSUED_AT),
        "expected a mint instant at or before the clock reading {ISSUED_AT}, got {issuance:?}"
    );
}
