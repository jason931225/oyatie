//! `PackEpoch` coverage checks: which signing key was authorized to sign
//! which period.
// ADR-0083 Tier 3: the `cfg(test)` unwrap/expect/panic exemption.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use audit_sealing_kernel::{PackEpoch, SigningKeyRef};

use crate::SealingDomainError;

/// Verify that `signing_key` was authorized, under `epoch`, to sign
/// `period_id` for `(pack, tenant_partition)`.
///
/// `period_id`, `period_lo` and `period_hi` are compared with plain
/// byte-lexicographic ordering: callers MUST encode periods so lexicographic
/// order matches chronological order (e.g. zero-padded ISO-8601). This function
/// never parses the encoding.
///
/// A retiring key is bounded by the SAME `[period_lo, period_hi)` window as the
/// active key — its grace never extends past the epoch that names it retiring.
///
/// # Errors
/// [`SealingDomainError`] — pack mismatch, tenant-partition mismatch, a key in
/// neither epoch slot, or a period outside the window (checked in that order).
pub fn verify_epoch_covers_period(
    epoch: &PackEpoch,
    pack: &str,
    tenant_partition: &str,
    period_id: &str,
    signing_key: &SigningKeyRef,
) -> Result<(), SealingDomainError> {
    if epoch.pack != pack {
        return Err(SealingDomainError::EpochPackMismatch {
            epoch_pack: epoch.pack.clone(),
            record_pack: pack.to_string(),
        });
    }
    if epoch.tenant_partition != tenant_partition {
        return Err(SealingDomainError::EpochTenantPartitionMismatch {
            epoch_tenant_partition: epoch.tenant_partition.clone(),
            record_tenant_partition: tenant_partition.to_string(),
        });
    }
    let in_window = period_id >= epoch.period_lo.as_str() && period_id < epoch.period_hi.as_str();
    if *signing_key == epoch.active_key {
        return if in_window {
            Ok(())
        } else {
            Err(SealingDomainError::PeriodOutsideEpochWindow {
                period: period_id.to_string(),
                period_lo: epoch.period_lo.clone(),
                period_hi: epoch.period_hi.clone(),
            })
        };
    }
    if epoch.retiring_key.as_ref() == Some(signing_key) {
        return if in_window {
            Ok(())
        } else {
            Err(SealingDomainError::RetiringKeyOutsideEpochWindow {
                key_id: signing_key.key_id.clone(),
                period: period_id.to_string(),
                period_lo: epoch.period_lo.clone(),
                period_hi: epoch.period_hi.clone(),
            })
        };
    }
    Err(SealingDomainError::SigningKeyNotInEpoch {
        key_id: signing_key.key_id.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(id: &str) -> SigningKeyRef {
        SigningKeyRef {
            key_id: id.to_string(),
        }
    }

    fn epoch_with_retiring() -> PackEpoch {
        PackEpoch {
            pack: "pack-alpha".to_string(),
            tenant_partition: "tenant-1".to_string(),
            period_lo: "2026-08-01".to_string(),
            period_hi: "2026-09-01".to_string(),
            active_key: key("active-1"),
            retiring_key: Some(key("retiring-1")),
        }
    }

    #[test]
    fn active_key_inside_window_is_covered() {
        let epoch = epoch_with_retiring();
        assert_eq!(
            verify_epoch_covers_period(
                &epoch,
                "pack-alpha",
                "tenant-1",
                "2026-08-15",
                &key("active-1"),
            ),
            Ok(())
        );
    }

    #[test]
    fn window_is_inclusive_of_period_lo() {
        let epoch = epoch_with_retiring();
        assert_eq!(
            verify_epoch_covers_period(
                &epoch,
                "pack-alpha",
                "tenant-1",
                "2026-08-01",
                &key("active-1"),
            ),
            Ok(())
        );
    }

    #[test]
    fn window_is_exclusive_of_period_hi() {
        let epoch = epoch_with_retiring();
        assert_eq!(
            verify_epoch_covers_period(
                &epoch,
                "pack-alpha",
                "tenant-1",
                "2026-09-01",
                &key("active-1"),
            ),
            Err(SealingDomainError::PeriodOutsideEpochWindow {
                period: "2026-09-01".to_string(),
                period_lo: "2026-08-01".to_string(),
                period_hi: "2026-09-01".to_string(),
            })
        );
    }

    #[test]
    fn active_key_before_window_is_rejected() {
        let epoch = epoch_with_retiring();
        assert_eq!(
            verify_epoch_covers_period(
                &epoch,
                "pack-alpha",
                "tenant-1",
                "2026-07-31",
                &key("active-1"),
            ),
            Err(SealingDomainError::PeriodOutsideEpochWindow {
                period: "2026-07-31".to_string(),
                period_lo: "2026-08-01".to_string(),
                period_hi: "2026-09-01".to_string(),
            })
        );
    }

    #[test]
    fn retiring_key_inside_window_is_covered() {
        let epoch = epoch_with_retiring();
        assert_eq!(
            verify_epoch_covers_period(
                &epoch,
                "pack-alpha",
                "tenant-1",
                "2026-08-20",
                &key("retiring-1"),
            ),
            Ok(())
        );
    }

    #[test]
    fn retiring_key_outside_window_is_rejected() {
        let epoch = epoch_with_retiring();
        assert_eq!(
            verify_epoch_covers_period(
                &epoch,
                "pack-alpha",
                "tenant-1",
                "2026-09-15",
                &key("retiring-1"),
            ),
            Err(SealingDomainError::RetiringKeyOutsideEpochWindow {
                key_id: "retiring-1".to_string(),
                period: "2026-09-15".to_string(),
                period_lo: "2026-08-01".to_string(),
                period_hi: "2026-09-01".to_string(),
            })
        );
    }

    #[test]
    fn no_retiring_key_configured_rejects_a_key_that_is_not_active() {
        let mut epoch = epoch_with_retiring();
        epoch.retiring_key = None;
        assert_eq!(
            verify_epoch_covers_period(
                &epoch,
                "pack-alpha",
                "tenant-1",
                "2026-08-15",
                &key("retiring-1"),
            ),
            Err(SealingDomainError::SigningKeyNotInEpoch {
                key_id: "retiring-1".to_string(),
            })
        );
    }

    #[test]
    fn unknown_key_is_rejected() {
        let epoch = epoch_with_retiring();
        assert_eq!(
            verify_epoch_covers_period(
                &epoch,
                "pack-alpha",
                "tenant-1",
                "2026-08-15",
                &key("stranger"),
            ),
            Err(SealingDomainError::SigningKeyNotInEpoch {
                key_id: "stranger".to_string(),
            })
        );
    }

    #[test]
    fn different_pack_never_covers_even_inside_window() {
        let epoch = epoch_with_retiring();
        assert_eq!(
            verify_epoch_covers_period(
                &epoch,
                "pack-other",
                "tenant-1",
                "2026-08-15",
                &key("active-1"),
            ),
            Err(SealingDomainError::EpochPackMismatch {
                epoch_pack: "pack-alpha".to_string(),
                record_pack: "pack-other".to_string(),
            })
        );
    }

    #[test]
    fn different_tenant_partition_never_covers_even_inside_window() {
        let epoch = epoch_with_retiring();
        assert_eq!(
            verify_epoch_covers_period(
                &epoch,
                "pack-alpha",
                "tenant-other",
                "2026-08-15",
                &key("active-1"),
            ),
            Err(SealingDomainError::EpochTenantPartitionMismatch {
                epoch_tenant_partition: "tenant-1".to_string(),
                record_tenant_partition: "tenant-other".to_string(),
            })
        );
    }

    #[test]
    fn pack_mismatch_is_reported_before_key_mismatch() {
        let epoch = epoch_with_retiring();
        assert_eq!(
            verify_epoch_covers_period(
                &epoch,
                "pack-other",
                "tenant-1",
                "2026-08-15",
                &key("stranger"),
            ),
            Err(SealingDomainError::EpochPackMismatch {
                epoch_pack: "pack-alpha".to_string(),
                record_pack: "pack-other".to_string(),
            })
        );
    }
}
