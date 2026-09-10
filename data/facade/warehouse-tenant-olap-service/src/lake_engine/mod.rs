//! lake_engine — open-table-format ACID write substrate for the data-warehouse
//! microservice.
//!
//! Truth-up scaffold (Wave 15-IMPL-truth-up, 2026-05-21). The IP slices
//! (IP-031 Delta, IP-032 Iceberg, IP-033 Hudi, IP-034 Unity-Catalog-class,
//! IP-037 CDF, IP-040 Time-Travel, IP-041 Zero-Copy-Clone) declare a
//! `lake-engine` sublayer that hosts the Delta / Iceberg / Hudi protocol
//! writers. The `manifest.json` `layer_enum_conformance.declared_layers`
//! list carries `lake-engine` as the 10th declared layer. The ADR-0105
//! 12-layer enum (`crate::domain::ArchitectureLayer`) keeps the canonical
//! enum closed at 12; `lake_engine` sits inside the adapter/worker/
//! infrastructure stratum as an open-table substrate, not as a new
//! top-level layer.
//!
//! This module is a scaffold: the protocol writers (`DeltaWriterCore`,
//! `IcebergWriterCore`, `HudiWriterCore`) carry only the shape that IPs
//! 031/032/033 reference, so downstream IPs can cite a real Rust path.
//! Full ACID commit semantics land in subsequent Wave-15B implementation
//! plans per the REMEDIATION-NOTES follow-up ledger.

#![allow(dead_code)]

use crate::domain::{DatasetId, TenantId};
use crate::error::{ServiceError, ServiceResult};

/// Open-table protocol identifier per IPs 031/032/033.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum LakeProtocol {
    /// Delta Lake (IP-031)
    Delta,
    /// Apache Iceberg v2 (IP-032)
    Iceberg,
    /// Apache Hudi (IP-033)
    Hudi,
}

impl LakeProtocol {
    pub const fn slug(&self) -> &'static str {
        match self {
            Self::Delta => "delta",
            Self::Iceberg => "iceberg",
            Self::Hudi => "hudi",
        }
    }
}

/// Tenant-scoped lake table identity (per-tenant bucket layout from IP-031 §3.2).
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct LakeTableRef {
    pub tenant_id: TenantId,
    pub dataset_id: DatasetId,
    pub catalog: String,
    pub schema: String,
    pub table: String,
    pub protocol: LakeProtocol,
}

impl LakeTableRef {
    pub fn validate(&self) -> ServiceResult<()> {
        if self.catalog.trim().is_empty() {
            return Err(ServiceError::missing_field("catalog"));
        }
        if self.schema.trim().is_empty() {
            return Err(ServiceError::missing_field("schema"));
        }
        if self.table.trim().is_empty() {
            return Err(ServiceError::missing_field("table"));
        }
        Ok(())
    }

    /// Storage prefix per IP-031 §3.2 layout (tenant-pinned bucket).
    pub fn storage_prefix(&self) -> String {
        format!(
            "oyatie-{}-warehouse/{}/{}/{}",
            self.tenant_id.as_str(),
            self.catalog,
            self.schema,
            self.table
        )
    }
}

/// Atomic-commit outcome shared by Delta / Iceberg / Hudi writers.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct LakeCommitReceipt {
    pub table: LakeTableRef,
    pub protocol: LakeProtocol,
    pub commit_version: u64,
    pub bytes_written: u64,
    pub conflict_retries: u8,
}

/// Delta-Lake-class protocol writer (IP-031).
///
/// Wave 15-IMPL-truth-up scaffold; full putIfAbsent commit loop is Wave-15B.
pub struct DeltaWriterCore;

impl DeltaWriterCore {
    pub const PROTOCOL: LakeProtocol = LakeProtocol::Delta;

    /// Validate a Delta write request and emit a scaffolded receipt.
    ///
    /// The full putIfAbsent commit-collision retry loop (IP-031 §3.3) is
    /// scheduled for Wave-15B. This scaffold only enforces the tenant
    /// scope and table identity invariants that the IP-031 acceptance
    /// criteria depend on.
    pub fn stage_commit(
        table: LakeTableRef,
        bytes_written: u64,
    ) -> ServiceResult<LakeCommitReceipt> {
        table.validate()?;
        Ok(LakeCommitReceipt {
            table,
            protocol: Self::PROTOCOL,
            commit_version: 0,
            bytes_written,
            conflict_retries: 0,
        })
    }
}

/// Apache-Iceberg-v2 protocol writer (IP-032).
pub struct IcebergWriterCore;

impl IcebergWriterCore {
    pub const PROTOCOL: LakeProtocol = LakeProtocol::Iceberg;

    pub fn stage_snapshot(
        table: LakeTableRef,
        bytes_written: u64,
    ) -> ServiceResult<LakeCommitReceipt> {
        table.validate()?;
        Ok(LakeCommitReceipt {
            table,
            protocol: Self::PROTOCOL,
            commit_version: 0,
            bytes_written,
            conflict_retries: 0,
        })
    }
}

/// Apache-Hudi protocol writer (IP-033).
pub struct HudiWriterCore;

impl HudiWriterCore {
    pub const PROTOCOL: LakeProtocol = LakeProtocol::Hudi;

    pub fn stage_commit(
        table: LakeTableRef,
        bytes_written: u64,
    ) -> ServiceResult<LakeCommitReceipt> {
        table.validate()?;
        Ok(LakeCommitReceipt {
            table,
            protocol: Self::PROTOCOL,
            commit_version: 0,
            bytes_written,
            conflict_retries: 0,
        })
    }
}

/// Change-Data-Feed cursor for IP-037.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ChangeDataFeedCursor {
    pub from_version: u64,
    pub max_rows: u32,
}

impl ChangeDataFeedCursor {
    pub fn validate(&self) -> ServiceResult<()> {
        if self.max_rows == 0 {
            return Err(ServiceError::invariant(
                "cdf_max_rows_nonzero",
                "change-data-feed pull requires nonzero max_rows",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_table() -> LakeTableRef {
        LakeTableRef {
            tenant_id: TenantId::new("tenant-demo"),
            dataset_id: DatasetId::new("dataset-demo"),
            catalog: "finance".to_owned(),
            schema: "ledger".to_owned(),
            table: "margin".to_owned(),
            protocol: LakeProtocol::Delta,
        }
    }

    #[test]
    fn delta_stage_commit_returns_receipt() {
        let receipt = DeltaWriterCore::stage_commit(sample_table(), 4096).unwrap();
        assert_eq!(receipt.protocol, LakeProtocol::Delta);
        assert_eq!(receipt.bytes_written, 4096);
    }

    #[test]
    fn iceberg_stage_snapshot_returns_receipt() {
        let mut t = sample_table();
        t.protocol = LakeProtocol::Iceberg;
        let receipt = IcebergWriterCore::stage_snapshot(t, 8192).unwrap();
        assert_eq!(receipt.protocol, LakeProtocol::Iceberg);
    }

    #[test]
    fn hudi_stage_commit_returns_receipt() {
        let mut t = sample_table();
        t.protocol = LakeProtocol::Hudi;
        let receipt = HudiWriterCore::stage_commit(t, 1024).unwrap();
        assert_eq!(receipt.protocol, LakeProtocol::Hudi);
    }

    #[test]
    fn empty_table_field_is_rejected() {
        let mut t = sample_table();
        t.table = String::new();
        assert!(DeltaWriterCore::stage_commit(t, 4096).is_err());
    }

    #[test]
    fn cdf_cursor_requires_nonzero_max_rows() {
        let cursor = ChangeDataFeedCursor {
            from_version: 1,
            max_rows: 0,
        };
        assert!(cursor.validate().is_err());
    }

    #[test]
    fn storage_prefix_uses_tenant_bucket_pattern() {
        let prefix = sample_table().storage_prefix();
        assert!(prefix.starts_with("oyatie-tenant-demo-warehouse/"));
    }
}
