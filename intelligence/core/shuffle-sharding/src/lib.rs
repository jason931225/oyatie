//! Deterministic shuffle-sharding for tenant-to-cell placement.
//!
//! Pure algorithm only: the mutable cell topology and the live health inputs
//! stay with `iac-app` and `observability`, so placement stays reproducible
//! from its arguments alone.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use std::collections::BTreeSet;

/// A cell candidate supplied by the caller.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellCandidate {
    pub cell_id: String,
    /// Residency label, honoured only when the request sets `required_pack`.
    pub pack: String,
    pub region: String,
    pub accepts_new_tenants: bool,
}

/// Request for deterministic tenant shuffle-shard selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShuffleShardRequest {
    /// Stable tenant identifier. Raw user identifiers do not belong here.
    pub tenant_id: String,
    pub shard_width: usize,
    /// Changing this moves every tenant; it is the deliberate-rebalance knob.
    pub placement_salt: String,
    pub required_pack: Option<String>,
    pub required_region: Option<String>,
    pub candidates: Vec<CellCandidate>,
}

/// Deterministic selection result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShuffleShard {
    pub tenant_id: String,
    pub placement_salt: String,
    pub cell_ids: Vec<String>,
}

/// Validation and selection failures.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShuffleShardError {
    EmptyTenantId,
    EmptyPlacementSalt,
    ZeroShardWidth,
    EmptyCellId,
    EmptyPack,
    EmptyRegion,
    DuplicateCellId(String),
    NotEnoughEligibleCells { required: usize, available: usize },
}

/// Selects a deterministic shuffle shard for one tenant.
///
/// Topology freshness and cell health are the caller's: a cell that is
/// unreachable but still `accepts_new_tenants` will be selected.
pub fn select_shuffle_shard(
    request: ShuffleShardRequest,
) -> Result<ShuffleShard, ShuffleShardError> {
    validate_request(&request)?;

    let eligible = eligible_cells(&request);
    if eligible.len() < request.shard_width {
        return Err(ShuffleShardError::NotEnoughEligibleCells {
            required: request.shard_width,
            available: eligible.len(),
        });
    }

    let mut ranked = eligible
        .into_iter()
        .map(|candidate| {
            (
                rank_cell(
                    &request.tenant_id,
                    &request.placement_salt,
                    &candidate.cell_id,
                ),
                candidate.cell_id.clone(),
            )
        })
        .collect::<Vec<_>>();

    ranked.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));

    Ok(ShuffleShard {
        tenant_id: request.tenant_id,
        placement_salt: request.placement_salt,
        cell_ids: ranked
            .into_iter()
            .take(request.shard_width)
            .map(|(_, cell_id)| cell_id)
            .collect(),
    })
}

fn validate_request(request: &ShuffleShardRequest) -> Result<(), ShuffleShardError> {
    if request.tenant_id.trim().is_empty() {
        return Err(ShuffleShardError::EmptyTenantId);
    }
    if request.placement_salt.trim().is_empty() {
        return Err(ShuffleShardError::EmptyPlacementSalt);
    }
    if request.shard_width == 0 {
        return Err(ShuffleShardError::ZeroShardWidth);
    }

    let mut seen_cell_ids = BTreeSet::new();
    for candidate in &request.candidates {
        validate_candidate(candidate)?;
        if !seen_cell_ids.insert(candidate.cell_id.clone()) {
            return Err(ShuffleShardError::DuplicateCellId(
                candidate.cell_id.clone(),
            ));
        }
    }

    Ok(())
}

fn validate_candidate(candidate: &CellCandidate) -> Result<(), ShuffleShardError> {
    if candidate.cell_id.trim().is_empty() {
        return Err(ShuffleShardError::EmptyCellId);
    }
    if candidate.pack.trim().is_empty() {
        return Err(ShuffleShardError::EmptyPack);
    }
    if candidate.region.trim().is_empty() {
        return Err(ShuffleShardError::EmptyRegion);
    }
    Ok(())
}

fn eligible_cells(request: &ShuffleShardRequest) -> Vec<&CellCandidate> {
    request
        .candidates
        .iter()
        .filter(|candidate| candidate.accepts_new_tenants)
        .filter(|candidate| {
            request
                .required_pack
                .as_ref()
                .is_none_or(|pack| candidate.pack == *pack)
        })
        .filter(|candidate| {
            request
                .required_region
                .as_ref()
                .is_none_or(|region| candidate.region == *region)
        })
        .collect()
}

fn rank_cell(tenant_id: &str, placement_salt: &str, cell_id: &str) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    hash = fnv1a_write(hash, tenant_id.as_bytes());
    hash = fnv1a_write(hash, &[0]);
    hash = fnv1a_write(hash, placement_salt.as_bytes());
    hash = fnv1a_write(hash, &[0]);
    hash = fnv1a_write(hash, cell_id.as_bytes());
    splitmix64_final(hash)
}

fn fnv1a_write(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn splitmix64_final(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
