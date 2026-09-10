mod delete;
mod fixture;
mod ledger;
mod ledger_match;
mod listing;
mod write;

pub use delete::check_async_delete_operation;
pub use fixture::{ConformanceFixture, ConformanceViolation};
pub use ledger::check_operation_ledger_semantics;
pub use listing::check_stable_pagination;
pub use write::{check_create_idempotency, check_idempotent_put, check_read_after_write};

pub const MAX_OPERATION_POLLS: u32 = 32;
pub const MAX_PAGE_WALK: u32 = 100;

fn violation(check: &'static str, detail: impl Into<String>) -> ConformanceViolation {
    ConformanceViolation {
        check,
        detail: detail.into(),
    }
}

pub async fn run_all_checks<F: ConformanceFixture>(fixture: &F) -> Vec<ConformanceViolation> {
    [
        check_idempotent_put(fixture).await,
        check_create_idempotency(fixture).await,
        check_read_after_write(fixture).await,
        check_stable_pagination(fixture).await,
        check_async_delete_operation(fixture).await,
        check_operation_ledger_semantics(fixture).await,
    ]
    .into_iter()
    .filter_map(Result::err)
    .collect()
}
