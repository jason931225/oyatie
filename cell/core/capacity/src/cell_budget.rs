//! Cell-level capacity budget.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use crate::{CapacityClass, ReservationId};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct CellId(pub String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellBudget {
    pub cell_id: CellId,
    pub class: CapacityClass,
    pub max_reservations: u32,
    pub active_reservation_count: u32,
}

impl CellBudget {
    pub fn remaining(&self) -> u32 {
        self.max_reservations
            .saturating_sub(self.active_reservation_count)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellReservationRequest {
    pub reservation_id: ReservationId,
    pub cell_id: CellId,
    pub class: CapacityClass,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CellBudgetError {
    EmptyReservationId,
    EmptyCellId,
    BudgetNotFound {
        cell_id: String,
        class: CapacityClass,
    },
    BudgetExceeded {
        max: u32,
        active: u32,
    },
}

impl CellBudgetError {
    pub fn message(&self) -> String {
        match self {
            Self::EmptyReservationId => "reservation id must be non-empty".to_owned(),
            Self::EmptyCellId => "cell id must be non-empty".to_owned(),
            Self::BudgetNotFound { cell_id, class } => {
                format!(
                    "no cell budget declared for cell={cell_id} class={}",
                    class.name()
                )
            }
            Self::BudgetExceeded { max, active } => {
                format!("cell budget exceeded: max={max} active={active}")
            }
        }
    }
}

/// Does **not** mutate the budgets — callers record the committed
/// reservation separately.
pub fn admit_cell_reservation(
    req: &CellReservationRequest,
    budgets: &[CellBudget],
) -> Result<(), CellBudgetError> {
    if req.reservation_id.0.is_empty() {
        return Err(CellBudgetError::EmptyReservationId);
    }
    if req.cell_id.0.is_empty() {
        return Err(CellBudgetError::EmptyCellId);
    }
    let budget = budgets
        .iter()
        .find(|b| b.cell_id == req.cell_id && b.class == req.class)
        .ok_or_else(|| CellBudgetError::BudgetNotFound {
            cell_id: req.cell_id.0.clone(),
            class: req.class,
        })?;
    if budget.remaining() == 0 {
        return Err(CellBudgetError::BudgetExceeded {
            max: budget.max_reservations,
            active: budget.active_reservation_count,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CapacityClass, ReservationId};

    fn budget(cell: &str, class: CapacityClass, max: u32, active: u32) -> CellBudget {
        CellBudget {
            cell_id: CellId(cell.into()),
            class,
            max_reservations: max,
            active_reservation_count: active,
        }
    }

    fn req(rsv: &str, cell: &str, class: CapacityClass) -> CellReservationRequest {
        CellReservationRequest {
            reservation_id: ReservationId(rsv.into()),
            cell_id: CellId(cell.into()),
            class,
        }
    }

    #[test]
    fn remaining_saturates_at_zero() {
        let b = budget("cell-kr1-a-01", CapacityClass::Cpu, 10, 50);
        assert_eq!(b.remaining(), 0);
    }

    #[test]
    fn remaining_reflects_slack() {
        let b = budget("cell-kr1-a-01", CapacityClass::Cpu, 10, 3);
        assert_eq!(b.remaining(), 7);
    }

    #[test]
    fn admit_within_budget_passes() {
        let r = req("rsv-1", "cell-kr1-a-01", CapacityClass::Cpu);
        let bs = vec![budget("cell-kr1-a-01", CapacityClass::Cpu, 10, 3)];
        assert!(admit_cell_reservation(&r, &bs).is_ok());
    }

    #[test]
    fn admit_at_boundary_passes() {
        let r = req("rsv-1", "cell-kr1-a-01", CapacityClass::Cpu);
        let bs = vec![budget("cell-kr1-a-01", CapacityClass::Cpu, 10, 9)];
        assert!(admit_cell_reservation(&r, &bs).is_ok());
    }

    #[test]
    fn admit_at_full_budget_rejected() {
        let r = req("rsv-1", "cell-kr1-a-01", CapacityClass::Cpu);
        let bs = vec![budget("cell-kr1-a-01", CapacityClass::Cpu, 10, 10)];
        assert!(matches!(
            admit_cell_reservation(&r, &bs),
            Err(CellBudgetError::BudgetExceeded { .. })
        ));
    }

    #[test]
    fn admit_unknown_cell_rejected() {
        let r = req("rsv-1", "cell-unknown", CapacityClass::Cpu);
        let bs = vec![budget("cell-kr1-a-01", CapacityClass::Cpu, 10, 3)];
        assert!(matches!(
            admit_cell_reservation(&r, &bs),
            Err(CellBudgetError::BudgetNotFound { .. })
        ));
    }

    #[test]
    fn class_isolation_respected() {
        let r = req("rsv-1", "cell-kr1-a-01", CapacityClass::Cpu);
        let bs = vec![budget("cell-kr1-a-01", CapacityClass::Gpu, 8, 2)];
        assert!(matches!(
            admit_cell_reservation(&r, &bs),
            Err(CellBudgetError::BudgetNotFound { .. })
        ));
    }

    #[test]
    fn empty_reservation_id_rejected() {
        let r = req("", "cell-kr1-a-01", CapacityClass::Cpu);
        let bs = vec![budget("cell-kr1-a-01", CapacityClass::Cpu, 10, 3)];
        assert!(matches!(
            admit_cell_reservation(&r, &bs),
            Err(CellBudgetError::EmptyReservationId)
        ));
    }

    #[test]
    fn empty_cell_id_rejected() {
        let r = req("rsv-1", "", CapacityClass::Cpu);
        let bs = vec![budget("cell-kr1-a-01", CapacityClass::Cpu, 10, 3)];
        assert!(matches!(
            admit_cell_reservation(&r, &bs),
            Err(CellBudgetError::EmptyCellId)
        ));
    }

    #[test]
    fn error_messages_non_empty() {
        let errors = [
            CellBudgetError::EmptyReservationId,
            CellBudgetError::EmptyCellId,
            CellBudgetError::BudgetNotFound {
                cell_id: "cell-x".into(),
                class: CapacityClass::Memory,
            },
            CellBudgetError::BudgetExceeded { max: 5, active: 5 },
        ];
        for e in &errors {
            assert!(!e.message().is_empty(), "empty message for {e:?}");
        }
    }
}
