//! Bounded-TTL DEK cache: data-plane static stability.
//!
//! AWS KMS precedent: the data plane never makes a per-request KMS call.
//! Unwrapped DEKs are cached with a hard TTL; while the KMS control plane is
//! down, cached DEKs keep serving until their TTL bound, and after the bound
//! the cache FAILS CLOSED — an expired DEK is never served, an unavailable
//! control plane is never silently retried around.
//!
//! Entry count is capped (cardinality cap doctrine); evicted and expired
//! entries scrub their key material via `DekMaterial`'s zeroize-on-drop
//! buffer.

use std::collections::HashMap;
use std::fmt;
use std::num::{NonZeroU64, NonZeroUsize};
use std::time::{SystemTime, UNIX_EPOCH};

use secrets_kms_domain::envelope_keys::{DekId, KekId};

use crate::material::{DekMaterial, KekVersion};

/// Injectable time source so TTL behavior is deterministic under test and
/// alignable with the platform clock substrate (G003 HLC `ClockSource`).
pub trait ClockSource {
    fn now_epoch_millis(&self) -> u64;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClockSource;

impl ClockSource for SystemClockSource {
    fn now_epoch_millis(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
            .unwrap_or(0)
    }
}

/// Cache key: a DEK is identified by the exact KEK id + version that wrapped
/// it plus its own id — never across versions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DekCacheKey {
    pub kek_id: KekId,
    pub kek_version: KekVersion,
    pub dek_id: DekId,
}

impl fmt::Display for DekCacheKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}/{}", self.kek_id, self.kek_version, self.dek_id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlPlaneUnavailable;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DekCacheError {
    ControlPlaneUnavailable {
        key: String,
        expired_at_epoch_millis: Option<u64>,
    },
}

impl fmt::Display for DekCacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ControlPlaneUnavailable {
                key,
                expired_at_epoch_millis,
            } => match expired_at_epoch_millis {
                Some(at) => write!(
                    f,
                    "dek-cache: '{key}' expired at {at}ms and the control plane is unavailable; failing closed"
                ),
                None => write!(
                    f,
                    "dek-cache: '{key}' not cached and the control plane is unavailable; failing closed"
                ),
            },
        }
    }
}

impl std::error::Error for DekCacheError {}

/// Where a served DEK came from — observability + static-stability evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchSource {
    Cache,
    ControlPlane,
}

struct CacheEntry {
    dek: DekMaterial,
    inserted_at: u64,
    expires_at: u64,
}

pub struct BoundedTtlDekCache<C: ClockSource> {
    ttl_millis: NonZeroU64,
    max_entries: NonZeroUsize,
    clock: C,
    entries: HashMap<DekCacheKey, CacheEntry>,
}

impl<C: ClockSource> BoundedTtlDekCache<C> {
    pub fn new(ttl_millis: NonZeroU64, max_entries: NonZeroUsize, clock: C) -> Self {
        Self {
            ttl_millis,
            max_entries,
            clock,
            entries: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Serve a DEK: from cache while fresh, otherwise via `fetch` against the
    /// control plane. A fresh hit never invokes `fetch` (no per-request KMS
    /// call); a stale/missing entry with an unavailable control plane fails
    /// closed and scrubs any stale entry.
    pub fn get_or_fetch<F>(
        &mut self,
        key: &DekCacheKey,
        fetch: F,
    ) -> Result<(&DekMaterial, FetchSource), DekCacheError>
    where
        F: FnOnce() -> Result<DekMaterial, ControlPlaneUnavailable>,
    {
        let now = self.clock.now_epoch_millis();
        let fresh = self.entries.get(key).is_some_and(|e| e.expires_at > now);
        let source = if fresh {
            FetchSource::Cache
        } else {
            match fetch() {
                Ok(dek) => {
                    self.replace_entry(key, dek, now);
                    FetchSource::ControlPlane
                }
                Err(ControlPlaneUnavailable) => {
                    let expired_at_epoch_millis =
                        self.entries.remove(key).map(|entry| entry.expires_at);
                    return Err(DekCacheError::ControlPlaneUnavailable {
                        key: key.to_string(),
                        expired_at_epoch_millis,
                    });
                }
            }
        };
        match self.entries.get(key) {
            Some(entry) => Ok((&entry.dek, source)),
            // Unreachable by construction (the entry is fresh or was just
            // inserted above); if it ever happens, fail closed.
            None => Err(DekCacheError::ControlPlaneUnavailable {
                key: key.to_string(),
                expired_at_epoch_millis: None,
            }),
        }
    }

    fn replace_entry(&mut self, key: &DekCacheKey, dek: DekMaterial, now: u64) {
        self.entries.remove(key);
        self.evict_expired(now);
        self.evict_to_capacity();
        let expires_at = now.saturating_add(self.ttl_millis.get());
        self.entries.insert(
            key.clone(),
            CacheEntry {
                dek,
                inserted_at: now,
                expires_at,
            },
        );
    }

    fn evict_expired(&mut self, now: u64) {
        self.entries.retain(|_, entry| entry.expires_at > now);
    }

    fn evict_to_capacity(&mut self) {
        while self.entries.len() >= self.max_entries.get() {
            let oldest = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.inserted_at)
                .map(|(key, _)| key.clone());
            match oldest {
                Some(key) => {
                    self.entries.remove(&key);
                }
                None => return,
            }
        }
    }
}

impl<C: ClockSource> fmt::Debug for BoundedTtlDekCache<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BoundedTtlDekCache {{ ttl_millis: {}, max_entries: {}, entries: {}, keys: [REDACTED] }}",
            self.ttl_millis,
            self.max_entries,
            self.entries.len()
        )
    }
}
