//! Dynamic secret-lease lifecycle: the fail-closed gates on issue, renew and revoke.

use std::collections::BTreeMap;

use secrets_domain::{
    DynamicLease, LeaseError, LeaseId, LeasePolicy, LeaseState, MAX_LEASE_LIFETIME_SECONDS,
    MAX_LEASE_TTL_SECONDS, MIN_LEASE_TTL_SECONDS, RevocationReason,
};
use shared_platform_contracts_kernel::identity::{Principal, PrincipalKind, PrincipalState};

const T0: u64 = 1_750_000_000;
const TTL: u64 = 300;
const LIFETIME: u64 = 1_000;

fn workload_principal() -> Principal {
    Principal {
        principal_id: "wl-api-7f3a".to_owned(),
        tenant_id: "ten-alpha".to_owned(),
        identity_domain_id: "dom-primordial".to_owned(),
        kind: PrincipalKind::Workload,
        state: PrincipalState::Active,
        group_ids: vec![],
        attributes: BTreeMap::new(),
    }
}

fn policy() -> LeasePolicy {
    LeasePolicy::new(TTL, 2, LIFETIME).expect("policy")
}

fn lease() -> DynamicLease {
    DynamicLease::issue(
        LeaseId::new("lease/abc123").expect("lease id"),
        "secret://ten_alpha/db-creds",
        &workload_principal(),
        policy(),
        T0,
    )
    .expect("issue")
}

#[test]
fn green_issue_live_within_ttl_bound_to_workload() {
    let issued = lease();
    assert_eq!(issued.state(T0), LeaseState::Live);
    assert_eq!(issued.state(T0 + TTL - 1), LeaseState::Live);
    assert!(issued.assert_live(T0 + TTL - 1).is_ok());
    assert_eq!(issued.expires_at_epoch_seconds(), T0 + TTL);
    assert_eq!(issued.absolute_expiry_epoch_seconds(), T0 + LIFETIME);
    assert_eq!(issued.principal_id(), "wl-api-7f3a");
    assert_eq!(issued.tenant_id(), "ten-alpha");
}

#[test]
fn red_non_workload_principals_rejected() {
    let mut human = workload_principal();
    human.kind = PrincipalKind::Human;
    assert_eq!(
        DynamicLease::issue(
            LeaseId::new("lease/x").unwrap(),
            "secret://s",
            &human,
            policy(),
            T0
        ),
        Err(LeaseError::PrincipalNotWorkload {
            kind: PrincipalKind::Human
        })
    );
    let mut federated = workload_principal();
    federated.kind = PrincipalKind::FederatedExternal;
    assert!(matches!(
        DynamicLease::issue(
            LeaseId::new("lease/x").unwrap(),
            "secret://s",
            &federated,
            policy(),
            T0
        ),
        Err(LeaseError::PrincipalNotWorkload { .. })
    ));
}

#[test]
fn red_non_active_principals_fail_closed() {
    for state in [
        PrincipalState::Pending,
        PrincipalState::Suspended,
        PrincipalState::Deprovisioned,
    ] {
        let mut principal = workload_principal();
        principal.state = state;
        assert_eq!(
            DynamicLease::issue(
                LeaseId::new("lease/x").unwrap(),
                "secret://s",
                &principal,
                policy(),
                T0
            ),
            Err(LeaseError::PrincipalNotOperational { state })
        );
    }
}

#[test]
fn red_contract_violating_principal_rejected() {
    let mut malformed = workload_principal();
    malformed.principal_id = String::new(); // violates the G001 slug contract
    assert!(matches!(
        DynamicLease::issue(
            LeaseId::new("lease/x").unwrap(),
            "secret://s",
            &malformed,
            policy(),
            T0
        ),
        Err(LeaseError::PrincipalContractViolation { .. })
    ));
}

#[test]
fn red_expiry_bound_is_inclusive_and_fails_closed() {
    let issued = lease();
    assert_eq!(issued.state(T0 + TTL), LeaseState::Expired);
    assert_eq!(
        issued.assert_live(T0 + TTL),
        Err(LeaseError::Expired {
            at_epoch_seconds: T0 + TTL
        })
    );
}

#[test]
fn renew_extends_from_now_never_stacking() {
    let mut renewed = lease();
    let new_expiry = renewed.renew(T0 + 100).expect("renew");
    assert_eq!(new_expiry, T0 + 100 + TTL);
    assert_eq!(renewed.renewals_used(), 1);
    assert_eq!(renewed.state(T0 + TTL + 50), LeaseState::Live);
}

#[test]
fn red_expired_lease_cannot_renew() {
    let mut expired = lease();
    assert_eq!(
        expired.renew(T0 + TTL),
        Err(LeaseError::Expired {
            at_epoch_seconds: T0 + TTL
        })
    );
    assert_eq!(expired.state(T0 + TTL), LeaseState::Expired);
}

#[test]
fn red_renewal_budget_exhausts() {
    let mut budgeted = lease();
    budgeted.renew(T0 + 10).expect("first");
    budgeted.renew(T0 + 20).expect("second");
    assert_eq!(
        budgeted.renew(T0 + 30),
        Err(LeaseError::RenewalsExhausted { max_renewals: 2 })
    );
    assert_eq!(budgeted.state(T0 + 30), LeaseState::Live);
}

#[test]
fn red_absolute_lifetime_ceiling_clamps_and_terminates() {
    let generous = LeasePolicy::new(TTL, 100, LIFETIME).expect("policy");
    let mut capped = DynamicLease::issue(
        LeaseId::new("lease/cap").unwrap(),
        "secret://s",
        &workload_principal(),
        generous,
        T0,
    )
    .expect("issue");

    assert_eq!(capped.renew(T0 + 250).expect("renew 1"), T0 + 550);
    assert_eq!(capped.renew(T0 + 500).expect("renew 2"), T0 + 800);
    let clamped = capped.renew(T0 + 750).expect("clamped renew");
    assert_eq!(
        clamped,
        T0 + LIFETIME,
        "expiry clamps to the absolute ceiling"
    );

    assert_eq!(
        capped.renew(T0 + LIFETIME - 10),
        Err(LeaseError::MaxLifetimeReached {
            absolute_expiry_epoch_seconds: T0 + LIFETIME
        })
    );

    assert_eq!(capped.state(T0 + LIFETIME), LeaseState::Expired);
}

#[test]
fn revocation_emits_caep_event_exactly_once_and_dominates() {
    let mut revoked = lease();
    let event = revoked
        .revoke(T0 + 5, RevocationReason::CompromiseSuspected)
        .expect("first revocation emits the event");
    assert_eq!(event.lease_id.value(), "lease/abc123");
    assert_eq!(event.principal_id, "wl-api-7f3a");
    assert_eq!(event.tenant_id, "ten-alpha");
    assert_eq!(event.secret_reference, "secret://ten_alpha/db-creds");
    assert_eq!(event.reason, RevocationReason::CompromiseSuspected);
    assert_eq!(event.revoked_at_epoch_seconds, T0 + 5);

    assert!(
        revoked
            .revoke(T0 + 50, RevocationReason::Administrative)
            .is_none()
    );
    assert_eq!(
        revoked.assert_live(T0 + 60),
        Err(LeaseError::Revoked {
            at_epoch_seconds: T0 + 5
        })
    );

    assert_eq!(revoked.state(T0 + TTL + 1), LeaseState::Revoked);
    assert_eq!(
        revoked.renew(T0 + 10),
        Err(LeaseError::Revoked {
            at_epoch_seconds: T0 + 5
        })
    );
}

#[test]
fn debug_redacts_principal_and_secret_reference() {
    let issued = lease();
    let rendered = format!("{issued:?}");
    assert!(
        !rendered.contains("wl-api-7f3a"),
        "principal_id must be redacted: {rendered}"
    );
    assert!(
        !rendered.contains("db-creds"),
        "secret_reference must be redacted: {rendered}"
    );
    assert!(rendered.contains("[REDACTED]"));
    assert!(
        rendered.contains("lease/abc123"),
        "lease id stays visible for correlation"
    );
}

#[test]
fn red_validation_gates() {
    let principal = workload_principal();
    assert_eq!(
        DynamicLease::issue(
            LeaseId::new("lease/x").unwrap(),
            " ",
            &principal,
            policy(),
            T0
        ),
        Err(LeaseError::InvalidSecretReference)
    );
    assert_eq!(
        LeasePolicy::new(MIN_LEASE_TTL_SECONDS - 1, 0, LIFETIME),
        Err(LeaseError::TtlOutOfBounds {
            requested_seconds: MIN_LEASE_TTL_SECONDS - 1
        })
    );
    assert_eq!(
        LeasePolicy::new(MAX_LEASE_TTL_SECONDS + 1, 0, MAX_LEASE_LIFETIME_SECONDS),
        Err(LeaseError::TtlOutOfBounds {
            requested_seconds: MAX_LEASE_TTL_SECONDS + 1
        })
    );
    assert_eq!(
        LeasePolicy::new(TTL, 0, TTL - 1),
        Err(LeaseError::LifetimeOutOfBounds {
            requested_seconds: TTL - 1
        })
    );
    assert_eq!(
        LeasePolicy::new(TTL, 0, MAX_LEASE_LIFETIME_SECONDS + 1),
        Err(LeaseError::LifetimeOutOfBounds {
            requested_seconds: MAX_LEASE_LIFETIME_SECONDS + 1
        })
    );
    assert!(LeasePolicy::new(TTL, 0, TTL).is_ok());
    assert!(LeasePolicy::new(MAX_LEASE_TTL_SECONDS, 0, MAX_LEASE_LIFETIME_SECONDS).is_ok());

    assert!(LeaseId::new("lease/abc").is_ok());
    assert_eq!(LeaseId::new("abc"), Err(LeaseError::InvalidLeaseId));
    assert_eq!(LeaseId::new("lease/"), Err(LeaseError::InvalidLeaseId));
    assert_eq!(LeaseId::new("lease/a/b"), Err(LeaseError::InvalidLeaseId));
}
