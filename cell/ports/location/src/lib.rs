//! Agreed cross-owner contract for region, availability-zone, and cell identity.
//!
//! This port owns the shared value shapes and their fail-closed validation.
//! Cell's region engine consumes and re-exports these types while retaining its
//! broader catalog and routing error contract.

#![forbid(unsafe_code)]

pub const CELL_ID_PREFIX: &str = "cell-";

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct RegionCode {
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct AzCode {
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct CellId {
    pub value: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CellLocationError {
    InvalidRegionCode,
    InvalidAzCode,
    InvalidCellId,
}

impl RegionCode {
    pub fn new(value: impl Into<String>) -> Result<Self, CellLocationError> {
        let value = value.into();
        validate_canonical_code(&value, CellLocationError::InvalidRegionCode)?;
        Ok(Self { value })
    }
}

impl AzCode {
    pub fn new(value: impl Into<String>) -> Result<Self, CellLocationError> {
        let value = value.into();
        validate_canonical_code(&value, CellLocationError::InvalidAzCode)?;
        Ok(Self { value })
    }
}

impl CellId {
    pub fn new(value: impl Into<String>) -> Result<Self, CellLocationError> {
        let value = value.into();
        if !value.starts_with(CELL_ID_PREFIX) {
            return Err(CellLocationError::InvalidCellId);
        }
        validate_canonical_code(&value, CellLocationError::InvalidCellId)?;
        Ok(Self { value })
    }
}

fn validate_canonical_code(value: &str, error: CellLocationError) -> Result<(), CellLocationError> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed != value
        || trimmed.starts_with('-')
        || trimmed.ends_with('-')
        || trimmed.contains("--")
        || !trimmed
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(error);
    }
    Ok(())
}
