//! The transition function stays in the locked G001 contract
//! (`TenantLifecycleOperation::apply`) — this crate only PLANS which
//! contract operation to request next; it never invents transitions.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use shared_platform_contracts_kernel::tenancy::{TenantLifecycleOperation, TenantLifecycleState};
use shared_resource_provider_contract_kernel::{ContractShapeError, IdempotencyKey};

/// The lifecycle state a tenant CR may DECLARE as desired. `Provisioning`
/// is deliberately absent: it is the birth state, never a goal state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesiredTenantState {
    Active,
    Suspended,
    Retired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plan {
    Converged,
    /// Request exactly this contract operation next (one step per pass —
    /// level-triggered, so multi-hop paths converge across passes).
    Step(TenantLifecycleOperation),
    /// Desired state is unreachable from the observed state under the
    /// contract FSM (only: anything other than `Retired` once retired —
    /// tenant ids are never reused). Reconcilers surface this as a terminal
    /// condition instead of retrying forever.
    Unreachable,
}

/// Plan the single next contract operation that moves `observed` toward
/// `desired`. Total over the state space; the multi-hop case
/// (Provisioning -> Suspended) activates first and suspends on the next
/// pass, mirroring how cloud control planes sequence provision-then-halt.
#[must_use]
pub fn plan_next_operation(observed: TenantLifecycleState, desired: DesiredTenantState) -> Plan {
    use TenantLifecycleOperation as Op;
    use TenantLifecycleState as S;
    match (observed, desired) {
        (S::Active, DesiredTenantState::Active)
        | (S::Suspended, DesiredTenantState::Suspended)
        | (S::Retired, DesiredTenantState::Retired) => Plan::Converged,
        (S::Provisioning, DesiredTenantState::Active | DesiredTenantState::Suspended) => {
            Plan::Step(Op::Activate)
        }
        (S::Active, DesiredTenantState::Suspended) => Plan::Step(Op::Suspend),
        (S::Suspended, DesiredTenantState::Active) => Plan::Step(Op::Resume),
        (S::Provisioning | S::Active | S::Suspended, DesiredTenantState::Retired) => {
            Plan::Step(Op::Retire)
        }
        (S::Retired, _) => Plan::Unreachable,
    }
}

/// Derive the deterministic client-UUID idempotency key for one reconcile
/// step: stable across retries of the SAME step (same CR uid, same
/// generation, same planned operation from the same observed state) and
/// distinct across steps. This is the ACK/AIP-155 play — the controller is
/// the client, so the client token must be a pure function of the declared
/// intent, never a random draw (random keys would defeat dedup on
/// controller restart).
///
/// Shape: FNV-1a 64-bit folded over the inputs into the 122 free bits of a
/// canonical RFC 4122 v4-shaped UUID (version/variant nibbles forced).
pub fn derive_step_key(
    cr_uid: &str,
    generation: i64,
    step: &str,
) -> Result<IdempotencyKey, ContractShapeError> {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut lo = FNV_OFFSET;
    let mut hi = FNV_OFFSET ^ 0x5bd1_e995_7b93_c1a4;
    for chunk in [
        cr_uid.as_bytes(),
        &generation.to_be_bytes(),
        step.as_bytes(),
    ] {
        for &byte in chunk {
            lo = (lo ^ u64::from(byte)).wrapping_mul(FNV_PRIME);
            hi = (hi ^ u64::from(byte).rotate_left(17) ^ lo).wrapping_mul(FNV_PRIME);
        }
        // Domain-separate the three inputs so ("a", b"") and ("", b"a")
        // cannot collide across field boundaries.
        lo = (lo ^ 0xff).wrapping_mul(FNV_PRIME);
        hi = (hi ^ 0xfe).wrapping_mul(FNV_PRIME);
    }
    let bytes_hi = hi.to_be_bytes();
    let bytes_lo = lo.to_be_bytes();
    IdempotencyKey::new(format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-4{:01x}{:02x}-8{:01x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes_hi[0],
        bytes_hi[1],
        bytes_hi[2],
        bytes_hi[3],
        bytes_hi[4],
        bytes_hi[5],
        bytes_hi[6] & 0x0f,
        bytes_hi[7],
        bytes_lo[0] & 0x0f,
        bytes_lo[1],
        bytes_lo[2],
        bytes_lo[3],
        bytes_lo[4],
        bytes_lo[5],
        bytes_lo[6],
        bytes_lo[7],
    ))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TenantPhase {
    Converged,
    Progressing,
    Blocked { reason: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_OBSERVED: [TenantLifecycleState; 4] = [
        TenantLifecycleState::Provisioning,
        TenantLifecycleState::Active,
        TenantLifecycleState::Suspended,
        TenantLifecycleState::Retired,
    ];
    const ALL_DESIRED: [DesiredTenantState; 3] = [
        DesiredTenantState::Active,
        DesiredTenantState::Suspended,
        DesiredTenantState::Retired,
    ];

    #[test]
    fn every_planned_step_is_contract_legal() {
        for observed in ALL_OBSERVED {
            for desired in ALL_DESIRED {
                if let Plan::Step(operation) = plan_next_operation(observed, desired) {
                    operation.apply(observed).unwrap_or_else(|violation| {
                        panic!(
                            "planner proposed illegal {operation:?} from {observed:?}: {violation}"
                        )
                    });
                }
            }
        }
    }

    #[test]
    fn plans_converge_within_fsm_diameter() {
        for start in ALL_OBSERVED {
            for desired in ALL_DESIRED {
                let mut observed = start;
                let mut steps = 0;
                loop {
                    match plan_next_operation(observed, desired) {
                        Plan::Converged | Plan::Unreachable => break,
                        Plan::Step(operation) => {
                            observed = operation.apply(observed).unwrap();
                            steps += 1;
                            assert!(steps <= 3, "{start:?}->{desired:?} did not converge");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn converged_means_desired() {
        for desired in ALL_DESIRED {
            for observed in ALL_OBSERVED {
                if plan_next_operation(observed, desired) == Plan::Converged {
                    let expected = match desired {
                        DesiredTenantState::Active => TenantLifecycleState::Active,
                        DesiredTenantState::Suspended => TenantLifecycleState::Suspended,
                        DesiredTenantState::Retired => TenantLifecycleState::Retired,
                    };
                    assert_eq!(observed, expected);
                }
            }
        }
    }

    #[test]
    fn retired_blocks_everything_but_retired() {
        assert_eq!(
            plan_next_operation(TenantLifecycleState::Retired, DesiredTenantState::Retired),
            Plan::Converged
        );
        for desired in [DesiredTenantState::Active, DesiredTenantState::Suspended] {
            assert_eq!(
                plan_next_operation(TenantLifecycleState::Retired, desired),
                Plan::Unreachable
            );
        }
    }

    #[test]
    fn step_keys_are_deterministic_and_distinct() {
        let a = derive_step_key("uid-1", 3, "activate-from-provisioning").unwrap();
        let b = derive_step_key("uid-1", 3, "activate-from-provisioning").unwrap();
        assert_eq!(a, b, "same step must rederive the same key");

        let mut seen = std::collections::BTreeSet::new();
        for uid in ["uid-1", "uid-2"] {
            for generation in 0..8i64 {
                for step in ["activate", "suspend", "resume", "retire", "create"] {
                    let key = derive_step_key(uid, generation, step).unwrap();
                    assert!(
                        seen.insert(key.as_str().to_owned()),
                        "collision at {uid}/{generation}/{step}"
                    );
                }
            }
        }
    }

    #[test]
    fn step_keys_are_canonical_uuids() {
        let key = derive_step_key("c7a9f9a2", 41, "retire-from-active").unwrap();
        let text = key.as_str();
        assert_eq!(text.len(), 36);
        assert_eq!(&text[14..15], "4", "version nibble");
        assert_eq!(&text[19..20], "8", "variant nibble");
        let shifted = derive_step_key("c7a9f9a21", 41, "etire-from-active").unwrap();
        assert_ne!(
            key, shifted,
            "field-boundary domain separation: shifting a byte between fields \
must change the key"
        );
    }
}
