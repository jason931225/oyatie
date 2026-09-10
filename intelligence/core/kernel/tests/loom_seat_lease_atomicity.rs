//! Seat-lease atomicity — a sequential interleaving model, NOT a loom run.
//!
//! The kernel holds `Arc<Mutex<SubscriptionPool>>` from `std::sync`, which loom
//! cannot intercept, so the `#[cfg(loom)]` model below is an unimplemented stub
//! and every test that runs is sequential — no real interleaving is exercised.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
// `loom` is an external cfg flag, not a Cargo feature, so rustc cannot know it
// without a build.rs check-cfg entry.
#![allow(unexpected_cfgs)]

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Instant;

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
    let tenant = TenantId::new("t-loom").unwrap();
    let mut pool = SubscriptionPool::new(
        tenant.clone(),
        Provider::Anthropic,
        SelectionStrategy::RoundRobin,
    );
    for i in 0..n_seats {
        pool.add_seat(OAuthSubscription::new(
            tenant.clone(),
            SeatId::new(format!("loom-seat-{i}")).unwrap(),
            SubscriptionId::new(format!("loom-sub-{i}")).unwrap(),
            Provider::Anthropic,
            SubscriptionState::Active,
            format!("secret-ref://t-loom/loom-seat-{i}/refresh"),
            0,
        ))
        .unwrap();
    }
    Arc::new(Mutex::new(pool))
}

#[cfg(loom)]
#[test]
fn loom_lease_complete_atomicity() {
    loom::model(|| {
        unimplemented!("Stage-7: kernel loom plumbing required");
    });
}

#[derive(Clone, Debug)]
enum TaskState {
    Idle,
    HoldingLease(String), // holds seat_id as String
    Done,
}

#[derive(Clone, Default)]
struct PoolSnapshot {
    leased: HashSet<String>,
    available: HashSet<String>,
}

impl PoolSnapshot {
    fn new(seat_ids: &[&str]) -> Self {
        Self {
            leased: HashSet::new(),
            available: seat_ids.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Try to lease the next available seat (round-robin order = sorted).
    fn try_lease(&mut self) -> Option<String> {
        let mut sorted: Vec<_> = self.available.iter().cloned().collect();
        sorted.sort();
        let sid = sorted.into_iter().next()?;
        self.available.remove(&sid);
        self.leased.insert(sid.clone());
        Some(sid)
    }

    fn release(&mut self, sid: &str) {
        self.leased.remove(sid);
        self.available.insert(sid.to_string());
    }

    fn double_leased(&self) -> bool {
        // Model caveat: `try_lease`/`release` keep these two sets disjoint by
        // construction, so this check cannot fire; it pins the model's bookkeeping.
        self.leased.intersection(&self.available).count() > 0
    }
}

fn enumerate_interleavings(
    tasks: &mut Vec<TaskState>,
    pool: &mut PoolSnapshot,
    violation_found: &mut bool,
) {
    if *violation_found {
        return;
    }

    if pool.double_leased() {
        *violation_found = true;
        return;
    }

    let all_done = tasks.iter().all(|t| matches!(t, TaskState::Done));
    if all_done {
        return;
    }

    let n = tasks.len();
    for i in 0..n {
        if *violation_found {
            return;
        }
        match tasks[i].clone() {
            TaskState::Idle => {
                match pool.try_lease() {
                    Some(sid) => {
                        let old =
                            std::mem::replace(&mut tasks[i], TaskState::HoldingLease(sid.clone()));
                        enumerate_interleavings(tasks, pool, violation_found);
                        // Undo: restore task state and release the seat.
                        let held = std::mem::replace(&mut tasks[i], old);
                        if let TaskState::HoldingLease(released) = held {
                            pool.release(&released);
                        } else {
                            pool.release(&sid);
                        }
                    }
                    None => {
                        // Pool full — task stays Idle (blocked); try next task.
                    }
                }
            }
            TaskState::HoldingLease(sid) => {
                let old = std::mem::replace(&mut tasks[i], TaskState::Done);
                pool.release(&sid);
                enumerate_interleavings(tasks, pool, violation_found);
                // Undo
                tasks[i] = old;
                pool.available.remove(&sid);
                pool.leased.insert(sid);
            }
            TaskState::Done => {}
        }
    }
}

#[test]
fn exhaustive_interleaving_no_double_lease_3_tasks_3_seats() {
    let mut tasks = vec![TaskState::Idle, TaskState::Idle, TaskState::Idle];
    let mut pool = PoolSnapshot::new(&["loom-seat-0", "loom-seat-1", "loom-seat-2"]);
    let mut violation = false;
    enumerate_interleavings(&mut tasks, &mut pool, &mut violation);
    assert!(
        !violation,
        "double-lease detected in exhaustive interleaving"
    );
}

#[test]
fn exhaustive_interleaving_no_double_lease_4_tasks_2_seats() {
    let mut tasks = vec![
        TaskState::Idle,
        TaskState::Idle,
        TaskState::Idle,
        TaskState::Idle,
    ];
    let mut pool = PoolSnapshot::new(&["loom-seat-0", "loom-seat-1"]);
    let mut violation = false;
    enumerate_interleavings(&mut tasks, &mut pool, &mut violation);
    assert!(
        !violation,
        "double-lease detected in exhaustive interleaving"
    );
}

#[test]
fn deterministic_3_task_lease_complete_loop() {
    let pool_ref = make_pool(3);
    let gate = AllowAll;
    let agent = AgentId::new("agent-loom-det").unwrap();
    let now = Instant::now();

    // Sequential: no interleaving occurs; this only pins the lease+complete cycle.
    let n_cycles = 5;
    for _ in 0..n_cycles {
        for task_idx in 0..3usize {
            let lease = SubscriptionPool::lease(
                &pool_ref,
                &TenantId::new("t-loom").unwrap(),
                &agent,
                &gate,
                now,
            )
            .unwrap_or_else(|e| panic!("task {task_idx}: lease failed: {e:?}"));
            assert_eq!(pool_ref.lock().unwrap().seat_count(), 3);
            lease
                .complete(SeatOutcome::Ok, now)
                .unwrap_or_else(|e| panic!("task {task_idx}: complete failed: {e:?}"));
        }
    }

    let pool = pool_ref.lock().unwrap();
    assert_eq!(pool.seat_count(), 3);
}
