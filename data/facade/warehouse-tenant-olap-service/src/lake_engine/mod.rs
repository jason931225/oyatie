#![allow(dead_code)]

use crate::domain::{DatasetId, TenantId};
use crate::error::{ServiceError, ServiceResult};

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum LakeProtocol {
    Delta,
    Iceberg,
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

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct LakeCommitReceipt {
    pub table: LakeTableRef,
    pub protocol: LakeProtocol,
    pub commit_version: u64,
    pub bytes_written: u64,
    pub conflict_retries: u8,
}

pub struct DeltaWriterCore;

impl DeltaWriterCore {
    pub const PROTOCOL: LakeProtocol = LakeProtocol::Delta;

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
