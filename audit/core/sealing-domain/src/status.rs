#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        reason = "assertion failure IS the test signal; ADR-0083 Tier 3 cfg(test) exemption"
    )
)]

use audit_sealing_kernel::{SealRecord, SealStatus};

use crate::SealingDomainError;

/// The complete, closed set of legal `(from, to)` edges of the `SealStatus`
/// lifecycle. Anything not listed here is illegal.
const LEGAL_EDGES: &[(SealStatus, SealStatus)] = &[
    (SealStatus::Accepted, SealStatus::Unsealed),
    (SealStatus::Unsealed, SealStatus::Sealed),
    (SealStatus::Sealed, SealStatus::Published),
    (SealStatus::Published, SealStatus::Verified),
    (SealStatus::Verified, SealStatus::Redacted),
    (SealStatus::Verified, SealStatus::Retained),
];

/// # Errors
/// [`SealingDomainError::IllegalSealStatusTransition`] for any `(from, to)`
/// pair not in [`LEGAL_EDGES`].
pub fn transition_seal_status(
    from: SealStatus,
    to: SealStatus,
) -> Result<SealStatus, SealingDomainError> {
    if LEGAL_EDGES.contains(&(from, to)) {
        Ok(to)
    } else {
        Err(SealingDomainError::IllegalSealStatusTransition { from, to })
    }
}

/// # Errors
/// [`SealingDomainError::IllegalSealStatusTransition`] when `to` is not a
/// legal successor of `record.status` — see [`transition_seal_status`].
pub fn apply_seal_status_transition(
    record: &SealRecord,
    to: SealStatus,
) -> Result<SealRecord, SealingDomainError> {
    transition_seal_status(record.status, to)?;
    Ok(SealRecord {
        status: to,
        ..record.clone()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_STATUSES: [SealStatus; 7] = [
        SealStatus::Accepted,
        SealStatus::Unsealed,
        SealStatus::Sealed,
        SealStatus::Published,
        SealStatus::Verified,
        SealStatus::Redacted,
        SealStatus::Retained,
    ];

    mod legal_edges {
        use super::*;

        #[test]
        fn accepted_to_unsealed_is_legal() {
            assert_eq!(
                transition_seal_status(SealStatus::Accepted, SealStatus::Unsealed),
                Ok(SealStatus::Unsealed)
            );
        }

        #[test]
        fn unsealed_to_sealed_is_legal() {
            assert_eq!(
                transition_seal_status(SealStatus::Unsealed, SealStatus::Sealed),
                Ok(SealStatus::Sealed)
            );
        }

        #[test]
        fn sealed_to_published_is_legal() {
            assert_eq!(
                transition_seal_status(SealStatus::Sealed, SealStatus::Published),
                Ok(SealStatus::Published)
            );
        }

        #[test]
        fn published_to_verified_is_legal() {
            assert_eq!(
                transition_seal_status(SealStatus::Published, SealStatus::Verified),
                Ok(SealStatus::Verified)
            );
        }

        #[test]
        fn verified_to_redacted_is_legal() {
            assert_eq!(
                transition_seal_status(SealStatus::Verified, SealStatus::Redacted),
                Ok(SealStatus::Redacted)
            );
        }

        #[test]
        fn verified_to_retained_is_legal() {
            assert_eq!(
                transition_seal_status(SealStatus::Verified, SealStatus::Retained),
                Ok(SealStatus::Retained)
            );
        }
    }

    mod illegal_transitions {
        use super::*;

        #[test]
        fn rejects_backward_move_sealed_to_unsealed() {
            assert_eq!(
                transition_seal_status(SealStatus::Sealed, SealStatus::Unsealed),
                Err(SealingDomainError::IllegalSealStatusTransition {
                    from: SealStatus::Sealed,
                    to: SealStatus::Unsealed,
                })
            );
        }

        #[test]
        fn rejects_backward_move_verified_to_published() {
            assert_eq!(
                transition_seal_status(SealStatus::Verified, SealStatus::Published),
                Err(SealingDomainError::IllegalSealStatusTransition {
                    from: SealStatus::Verified,
                    to: SealStatus::Published,
                })
            );
        }

        #[test]
        fn rejects_skipped_stage_accepted_to_sealed() {
            assert_eq!(
                transition_seal_status(SealStatus::Accepted, SealStatus::Sealed),
                Err(SealingDomainError::IllegalSealStatusTransition {
                    from: SealStatus::Accepted,
                    to: SealStatus::Sealed,
                })
            );
        }

        #[test]
        fn rejects_skipped_stage_accepted_to_verified() {
            assert_eq!(
                transition_seal_status(SealStatus::Accepted, SealStatus::Verified),
                Err(SealingDomainError::IllegalSealStatusTransition {
                    from: SealStatus::Accepted,
                    to: SealStatus::Verified,
                })
            );
        }

        #[test]
        fn rejects_self_transition() {
            assert_eq!(
                transition_seal_status(SealStatus::Sealed, SealStatus::Sealed),
                Err(SealingDomainError::IllegalSealStatusTransition {
                    from: SealStatus::Sealed,
                    to: SealStatus::Sealed,
                })
            );
        }

        #[test]
        fn rejects_exit_from_redacted() {
            assert_eq!(
                transition_seal_status(SealStatus::Redacted, SealStatus::Retained),
                Err(SealingDomainError::IllegalSealStatusTransition {
                    from: SealStatus::Redacted,
                    to: SealStatus::Retained,
                })
            );
        }

        #[test]
        fn rejects_exit_from_retained() {
            assert_eq!(
                transition_seal_status(SealStatus::Retained, SealStatus::Verified),
                Err(SealingDomainError::IllegalSealStatusTransition {
                    from: SealStatus::Retained,
                    to: SealStatus::Verified,
                })
            );
        }

        #[test]
        fn rejects_jump_straight_to_redacted_from_sealed() {
            assert_eq!(
                transition_seal_status(SealStatus::Sealed, SealStatus::Redacted),
                Err(SealingDomainError::IllegalSealStatusTransition {
                    from: SealStatus::Sealed,
                    to: SealStatus::Redacted,
                })
            );
        }
    }

    mod exhaustive_matrix {
        use super::*;

        #[test]
        fn exactly_the_six_declared_edges_are_legal() {
            let mut legal_count = 0;
            for &from in &ALL_STATUSES {
                for &to in &ALL_STATUSES {
                    let result = transition_seal_status(from, to);
                    let is_legal = LEGAL_EDGES.contains(&(from, to));
                    assert_eq!(
                        result.is_ok(),
                        is_legal,
                        "transition_seal_status({from:?}, {to:?}) legality mismatch"
                    );
                    if !is_legal {
                        assert_eq!(
                            result,
                            Err(SealingDomainError::IllegalSealStatusTransition { from, to })
                        );
                    } else {
                        legal_count += 1;
                    }
                }
            }
            assert_eq!(legal_count, LEGAL_EDGES.len());
            assert_eq!(legal_count, 6);
        }
    }

    mod apply_transition {
        use super::*;

        fn sample_record(status: SealStatus) -> SealRecord {
            SealRecord {
                pack: "pack-alpha".to_string(),
                tenant_partition: "tenant-1".to_string(),
                period_id: "2026-08".to_string(),
                leaf_count: 3,
                merkle_root: "sha256:aa".to_string(),
                prior_root: None,
                signing_key: audit_sealing_kernel::SigningKeyRef {
                    key_id: "key-1".to_string(),
                },
                status,
            }
        }

        #[test]
        fn apply_transition_updates_status_and_preserves_other_fields() {
            let record = sample_record(SealStatus::Sealed);
            let updated = apply_seal_status_transition(&record, SealStatus::Published)
                .expect("Sealed -> Published is legal");
            assert_eq!(updated.status, SealStatus::Published);
            assert_eq!(updated.pack, record.pack);
            assert_eq!(updated.merkle_root, record.merkle_root);
        }

        #[test]
        fn apply_transition_rejects_illegal_move_without_mutating() {
            let record = sample_record(SealStatus::Accepted);
            assert_eq!(
                apply_seal_status_transition(&record, SealStatus::Verified),
                Err(SealingDomainError::IllegalSealStatusTransition {
                    from: SealStatus::Accepted,
                    to: SealStatus::Verified,
                })
            );
        }
    }
}
