//! Cloud capacity-management kernel.

pub mod cell_budget;
pub use cell_budget::{
    CellBudget, CellBudgetError, CellId, CellReservationRequest, admit_cell_reservation,
};

pub mod committed_use;
pub use committed_use::{
    CommittedUseContract, CommittedUseContractId, CommittedUseError, ReservationTerm,
    ReservedCapacity, ReservedCapacityError, ReservedCapacityId, SpotPool, SpotPoolError,
    SpotPoolId, admit_spot_request, amortized_monthly_commit_micros, committed_coverage_bps,
    effective_discounted_rate, validate_committed_use_contract, validate_reserved_capacity,
    validate_spot_pool,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum CapacityClass {
    Cpu,
    Memory,
    Disk,
    Gpu,
}

impl CapacityClass {
    pub fn name(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Memory => "memory",
            Self::Disk => "disk",
            Self::Gpu => "gpu",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegionId(pub String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReservationId(pub String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapacityQuota {
    pub region: RegionId,
    pub class: CapacityClass,
    pub limit_units: u64,
    pub used_units: u64,
}

impl CapacityQuota {
    pub fn available(&self) -> u64 {
        self.limit_units.saturating_sub(self.used_units)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reservation {
    pub id: ReservationId,
    pub region: RegionId,
    pub class: CapacityClass,
    pub units: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CapacityError {
    EmptyReservationId,
    EmptyRegionId,
    ZeroUnits,
    QuotaExceeded { available: u64, requested: u64 },
    RegionUnknown { region: String },
}

impl CapacityError {
    pub fn message(&self) -> String {
        match self {
            Self::EmptyReservationId => "reservation id is empty".to_owned(),
            Self::EmptyRegionId => "region id is empty".to_owned(),
            Self::ZeroUnits => "reservation requests zero units".to_owned(),
            Self::QuotaExceeded {
                available,
                requested,
            } => {
                format!("quota exceeded: requested={requested} available={available}")
            }
            Self::RegionUnknown { region } => format!("unknown region: {region}"),
        }
    }
}

pub fn admit_reservation(
    reservation: &Reservation,
    quotas: &[CapacityQuota],
) -> Result<(), CapacityError> {
    if reservation.id.0.is_empty() {
        return Err(CapacityError::EmptyReservationId);
    }
    if reservation.region.0.is_empty() {
        return Err(CapacityError::EmptyRegionId);
    }
    if reservation.units == 0 {
        return Err(CapacityError::ZeroUnits);
    }
    let q = quotas
        .iter()
        .find(|q| q.region == reservation.region && q.class == reservation.class)
        .ok_or_else(|| CapacityError::RegionUnknown {
            region: reservation.region.0.clone(),
        })?;
    if reservation.units > q.available() {
        return Err(CapacityError::QuotaExceeded {
            available: q.available(),
            requested: reservation.units,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(region: &str, class: CapacityClass, limit: u64, used: u64) -> CapacityQuota {
        CapacityQuota {
            region: RegionId(region.into()),
            class,
            limit_units: limit,
            used_units: used,
        }
    }
    fn r(id: &str, region: &str, class: CapacityClass, units: u64) -> Reservation {
        Reservation {
            id: ReservationId(id.into()),
            region: RegionId(region.into()),
            class,
            units,
        }
    }

    #[test]
    fn class_names_distinct() {
        let names: std::collections::HashSet<_> = [
            CapacityClass::Cpu,
            CapacityClass::Memory,
            CapacityClass::Disk,
            CapacityClass::Gpu,
        ]
        .iter()
        .map(|c| c.name())
        .collect();
        assert_eq!(names.len(), 4);
    }

    #[test]
    fn quota_available_saturates() {
        assert_eq!(q("kr1", CapacityClass::Cpu, 100, 30).available(), 70);
        assert_eq!(q("kr1", CapacityClass::Cpu, 10, 50).available(), 0);
    }

    #[test]
    fn admit_within_quota_passes() {
        let r = r("rsv-1", "kr1", CapacityClass::Cpu, 50);
        let qs = vec![q("kr1", CapacityClass::Cpu, 100, 30)];
        assert!(admit_reservation(&r, &qs).is_ok());
    }

    #[test]
    fn admit_at_quota_boundary_passes() {
        let r = r("rsv-1", "kr1", CapacityClass::Cpu, 70);
        let qs = vec![q("kr1", CapacityClass::Cpu, 100, 30)];
        assert!(admit_reservation(&r, &qs).is_ok());
    }

    #[test]
    fn admit_over_quota_rejected() {
        let r = r("rsv-1", "kr1", CapacityClass::Cpu, 71);
        let qs = vec![q("kr1", CapacityClass::Cpu, 100, 30)];
        assert!(matches!(
            admit_reservation(&r, &qs),
            Err(CapacityError::QuotaExceeded { .. })
        ));
    }

    #[test]
    fn admit_unknown_region_rejected() {
        let r = r("rsv-1", "unknown", CapacityClass::Cpu, 5);
        let qs = vec![q("kr1", CapacityClass::Cpu, 100, 30)];
        assert!(matches!(
            admit_reservation(&r, &qs),
            Err(CapacityError::RegionUnknown { .. })
        ));
    }

    #[test]
    fn empty_id_rejected() {
        let r = r("", "kr1", CapacityClass::Cpu, 5);
        let qs = vec![q("kr1", CapacityClass::Cpu, 100, 30)];
        assert!(matches!(
            admit_reservation(&r, &qs),
            Err(CapacityError::EmptyReservationId)
        ));
    }

    #[test]
    fn zero_units_rejected() {
        let r = r("rsv-1", "kr1", CapacityClass::Cpu, 0);
        let qs = vec![q("kr1", CapacityClass::Cpu, 100, 30)];
        assert!(matches!(
            admit_reservation(&r, &qs),
            Err(CapacityError::ZeroUnits)
        ));
    }

    #[test]
    fn class_isolation_does_not_cross_count() {
        let r = r("rsv-gpu", "kr1", CapacityClass::Gpu, 1);
        let qs = vec![
            q("kr1", CapacityClass::Cpu, 100, 100),
            q("kr1", CapacityClass::Gpu, 8, 4),
        ];
        assert!(admit_reservation(&r, &qs).is_ok());
    }

    #[test]
    fn empty_region_id_rejected() {
        let r = r("rsv-1", "", CapacityClass::Cpu, 5);
        let qs = vec![q("kr1", CapacityClass::Cpu, 100, 30)];
        assert!(matches!(
            admit_reservation(&r, &qs),
            Err(CapacityError::EmptyRegionId)
        ));
    }
}
