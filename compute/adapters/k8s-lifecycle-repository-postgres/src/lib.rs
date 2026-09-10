#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

mod acceptance;
mod acceptance_integrity;
mod acceptance_storage;
mod canonical_json;
mod catalog_connection;
mod error;
mod expression_dependencies;
mod integrity;
mod migrations;
mod migrator;
mod operation;
mod repository;
mod rls_guard;
mod role_database_claim;
mod runtime_contract;
mod runtime_guard;
mod schema;
mod schema_catalog;
mod schema_contract;
mod schema_phase;
mod serving_role_guard;

use error::validate_database_url;
pub use error::{
    PgK8sLifecycleConnectError, PgK8sLifecycleMigrationError, PgK8sLifecycleRoleDatabaseClaimError,
    PgK8sLifecycleSchemaError,
};
pub use migrations::{
    CURRENT_MIGRATION_VERSION, K8S_LIFECYCLE_MIGRATIONS, K8S_LIFECYCLE_PENDING_INTENT_MIGRATION,
    K8S_LIFECYCLE_REPOSITORY_MIGRATION, K8S_LIFECYCLE_RUNTIME_ROLE_MIGRATION, MIGRATIONS_TABLE,
    PgK8sLifecycleMigration,
};
pub use migrator::{PgK8sLifecycleMigrationReport, PgK8sLifecycleMigrator};
pub use runtime_contract::{PgK8sLifecycleRuntimeContract, PgK8sLifecycleRuntimeContractError};
use sqlx::{PgPool, postgres::PgPoolOptions};

pub const SCHEMA_NAME: &str = "compute_k8s_lifecycle";
pub const CLUSTERS_TABLE: &str = "compute_k8s_lifecycle.clusters";
pub const OPERATIONS_TABLE: &str = "compute_k8s_lifecycle.operations";
pub const GOVERNED_TABLES: &[&str] = &[CLUSTERS_TABLE, OPERATIONS_TABLE];
pub const RUNTIME_ROLE: &str = "compute_k8s_lifecycle_runtime";
pub const SCHEMA_VERSION: i32 = 1;

#[derive(Clone, Debug)]
pub struct PgK8sLifecycleRepository {
    pool: PgPool,
}

impl PgK8sLifecycleRepository {
    pub async fn connect(
        database_url: &str,
        runtime_contract: &PgK8sLifecycleRuntimeContract,
    ) -> Result<Self, PgK8sLifecycleConnectError> {
        validate_database_url(database_url)?;
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .connect(database_url)
            .await
            .map_err(|error| PgK8sLifecycleConnectError::Sqlx(error.to_string()))?;
        Self::from_pool(pool, runtime_contract).await
    }

    pub async fn from_pool(
        pool: PgPool,
        runtime_contract: &PgK8sLifecycleRuntimeContract,
    ) -> Result<Self, PgK8sLifecycleConnectError> {
        runtime_guard::attest_runtime(&pool, runtime_contract).await?;
        let repository = Self { pool };
        repository.assert_rls_enforceable().await?;
        Ok(repository)
    }

    pub async fn assert_rls_enforceable(&self) -> Result<(), PgK8sLifecycleConnectError> {
        rls_guard::attest_rls(&self.pool)
            .await
            .map_err(PgK8sLifecycleConnectError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operation::{
        COMPLETE_OPERATION_SQL, INSERT_CLUSTER_SQL, RESERVE_OPERATION_SQL,
        SELECT_CLUSTER_FOR_UPDATE_SQL, SELECT_OPERATION_FOR_UPDATE_SQL, UPDATE_CLUSTER_SQL,
    };
    use compute_k8s_api::CLOUD_COMPUTE_K8S_CLUSTER_RECORD_SCHEMA_VERSION;
    use shared_postgres_command_kernel::{SET_LOCAL_TENANT_SQL, force_rls_tables};

    #[test]
    fn connect_rejects_blank_database_url() {
        assert_eq!(
            validate_database_url("   "),
            Err(PgK8sLifecycleConnectError::MissingDatabaseUrl)
        );
    }

    #[test]
    fn operation_reservation_serializes_replicas_without_overwriting_first_receipt() {
        assert!(RESERVE_OPERATION_SQL.contains("ON CONFLICT"));
        assert!(RESERVE_OPERATION_SQL.contains("DO NOTHING"));
        assert!(SELECT_OPERATION_FOR_UPDATE_SQL.contains("FOR UPDATE"));
        assert!(SELECT_CLUSTER_FOR_UPDATE_SQL.contains("FOR UPDATE"));
        assert!(!COMPLETE_OPERATION_SQL.contains("INSERT"));
    }

    #[test]
    fn every_data_statement_is_explicitly_tenant_scoped() {
        for statement in [
            RESERVE_OPERATION_SQL,
            SELECT_OPERATION_FOR_UPDATE_SQL,
            COMPLETE_OPERATION_SQL,
            INSERT_CLUSTER_SQL,
            SELECT_CLUSTER_FOR_UPDATE_SQL,
            UPDATE_CLUSTER_SQL,
        ] {
            assert!(statement.contains("tenant_id"), "{statement}");
        }
        assert_eq!(
            SET_LOCAL_TENANT_SQL,
            "SELECT set_config('oyatie.tenant_id', $1, true)"
        );
    }

    #[test]
    fn governed_tables_exactly_match_force_rls_migration() {
        let mut forced = force_rls_tables(K8S_LIFECYCLE_REPOSITORY_MIGRATION);
        forced.sort();
        let mut governed: Vec<String> = GOVERNED_TABLES
            .iter()
            .map(|table| (*table).to_string())
            .collect();
        governed.sort();
        assert_eq!(governed, forced);
    }

    #[test]
    fn runtime_role_is_shared_by_role_and_policy_migrations() {
        assert!(K8S_LIFECYCLE_RUNTIME_ROLE_MIGRATION.contains(RUNTIME_ROLE));
        assert!(K8S_LIFECYCLE_REPOSITORY_MIGRATION.contains(&format!("TO {RUNTIME_ROLE}")));
    }

    #[test]
    fn migrations_bind_native_projection_and_completed_receipt_digest() {
        for constraint in [
            "clusters_region_matches_desired",
            "clusters_flavor_matches_desired",
            "clusters_version_matches_desired",
            "clusters_privacy_matches_desired",
            "clusters_node_count_matches_desired",
            "clusters_residency_matches_desired",
            "clusters_data_class_matches_desired",
            "clusters_created_at_matches_desired",
            "clusters_record_schema_version",
            "operations_receipt_atomicity",
            "operations_receipt_digest_shape",
        ] {
            assert!(
                K8S_LIFECYCLE_REPOSITORY_MIGRATION.contains(constraint),
                "missing {constraint}"
            );
        }
        assert!(K8S_LIFECYCLE_REPOSITORY_MIGRATION.contains(&format!(
            "cluster_json -> 'schema_version' = '{CLOUD_COMPUTE_K8S_CLUSTER_RECORD_SCHEMA_VERSION}'::jsonb"
        )));
        assert!(COMPLETE_OPERATION_SQL.contains("receipt_digest = $7"));
    }
}
