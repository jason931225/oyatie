//! Chaos-lite lease storm: many tasks contend for a small seat pool, then the
//! pool's invariants are checked against what actually happened.
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::cast_possible_truncation
    )
)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use intelligence_kernel::{
    AgentId, AuthzDecision, AuthzGate, AuthzRequest, OAuthSubscription, Provider, SeatId,
    SeatOutcome, SelectionStrategy, SubscriptionId, SubscriptionPool, SubscriptionPoolError,
    SubscriptionState, TenantId,
};

struct AllowAll;
impl AuthzGate for AllowAll {
    fn decide(&self, _r: &AuthzRequest<'_>) -> AuthzDecision {
        AuthzDecision::Allow
    }
}

fn make_pool(n_seats: usize) -> Arc<Mutex<SubscriptionPool>> {
    let tenant = TenantId::new("t-chaos").unwrap();
    let mut pool = SubscriptionPool::new(
        tenant.clone(),
        Provider::Anthropic,
        SelectionStrategy::RoundRobin,
    );
    for i in 0..n_seats {
        pool.add_seat(OAuthSubscription::new(
            tenant.clone(),
            SeatId::new(format!("chaos-seat-{i}")).unwrap(),
            SubscriptionId::new(format!("chaos-sub-{i}")).unwrap(),
            Provider::Anthropic,
            SubscriptionState::Active,
            format!("secret-ref://t-chaos/chaos-seat-{i}/refresh"),
            0,
        ))
        .unwrap();
    }
    Arc::new(Mutex::new(pool))
}

/// Weighted 70% Ok, 15% RateLimited429, 15% RefreshFailed.
fn pick_outcome(seed: u64) -> SeatOutcome {
    match seed % 20 {
        0..=13 => SeatOutcome::Ok,
        14..=16 => SeatOutcome::RateLimited429,
        _ => SeatOutcome::RefreshFailed,
    }
}

/// A second concurrent holder of one seat is the violation this records.
fn mark_leased(in_flight: &Mutex<HashMap<String, u32>>, seat: &str, violation: &AtomicBool) {
    let mut map = in_flight.lock().unwrap();
    let holders = map.entry(seat.to_string()).or_insert(0);
    *holders += 1;
    if *holders > 1 {
        violation.store(true, Ordering::SeqCst);
    }
}

fn mark_released(in_flight: &Mutex<HashMap<String, u32>>, seat: &str) {
    let mut map = in_flight.lock().unwrap();
    let holders = map.entry(seat.to_string()).or_insert(1);
    *holders = holders.saturating_sub(1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn chaos_lease_storm_100_tasks_5_seats_5_seconds() {
    const N_TASKS: usize = 100;
    const N_SEATS: usize = 5;
    const DURATION: Duration = Duration::from_secs(5);

    let pool_ref = make_pool(N_SEATS);
    let gate = Arc::new(AllowAll);
    let deadline = Instant::now() + DURATION;

    let total_ok = Arc::new(AtomicU64::new(0));
    let total_failures = Arc::new(AtomicU64::new(0));
    let total_ops = Arc::new(AtomicU64::new(0));
    let violation = Arc::new(AtomicBool::new(false));

    let in_flight: Arc<Mutex<HashMap<String, u32>>> = Arc::new(Mutex::new(HashMap::new()));

    let mut handles = Vec::with_capacity(N_TASKS);

    for task_idx in 0..N_TASKS {
        let pool_ref = Arc::clone(&pool_ref);
        let gate = Arc::clone(&gate);
        let agent = AgentId::new(format!("chaos-agent-{task_idx}")).unwrap();
        let in_flight = Arc::clone(&in_flight);
        let violation = Arc::clone(&violation);
        let total_ok = Arc::clone(&total_ok);
        let total_failures = Arc::clone(&total_failures);
        let total_ops = Arc::clone(&total_ops);

        handles.push(tokio::spawn(async move {
            let mut local_seed: u64 = task_idx as u64;

            loop {
                if Instant::now() >= deadline {
                    break;
                }

                let now = Instant::now();
                let lease_result = SubscriptionPool::lease(
                    &pool_ref,
                    &TenantId::new("t-chaos").unwrap(),
                    &agent,
                    gate.as_ref(),
                    now,
                );

                match lease_result {
                    Ok(lease) => {
                        let seat_key = lease.seat_id().as_str().to_string();

                        mark_leased(&in_flight, &seat_key, &violation);
                        tokio::task::yield_now().await;
                        mark_released(&in_flight, &seat_key);

                        local_seed = local_seed
                            .wrapping_add(1)
                            .wrapping_mul(6_364_136_223_846_793_005);
                        let outcome = pick_outcome(local_seed);

                        let is_ok = outcome == SeatOutcome::Ok;
                        let complete_now = Instant::now();
                        if lease.complete(outcome, complete_now).is_ok() {
                            total_ops.fetch_add(1, Ordering::Relaxed);
                            if is_ok {
                                total_ok.fetch_add(1, Ordering::Relaxed);
                            } else {
                                total_failures.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                    Err(SubscriptionPoolError::NoEligibleSeat) => {
                        // Every seat is leased or cooling down; retry later.
                        tokio::task::yield_now().await;
                    }
                    Err(e) => {
                        panic!("unexpected error in task {task_idx}: {e:?}");
                    }
                }
            }
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    assert!(
        !violation.load(Ordering::SeqCst),
        "double-lease violation detected during chaos storm"
    );

    assert_eq!(
        pool_ref.lock().unwrap().seat_count(),
        N_SEATS,
        "seat_count must remain {N_SEATS} after chaos storm"
    );

    let ok = total_ok.load(Ordering::Relaxed);
    let fail = total_failures.load(Ordering::Relaxed);
    let ops = total_ops.load(Ordering::Relaxed);
    assert_eq!(
        ok + fail,
        ops,
        "ok_count ({ok}) + failure_count ({fail}) must equal total_ops ({ops})"
    );

    let remaining: u32 = in_flight.lock().unwrap().values().sum();
    assert_eq!(
        remaining, 0,
        "all in-flight leases should be zero after task completion"
    );

    assert!(ops > 0, "at least one lease+complete cycle must have run");
}
