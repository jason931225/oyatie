use crate::{
    BindingOutcomeQueryRefV1, CellAdmissionTermV1, CellId, Digest32, ProofConstructionError,
};

macro_rules! opaque_id {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }

            pub fn parse(_value: impl Into<String>) -> Result<Self, ProofConstructionError> {
                Err(ProofConstructionError::NotImplemented)
            }
        }
    };
}

opaque_id!(ReservationId);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ReservationTerm(pub u64);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CellCapacityRevision(pub u64);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CapacityDimensionV1 {
    CpuMillis,
    MemoryBytes,
    StorageBytes,
    LocalSsdBytes,
    AcceleratorUnits,
    NetworkMegabitsPerSecond,
    Ipv4Addresses,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapacityAmountV1 {
    pub dimension: CapacityDimensionV1,
    pub units: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapacityVectorV1 {
    amounts: Vec<CapacityAmountV1>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CapacityVectorConstructionErrorV1 {
    UnspecifiedDimension,
    DuplicateDimension,
    DimensionsNotCanonical,
    ZeroUnits,
    NotImplemented,
}

impl CapacityVectorV1 {
    pub fn rehydrate(
        _amounts: Vec<CapacityAmountV1>,
    ) -> Result<Self, CapacityVectorConstructionErrorV1> {
        Err(CapacityVectorConstructionErrorV1::NotImplemented)
    }

    #[must_use]
    pub fn amounts(&self) -> &[CapacityAmountV1] {
        &self.amounts
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellCapacityLedgerV1 {
    pub cell_id: CellId,
    pub revision: CellCapacityRevision,
    pub total: CapacityVectorV1,
    pub safety_headroom: CapacityVectorV1,
    pub recovery_reserve: CapacityVectorV1,
    pub tentative_home: CapacityVectorV1,
    pub committed_home: CapacityVectorV1,
    pub tentative_warm_recovery: CapacityVectorV1,
    pub committed_warm_recovery: CapacityVectorV1,
    pub observed_at_unix_seconds: u64,
    pub record_digest: Digest32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CellCapacityLedgerVerificationErrorV1 {
    CellMismatch,
    RevisionDidNotAdvance,
    TotalChangedOutsideCellResourceMutation,
    DimensionSetMismatch,
    SafetyHeadroomExceedsTotal,
    RecoveryReserveExceedsTotal,
    HomeCapacityOvercommitted,
    RecoveryCapacityOvercommitted,
    ArithmeticOverflow,
    RecordDigestMismatch,
    NotImplemented,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedCellCapacityLedgerV1(CellCapacityLedgerV1);

impl VerifiedCellCapacityLedgerV1 {
    #[must_use]
    pub fn ledger(&self) -> &CellCapacityLedgerV1 {
        &self.0
    }
}

pub fn verify_cell_capacity_ledger(
    _previous: Option<&CellCapacityLedgerV1>,
    _next: CellCapacityLedgerV1,
) -> Result<VerifiedCellCapacityLedgerV1, CellCapacityLedgerVerificationErrorV1> {
    Err(CellCapacityLedgerVerificationErrorV1::NotImplemented)
}

/// Compare-and-set on the cell capacity ledger row:
/// [`CellCapacityLedgerV1`]. That row is proposed through the private-field
/// [`crate::VerifiedCellCapacityLedgerV1`] rather than as a bare record, so it
/// is not a row the write-set population reaches directly.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellCapacityPreconditionV1 {
    pub expected_revision: CellCapacityRevision,
    pub expected_record_digest: Digest32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CapacityAccountingClassV1 {
    Home,
    WarmRecovery,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReservationRefV1 {
    pub cell_id: CellId,
    pub reservation_id: ReservationId,
    pub term: ReservationTerm,
    pub accounting_class: CapacityAccountingClassV1,
    pub capacity: CapacityVectorV1,
    pub admission_term: CellAdmissionTermV1,
    pub reservation_digest: Digest32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ReservationLifecycleV1 {
    TentativeHeld,
    AwaitingBindingOutcome,
    Committed,
    Released,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReservationStatusV1 {
    pub reservation: ReservationRefV1,
    pub lifecycle: ReservationLifecycleV1,
    pub revision: u64,
    pub lifecycle_changed_at_unix_seconds: u64,
    pub tentative_expires_at_unix_seconds: Option<u64>,
    pub binding_operation_digest: Option<Digest32>,
    pub binding_attempt_digest: Option<Digest32>,
    pub binding_outcome_query: Option<BindingOutcomeQueryRefV1>,
    pub reservation_arm_receipt_digest: Option<Digest32>,
    pub reservation_commit_permit_digest: Option<Digest32>,
}
