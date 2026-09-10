use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const MIGRATIONS_TABLE: &str = "compute_k8s_lifecycle.schema_migrations";
pub const CURRENT_MIGRATION_VERSION: i64 = 3;

pub(crate) const MIGRATION_LOCK_KEY: i64 = 0x4f59_4154_4945_4b38;
pub(crate) const MIGRATION_LEDGER_BOOTSTRAP: &str = r#"CREATE SCHEMA IF NOT EXISTS compute_k8s_lifecycle;
CREATE TABLE IF NOT EXISTS compute_k8s_lifecycle.schema_migrations (
    version bigint NOT NULL,
    name text NOT NULL,
    sha256 text NOT NULL,
    applied_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT schema_migrations_primary_key PRIMARY KEY (version),
    CONSTRAINT schema_migrations_name_unique UNIQUE (name),
    CONSTRAINT schema_migrations_version_positive CHECK (version > 0),
    CONSTRAINT schema_migrations_name_not_empty CHECK (name <> ''),
    CONSTRAINT schema_migrations_sha256_shape CHECK (sha256 ~ '^[0-9a-f]{64}$')
);"#;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PgK8sLifecycleMigration {
    version: i64,
    name: &'static str,
    sql: &'static str,
    governed_table_count_after: i64,
}

impl PgK8sLifecycleMigration {
    #[must_use]
    pub const fn version(self) -> i64 {
        self.version
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }

    #[must_use]
    pub const fn sql(self) -> &'static str {
        self.sql
    }

    #[must_use]
    pub const fn governed_table_count_after(self) -> i64 {
        self.governed_table_count_after
    }

    #[must_use]
    pub fn sha256(self) -> String {
        format!("{:x}", Sha256::digest(self.sql.as_bytes()))
    }
}

pub const K8S_LIFECYCLE_RUNTIME_ROLE_MIGRATION: &str = r#"DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'compute_k8s_lifecycle_runtime') THEN
        CREATE ROLE compute_k8s_lifecycle_runtime
            NOSUPERUSER NOCREATEDB NOCREATEROLE INHERIT NOLOGIN
            NOREPLICATION NOBYPASSRLS CONNECTION LIMIT -1 PASSWORD NULL;
    END IF;
END
$$;
CREATE SCHEMA IF NOT EXISTS compute_k8s_lifecycle;
GRANT USAGE ON SCHEMA compute_k8s_lifecycle TO compute_k8s_lifecycle_runtime;
"#;

pub const K8S_LIFECYCLE_REPOSITORY_MIGRATION: &str = r#"CREATE SCHEMA IF NOT EXISTS compute_k8s_lifecycle;

CREATE TABLE IF NOT EXISTS compute_k8s_lifecycle.clusters (
    tenant_id text NOT NULL CHECK (tenant_id <> ''),
    resource_id text NOT NULL CHECK (resource_id <> ''),
    desired_spec_json jsonb NOT NULL,
    cluster_json jsonb NOT NULL,
    observed_state text NOT NULL CHECK (observed_state <> ''),
    desired_state text NOT NULL CHECK (desired_state <> ''),
    schema_version integer NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, resource_id),
    CONSTRAINT clusters_database_schema_version CHECK (schema_version = 1),
    CONSTRAINT clusters_json_objects CHECK (jsonb_typeof(desired_spec_json) = 'object' AND jsonb_typeof(cluster_json) = 'object'),
    CONSTRAINT clusters_desired_tenant_matches CHECK (desired_spec_json ? 'tenant_id' AND desired_spec_json ->> 'tenant_id' = tenant_id),
    CONSTRAINT clusters_desired_resource_matches CHECK (desired_spec_json ? 'resource_id' AND desired_spec_json ->> 'resource_id' = resource_id),
    CONSTRAINT clusters_record_tenant_matches CHECK (cluster_json ? 'tenant_id' AND cluster_json ->> 'tenant_id' = tenant_id),
    CONSTRAINT clusters_record_resource_matches CHECK (cluster_json ? 'resource_id' AND cluster_json ->> 'resource_id' = resource_id),
    CONSTRAINT clusters_region_matches_desired CHECK (desired_spec_json ? 'region' AND cluster_json ? 'region' AND cluster_json -> 'region' = desired_spec_json -> 'region'),
    CONSTRAINT clusters_flavor_matches_desired CHECK (desired_spec_json ? 'flavor' AND cluster_json ? 'flavor' AND cluster_json -> 'flavor' = desired_spec_json -> 'flavor'),
    CONSTRAINT clusters_version_matches_desired CHECK (desired_spec_json ? 'control_plane_version' AND cluster_json ? 'control_plane_version' AND cluster_json -> 'control_plane_version' = desired_spec_json -> 'control_plane_version'),
    CONSTRAINT clusters_privacy_matches_desired CHECK (desired_spec_json ? 'control_plane_private' AND cluster_json ? 'control_plane_private' AND cluster_json -> 'control_plane_private' = desired_spec_json -> 'control_plane_private'),
    CONSTRAINT clusters_node_count_matches_desired CHECK (desired_spec_json ? 'node_pools' AND jsonb_typeof(desired_spec_json -> 'node_pools') = 'array' AND cluster_json ? 'node_pool_count' AND cluster_json -> 'node_pool_count' = to_jsonb(jsonb_array_length(desired_spec_json -> 'node_pools'))),
    CONSTRAINT clusters_residency_matches_desired CHECK (desired_spec_json ? 'residency' AND cluster_json ? 'residency' AND cluster_json -> 'residency' = desired_spec_json -> 'residency'),
    CONSTRAINT clusters_data_class_matches_desired CHECK (desired_spec_json ? 'data_class' AND cluster_json ? 'data_class' AND cluster_json -> 'data_class' = desired_spec_json -> 'data_class'),
    CONSTRAINT clusters_created_at_matches_desired CHECK (desired_spec_json ? 'created_at_epoch_seconds' AND cluster_json ? 'created_at_epoch_seconds' AND cluster_json -> 'created_at_epoch_seconds' = desired_spec_json -> 'created_at_epoch_seconds'),
    CONSTRAINT clusters_record_schema_version CHECK (cluster_json ? 'schema_version' AND cluster_json -> 'schema_version' = '2'::jsonb),
    CONSTRAINT clusters_observed_state_matches CHECK (cluster_json ? 'state' AND cluster_json ->> 'state' = observed_state AND observed_state IN ('creating', 'ready', 'reconciling', 'draining', 'deleted')),
    CONSTRAINT clusters_desired_state_matches CHECK (cluster_json ? 'desired_state' AND cluster_json ->> 'desired_state' = desired_state AND desired_state IN ('present', 'deleted'))
);
CREATE INDEX IF NOT EXISTS clusters_reconciliation_scan
    ON compute_k8s_lifecycle.clusters (tenant_id, desired_state, updated_at, resource_id);
ALTER TABLE compute_k8s_lifecycle.clusters ENABLE ROW LEVEL SECURITY;
ALTER TABLE compute_k8s_lifecycle.clusters FORCE ROW LEVEL SECURITY;
CREATE POLICY clusters_tenant_isolation ON compute_k8s_lifecycle.clusters AS PERMISSIVE FOR ALL TO compute_k8s_lifecycle_runtime USING (tenant_id = current_setting('oyatie.tenant_id', true)) WITH CHECK (tenant_id = current_setting('oyatie.tenant_id', true));
CREATE POLICY clusters_require_tenant_guc ON compute_k8s_lifecycle.clusters AS RESTRICTIVE FOR ALL TO compute_k8s_lifecycle_runtime USING (current_setting('oyatie.tenant_id', true) IS NOT NULL AND current_setting('oyatie.tenant_id', true) <> '') WITH CHECK (current_setting('oyatie.tenant_id', true) IS NOT NULL AND current_setting('oyatie.tenant_id', true) <> '');

CREATE TABLE IF NOT EXISTS compute_k8s_lifecycle.operations (
    tenant_id text NOT NULL CHECK (tenant_id <> ''),
    principal_id text NOT NULL CHECK (principal_id <> ''),
    surface text NOT NULL CHECK (surface IN ('cloud.compute.k8s.cluster.create', 'cloud.compute.k8s.cluster.delete')),
    idempotency_key text NOT NULL CHECK (idempotency_key <> ''),
    resource_id text NOT NULL CHECK (resource_id <> ''),
    request_fingerprint text NOT NULL CHECK (request_fingerprint <> ''),
    receipt_kind text CHECK (receipt_kind IN ('create', 'delete')),
    receipt_json jsonb,
    receipt_digest text,
    schema_version integer NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    completed_at timestamptz,
    PRIMARY KEY (tenant_id, principal_id, surface, idempotency_key),
    CONSTRAINT operations_database_schema_version CHECK (schema_version = 1),
    CONSTRAINT operations_receipt_atomicity CHECK ((receipt_kind IS NULL) = (receipt_json IS NULL) AND (receipt_json IS NULL) = (receipt_digest IS NULL) AND (receipt_digest IS NULL) = (completed_at IS NULL)),
    CONSTRAINT operations_receipt_digest_shape CHECK (receipt_digest IS NULL OR receipt_digest ~ '^[0-9a-f]{64}$'),
    CONSTRAINT operations_receipt_surface_matches CHECK (receipt_kind IS NULL OR (surface = 'cloud.compute.k8s.cluster.create' AND receipt_kind = 'create') OR (surface = 'cloud.compute.k8s.cluster.delete' AND receipt_kind = 'delete'))
);
CREATE INDEX IF NOT EXISTS operations_resource_lookup
    ON compute_k8s_lifecycle.operations (tenant_id, resource_id, created_at);
ALTER TABLE compute_k8s_lifecycle.operations ENABLE ROW LEVEL SECURITY;
ALTER TABLE compute_k8s_lifecycle.operations FORCE ROW LEVEL SECURITY;
CREATE POLICY operations_tenant_isolation ON compute_k8s_lifecycle.operations AS PERMISSIVE FOR ALL TO compute_k8s_lifecycle_runtime USING (tenant_id = current_setting('oyatie.tenant_id', true)) WITH CHECK (tenant_id = current_setting('oyatie.tenant_id', true));
CREATE POLICY operations_require_tenant_guc ON compute_k8s_lifecycle.operations AS RESTRICTIVE FOR ALL TO compute_k8s_lifecycle_runtime USING (current_setting('oyatie.tenant_id', true) IS NOT NULL AND current_setting('oyatie.tenant_id', true) <> '') WITH CHECK (current_setting('oyatie.tenant_id', true) IS NOT NULL AND current_setting('oyatie.tenant_id', true) <> '');

GRANT SELECT, INSERT, UPDATE ON compute_k8s_lifecycle.clusters TO compute_k8s_lifecycle_runtime;
GRANT SELECT, INSERT, UPDATE ON compute_k8s_lifecycle.operations TO compute_k8s_lifecycle_runtime;
"#;

pub const K8S_LIFECYCLE_PENDING_INTENT_MIGRATION: &str = r#"ALTER TABLE compute_k8s_lifecycle.operations
  ADD COLUMN request_contract text NOT NULL DEFAULT 'trusted_envelope',
  ADD COLUMN operation_state text;
ALTER TABLE compute_k8s_lifecycle.operations
  ADD CONSTRAINT operations_request_contract CHECK
    (request_contract IN ('trusted_envelope', 'pending_intent')),
  ADD CONSTRAINT operations_contract_state CHECK
    ((request_contract = 'trusted_envelope' AND operation_state IS NULL)
     OR (request_contract = 'pending_intent'
         AND surface = 'cloud.compute.k8s.cluster.create'
         AND operation_state IS NOT NULL AND operation_state = 'accepted'));
"#;

pub const K8S_LIFECYCLE_MIGRATIONS: &[PgK8sLifecycleMigration] = &[
    PgK8sLifecycleMigration {
        version: 1,
        name: "runtime-role-boundary",
        sql: K8S_LIFECYCLE_RUNTIME_ROLE_MIGRATION,
        governed_table_count_after: 0,
    },
    PgK8sLifecycleMigration {
        version: 2,
        name: "tenant-lifecycle-repository",
        sql: K8S_LIFECYCLE_REPOSITORY_MIGRATION,
        governed_table_count_after: 2,
    },
    PgK8sLifecycleMigration {
        version: 3,
        name: "pending-intent-acceptance",
        sql: K8S_LIFECYCLE_PENDING_INTENT_MIGRATION,
        governed_table_count_after: 2,
    },
];

pub(crate) fn registry_is_valid() -> bool {
    let mut names = BTreeSet::new();
    let mut digests = BTreeSet::new();
    let versions_are_contiguous =
        K8S_LIFECYCLE_MIGRATIONS
            .iter()
            .enumerate()
            .all(|(index, migration)| {
                i64::try_from(index + 1).is_ok_and(|version| migration.version() == version)
                    && !migration.name().is_empty()
                    && names.insert(migration.name())
                    && digests.insert(migration.sha256())
            });
    versions_are_contiguous
        && K8S_LIFECYCLE_MIGRATIONS
            .last()
            .is_some_and(|migration| migration.version() == CURRENT_MIGRATION_VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_registry_is_contiguous_named_and_digestible() {
        assert!(registry_is_valid());
        assert_eq!(K8S_LIFECYCLE_MIGRATIONS.len(), 3);
        assert_eq!(
            K8S_LIFECYCLE_MIGRATIONS
                .iter()
                .map(|migration| migration.governed_table_count_after())
                .collect::<Vec<_>>(),
            [0, 2, 2]
        );
        assert_eq!(
            format!(
                "{:x}",
                Sha256::digest(MIGRATION_LEDGER_BOOTSTRAP.as_bytes())
            ),
            "0f84823fdbf60c78d421e3a6806dc1ea7a74f6443291dd2f108f5bc3c0ea1cf2"
        );
        assert_eq!(
            K8S_LIFECYCLE_MIGRATIONS
                .iter()
                .take(2)
                .map(|migration| migration.sha256())
                .collect::<Vec<_>>(),
            [
                "f94126ade44b1c866c8a7d1e035bf0112f520e5855a618fe909cca79b110784b",
                "84d5858b947f02a09c5156049ddf7f5b45819508f4c07d141275330a753a0c25",
            ]
        );
    }
}
