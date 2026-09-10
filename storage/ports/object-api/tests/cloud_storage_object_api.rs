#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "cloud_storage_object_api/common.rs"]
mod common;
#[path = "cloud_storage_object_api/get_contract.rs"]
mod get_contract;
#[path = "cloud_storage_object_api/idempotency_ledger_contract.rs"]
mod idempotency_ledger_contract;
#[path = "cloud_storage_object_api/idempotency_replay_contract.rs"]
mod idempotency_replay_contract;
#[path = "cloud_storage_object_api/put_contract.rs"]
mod put_contract;
#[path = "cloud_storage_object_api/surface_contract.rs"]
mod surface_contract;
