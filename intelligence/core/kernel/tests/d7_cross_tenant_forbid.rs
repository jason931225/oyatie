//! Per-tenant isolation contract: the kernel must consult the gate before
//! it hands back a seat, and a `Forbid` must survive any number of allows.
//!
//! The gate here is a double; a concrete policy engine has its own corpus.
use std::cell::RefCell;
use std::time::Instant;

use intelligence_kernel::{
    AgentId, AuthzDecision, AuthzGate, AuthzRequest, OAuthSubscription, Provider, SeatId,
    SelectionStrategy, SubscriptionId, SubscriptionPool, SubscriptionPoolError, SubscriptionState,
    TenantId,
};

/// Records every request it sees, and forbids across tenants.
struct PolicyEngineLikeGate {
    requests: RefCell<Vec<(String, String)>>,
}

impl PolicyEngineLikeGate {
    fn new() -> Self {
        Self {
            requests: RefCell::new(Vec::new()),
        }
    }

    fn calls(&self) -> usize {
        self.requests.borrow().len()
    }
}

impl AuthzGate for PolicyEngineLikeGate {
    fn decide(&self, request: &AuthzRequest<'_>) -> AuthzDecision {
        self.requests.borrow_mut().push((
            request.principal_tenant.as_str().to_string(),
            request.resource_tenant.as_str().to_string(),
        ));
        if request.principal_tenant != request.resource_tenant {
            return AuthzDecision::Forbid;
        }
        AuthzDecision::Allow
    }
}

fn tenant(s: &str) -> TenantId {
    TenantId::new(s).unwrap()
}
fn agent(s: &str) -> AgentId {
    AgentId::new(s).unwrap()
}
fn seat(s: &str) -> SeatId {
    SeatId::new(s).unwrap()
}
fn sub(s: &str) -> SubscriptionId {
    SubscriptionId::new(s).unwrap()
}

fn make_sub(tenant_str: &str, seat_str: &str) -> OAuthSubscription {
    OAuthSubscription::new(
        tenant(tenant_str),
        seat(seat_str),
        sub(&format!("{seat_str}-sub-1")),
        Provider::Anthropic,
        SubscriptionState::Active,
        format!("secret-ref://{tenant_str}/{seat_str}/refresh"),
        0,
    )
}

#[test]
fn same_tenant_select_consults_gate_and_succeeds() {
    let mut pool = SubscriptionPool::new(
        tenant("t-acme"),
        Provider::Anthropic,
        SelectionStrategy::RoundRobin,
    );
    pool.add_seat(make_sub("t-acme", "seat-a")).unwrap();

    let gate = PolicyEngineLikeGate::new();
    let now = Instant::now();

    let picked = pool
        .select(&tenant("t-acme"), &agent("agent-acme-1"), &gate, now)
        .unwrap();
    assert_eq!(picked, seat("seat-a"));
    assert!(gate.calls() >= 1, "kernel MUST consult the AuthzGate");
}

#[test]
fn select_with_foreign_principal_tenant_is_forbidden_by_gate() {
    // The pool is t-acme's; the caller's server-verified principal is t-evil.
    // The kernel must carry the principal tenant through to the gate, or a
    // cross-tenant pool mis-route reaches the gate looking legitimate.
    let mut pool = SubscriptionPool::new(
        tenant("t-acme"),
        Provider::Anthropic,
        SelectionStrategy::RoundRobin,
    );
    pool.add_seat(make_sub("t-acme", "seat-a")).unwrap();

    let gate = PolicyEngineLikeGate::new();
    let now = Instant::now();

    assert_eq!(
        pool.select(&tenant("t-evil"), &agent("agent-evil-1"), &gate, now),
        Err(SubscriptionPoolError::ForbiddenByPolicy)
    );
    assert!(gate.calls() >= 1, "kernel MUST consult the AuthzGate");
}

#[test]
fn cross_tenant_pool_rejects_select_for_foreign_tenant_principal() {
    // The REST adapter builds the real AuthzRequest; the kernel contract
    // asserted here is only that the gate's verdict is final.
    let mut pool = SubscriptionPool::new(
        tenant("t-acme"),
        Provider::Anthropic,
        SelectionStrategy::RoundRobin,
    );
    pool.add_seat(make_sub("t-acme", "seat-a")).unwrap();

    struct AlwaysForbid;
    impl AuthzGate for AlwaysForbid {
        fn decide(&self, _request: &AuthzRequest<'_>) -> AuthzDecision {
            AuthzDecision::Forbid
        }
    }

    let now = Instant::now();
    assert_eq!(
        pool.select(
            &tenant("t-acme"),
            &agent("agent-evil-1"),
            &AlwaysForbid,
            now
        ),
        Err(SubscriptionPoolError::ForbiddenByPolicy)
    );
}

#[test]
fn deny_wins_even_when_pool_has_capacity() {
    let mut pool = SubscriptionPool::new(
        tenant("t-acme"),
        Provider::Anthropic,
        SelectionStrategy::RoundRobin,
    );
    for s in ["seat-a", "seat-b", "seat-c", "seat-d", "seat-e"] {
        pool.add_seat(make_sub("t-acme", s)).unwrap();
    }

    struct AlwaysForbid;
    impl AuthzGate for AlwaysForbid {
        fn decide(&self, _request: &AuthzRequest<'_>) -> AuthzDecision {
            AuthzDecision::Forbid
        }
    }

    let now = Instant::now();
    // A Forbid outranks any number of eligible seats.
    assert_eq!(
        pool.select(&tenant("t-acme"), &agent("agent-1"), &AlwaysForbid, now),
        Err(SubscriptionPoolError::ForbiddenByPolicy)
    );
}

#[test]
fn adding_seat_with_wrong_tenant_is_rejected() {
    // A pool holds exactly one tenant, so a foreign seat in it would be a
    // data-plane isolation breach that no gate is consulted about.
    let mut pool = SubscriptionPool::new(
        tenant("t-acme"),
        Provider::Anthropic,
        SelectionStrategy::RoundRobin,
    );
    let err = pool
        .add_seat(make_sub("t-evil", "seat-a"))
        .expect_err("foreign-tenant seat must be rejected");
    assert_eq!(err, SubscriptionPoolError::ForbiddenByPolicy);
    assert_eq!(pool.seat_count(), 0);
}

#[test]
fn invalid_identifiers_are_rejected_at_construction() {
    assert_eq!(
        TenantId::new("").unwrap_err(),
        SubscriptionPoolError::InvalidTenantId
    );
    assert_eq!(
        AgentId::new("   ").unwrap_err(),
        SubscriptionPoolError::InvalidAgentId
    );
    assert_eq!(
        SeatId::new("").unwrap_err(),
        SubscriptionPoolError::InvalidSeatId
    );
    assert_eq!(
        SubscriptionId::new("").unwrap_err(),
        SubscriptionPoolError::InvalidSubscriptionId
    );
}
