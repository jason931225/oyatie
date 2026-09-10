#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use data_boundary_kernel::DataClass;
use hr_employment_domain::{
    HrDomainError, LeaveCarryoverForfeitureInput, evaluate_leave_carryover_forfeiture,
};

include!(concat!(
    env!("OUT_DIR"),
    "/leave_carryover_forfeiture.generated.rs"
));
