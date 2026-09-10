#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use data_boundary_kernel::DataClass;
use hr_employment_domain::{
    HrDomainError, LeaveBalanceAccrualInput, evaluate_leave_balance_accrual,
};

include!(concat!(env!("OUT_DIR"), "/leave_balance.generated.rs"));
