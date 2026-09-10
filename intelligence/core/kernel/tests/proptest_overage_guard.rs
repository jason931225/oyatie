//! Proptest invariants for the overage-guard state machine.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use intelligence_kernel::overage_guard::{
    ClaimClassification, CodexQuotaSignal, DEFAULT_RESUME_COOLDOWN, GuardDecision, GuardMode,
    HaltReason, OverageGuardPolicy, RepresentativeClaim, classify_codex_error,
    classify_representative_claim, evaluate_codex_quota, evaluate_representative_claim,
};
use intelligence_kernel::{
    OAuthSubscription, Provider, SeatId, SelectionStrategy, SubscriptionId, SubscriptionPool,
    SubscriptionState, TenantId,
};
use proptest::prelude::*;

const ALLOWED: [&str; 4] = [
    "five_hour",
    "seven_day",
    "five_hour_fallback",
    "seven_day_fallback",
];
const TRANSIENT: [&str; 3] = ["", "   ", "unknown"];

fn arc_pool_one_active() -> (Arc<Mutex<SubscriptionPool>>, SeatId) {
    let tenant = TenantId::new("t-overage").unwrap();
    let mut pool = SubscriptionPool::new(
        tenant.clone(),
        Provider::Anthropic,
        SelectionStrategy::RoundRobin,
    );
    let sid = SeatId::new("seat-0").unwrap();
    pool.add_seat(OAuthSubscription::new(
        tenant,
        sid.clone(),
        SubscriptionId::new("sub-0").unwrap(),
        Provider::Anthropic,
        SubscriptionState::Active,
        "secret-ref://t-overage/seat-0/refresh",
        0,
    ))
    .unwrap();
    (Arc::new(Mutex::new(pool)), sid)
}

proptest! {
    #[test]
    fn classification_is_total_and_correct(s in ".{0,40}") {
        let lowered = s.trim().to_ascii_lowercase();
        let class = classify_representative_claim(Some(&s));
        if lowered.is_empty() || lowered == "unknown" {
            prop_assert_eq!(class, ClaimClassification::Transient);
        } else if ALLOWED.contains(&lowered.as_str()) {
            let is_allowed = matches!(class, ClaimClassification::Allowed(_));
            prop_assert!(is_allowed);
        } else {
            // Brace-bearing literals are bound to locals: prop_assert*! stringify
            // their args into a format string and `{` would be misread.
            let expected = ClaimClassification::Overage { bucket: lowered };
            prop_assert_eq!(class, expected);
        }
    }

    #[test]
    fn mode_semantics_hold(s in ".{0,40}", enforce in any::<bool>()) {
        let now = Instant::now();
        let policy = if enforce { OverageGuardPolicy::enforce() } else { OverageGuardPolicy::warn() };
        let decision = evaluate_representative_claim(policy, Some(&s), now);
        match classify_representative_claim(Some(&s)) {
            ClaimClassification::Transient | ClaimClassification::Allowed(_) => {
                prop_assert_eq!(decision, GuardDecision::Continue);
            }
            ClaimClassification::Overage { .. } => {
                if enforce {
                    prop_assert!(decision.is_halt());
                } else {
                    let is_warn = matches!(decision, GuardDecision::Warn { .. });
                    prop_assert!(is_warn);
                    prop_assert!(!decision.is_halt());
                }
            }
        }
    }

    #[test]
    fn halted_seat_resumes_exactly_at_horizon(secs in 1u64..7200) {
        let (pool, sid) = arc_pool_one_active();
        let now = Instant::now();
        let horizon = Duration::from_secs(secs);
        let policy = OverageGuardPolicy::enforce().with_resume_cooldown(horizon);
        let decision = evaluate_representative_claim(policy, Some("overage"), now);

        let halted = pool.lock().unwrap().apply_overage_decision(&sid, &decision, now).unwrap();
        prop_assert!(halted);

        let p = pool.lock().unwrap();
        prop_assert!(!p.has_eligible_seat(now));
        prop_assert!(!p.has_eligible_seat(now + horizon - Duration::from_nanos(1)));
        prop_assert!(p.has_eligible_seat(now + horizon));
        prop_assert!(p.has_eligible_seat(now + horizon + Duration::from_secs(1)));
        let state = p.redacted_seat_statuses(now)[0].state;
        prop_assert_eq!(state, "halted");
    }

    #[test]
    fn admin_resume_reactivates_halted_seat(secs in 1u64..7200) {
        let (pool, sid) = arc_pool_one_active();
        let now = Instant::now();
        let policy = OverageGuardPolicy::enforce().with_resume_cooldown(Duration::from_secs(secs));
        let decision = evaluate_representative_claim(policy, Some("overage"), now);
        pool.lock().unwrap().apply_overage_decision(&sid, &decision, now).unwrap();

        let mut p = pool.lock().unwrap();
        prop_assert!(!p.has_eligible_seat(now));
        let resumed = p.admin_resume(&sid).unwrap();
        prop_assert!(resumed);
        prop_assert!(p.has_eligible_seat(now));
        // A second resume returns false: nothing was halted to resume.
        prop_assert!(!p.admin_resume(&sid).unwrap());
    }

    #[test]
    fn codex_only_usage_limit_reached_halts(et in "[a-z_]{1,30}", hint in proptest::option::of(0u64..3600)) {
        let now = Instant::now();
        let signal = classify_codex_error(Some(&et), hint);
        let decision = evaluate_codex_quota(OverageGuardPolicy::enforce(), &signal, now);
        if et == "usage_limit_reached" {
            let expected_signal = CodexQuotaSignal::Exhausted { resets_in_seconds: hint };
            prop_assert_eq!(&signal, &expected_signal);
            prop_assert!(decision.is_halt());
            let expected = now + hint.map(Duration::from_secs).unwrap_or(DEFAULT_RESUME_COOLDOWN);
            let expected_decision = GuardDecision::Halt {
                reason: HaltReason::QuotaExhausted,
                resume_at: Some(expected),
            };
            prop_assert_eq!(decision, expected_decision);
        } else {
            prop_assert_eq!(signal, CodexQuotaSignal::None);
            prop_assert_eq!(decision, GuardDecision::Continue);
        }
    }

    #[test]
    fn continue_and_warn_never_mutate_state(s in ".{0,40}") {
        let (pool, sid) = arc_pool_one_active();
        let now = Instant::now();
        let decision = evaluate_representative_claim(OverageGuardPolicy::warn(), Some(&s), now);
        prop_assume!(!decision.is_halt());
        let halted = pool.lock().unwrap().apply_overage_decision(&sid, &decision, now).unwrap();
        prop_assert!(!halted);
        prop_assert!(pool.lock().unwrap().has_eligible_seat(now));
    }
}

#[test]
fn allowed_bucket_round_trips_through_as_str() {
    for claim in [
        RepresentativeClaim::FiveHour,
        RepresentativeClaim::SevenDay,
        RepresentativeClaim::FiveHourFallback,
        RepresentativeClaim::SevenDayFallback,
    ] {
        assert_eq!(
            classify_representative_claim(Some(claim.as_str())),
            ClaimClassification::Allowed(claim),
        );
    }
}

#[test]
fn transient_inputs_never_halt_in_enforce_mode() {
    let now = Instant::now();
    for t in TRANSIENT {
        assert_eq!(
            evaluate_representative_claim(OverageGuardPolicy::enforce(), Some(t), now),
            GuardDecision::Continue
        );
    }
    assert_eq!(
        evaluate_representative_claim(OverageGuardPolicy::enforce(), None, now),
        GuardDecision::Continue
    );
}

#[test]
fn admin_resume_on_quota_exhaustion_halt_reactivates() {
    let (pool, sid) = arc_pool_one_active();
    let now = Instant::now();
    let signal = classify_codex_error(Some("usage_limit_reached"), Some(300));
    let decision = evaluate_codex_quota(OverageGuardPolicy::enforce(), &signal, now);
    let mut p = pool.lock().unwrap();
    assert!(p.apply_overage_decision(&sid, &decision, now).unwrap());
    assert!(!p.has_eligible_seat(now));
    assert!(p.has_eligible_seat(now + Duration::from_secs(300)));
    assert!(p.admin_resume(&sid).unwrap());
    assert!(p.has_eligible_seat(now));
}

#[test]
fn warn_mode_keeps_overage_seat_serving() {
    let (pool, sid) = arc_pool_one_active();
    let now = Instant::now();
    let decision = evaluate_representative_claim(OverageGuardPolicy::warn(), Some("overage"), now);
    assert!(!decision.is_halt());
    let mut p = pool.lock().unwrap();
    assert!(!p.apply_overage_decision(&sid, &decision, now).unwrap());
    assert!(p.has_eligible_seat(now));
    assert_eq!(p.redacted_seat_statuses(now)[0].state, "active");
    // The bucket rides on the warn, for the event the adapter emits.
    assert!(matches!(
        decision,
        GuardDecision::Warn {
            reason: HaltReason::RepresentativeClaimOverage { .. }
        }
    ));
    let _ = GuardMode::Warn;
}
