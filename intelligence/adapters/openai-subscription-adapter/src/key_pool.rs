//! Round-robin API-key pool: failure-count blacklist, jittered cooldown, success-restore.
// ADR-0083 Tier 3: `cfg(test)` exemption for unwrap/expect/panic.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
// data_class: INTERNAL_ONLY throughout this module.

use tracing::{debug, info, warn};

use crate::classifier::ResponseClass;
use crate::key_status::{KeyEntry, KeyStatus};

/// Number of consecutive transient failures before a key enters Cooling.
pub const FAILURE_THRESHOLD: u32 = 3;

/// Base cooldown duration in seconds before jitter is applied.
pub const COOLDOWN_BASE_SECS: u64 = 60;

/// Default upper bound for jitter (exclusive). Cooldown = base + [0, jitter_max).
pub const DEFAULT_JITTER_MAX_SECS: u64 = 30;

/// A pool of OpenAI API keys with round-robin selection and circuit-breaker logic.
///
/// SECURITY: holds secret-reference paths only, never raw key material.
pub struct KeyPool {
    entries: Vec<KeyEntry>,
    next_idx: usize,
    jitter_max_secs: u64,
}

impl KeyPool {
    /// Create a new pool from a list of secret-reference paths.
    pub fn new(sref_paths: Vec<String>) -> Self {
        let entries = sref_paths.into_iter().map(KeyEntry::new).collect();
        Self {
            entries,
            next_idx: 0,
            jitter_max_secs: DEFAULT_JITTER_MAX_SECS,
        }
    }

    /// Override the jitter upper bound (for testing with deterministic cooldown).
    pub fn with_jitter_max(mut self, jitter_max_secs: u64) -> Self {
        self.jitter_max_secs = jitter_max_secs;
        self
    }

    /// Returns `true` when the pool holds no keys.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Select the next eligible key by round-robin. `None` when every key is
    /// blacklisted or still cooling.
    pub fn select(&mut self, now_epoch_secs: u64) -> Option<usize> {
        let len = self.entries.len();
        if len == 0 {
            return None;
        }
        for i in 0..len {
            let idx = (self.next_idx + i) % len;
            if self.entries[idx].status.is_eligible(now_epoch_secs) {
                self.next_idx = (idx + 1) % len;
                debug!(key_index = idx, "selected OpenAI API key");
                return Some(idx);
            }
        }
        None
    }

    /// Record the result of a call that used key at `key_index`.
    ///
    /// `jitter_secs` must be in `[0, jitter_max_secs)`; derive it from a CSPRNG.
    pub fn record_result(
        &mut self,
        key_index: usize,
        class: ResponseClass,
        now_epoch_secs: u64,
        jitter_secs: u64,
    ) {
        let entry = &mut self.entries[key_index];
        match class {
            ResponseClass::Success => {
                if matches!(entry.status, KeyStatus::Cooling { .. }) {
                    info!(key_index, "OpenAI API key restored from cooling to active");
                    entry.status = KeyStatus::Active;
                }
            }
            ResponseClass::TerminalKeyInvalid | ResponseClass::TerminalQuotaExhausted => {
                warn!(
                    key_index,
                    class = ?class,
                    "OpenAI API key blacklisted (terminal error)"
                );
                entry.status = KeyStatus::Blacklisted;
            }
            ResponseClass::TransientRateLimit
            | ResponseClass::TransientServer
            | ResponseClass::TransientUnknown => {
                let new_count = match &entry.status {
                    KeyStatus::Active => 1,
                    KeyStatus::Cooling { failure_count, .. } => failure_count + 1,
                    KeyStatus::Blacklisted => return, // terminal; ignore
                };
                if new_count >= FAILURE_THRESHOLD {
                    let jitter = jitter_secs.min(self.jitter_max_secs.saturating_sub(1));
                    let until = now_epoch_secs
                        .saturating_add(COOLDOWN_BASE_SECS)
                        .saturating_add(jitter);
                    debug!(
                        key_index,
                        failure_count = new_count,
                        until_epoch_secs = until,
                        "OpenAI API key entering cooling"
                    );
                    entry.status = KeyStatus::Cooling {
                        until_epoch_secs: until,
                        failure_count: new_count,
                    };
                } else {
                    entry.status = KeyStatus::Cooling {
                        until_epoch_secs: 0, // eligible immediately (0 ≤ any now)
                        failure_count: new_count,
                    };
                }
            }
        }
    }

    /// Returns the status of the key at `index` (for tests / observability).
    pub fn key_status(&self, index: usize) -> &KeyStatus {
        &self.entries[index].status
    }

    /// Returns the sref_path of the key at `index`.
    pub fn key_sref(&self, index: usize) -> &str {
        &self.entries[index].sref_path
    }

    /// Returns the number of keys in the pool.
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classifier::ResponseClass;

    fn pool(keys: &[&str]) -> KeyPool {
        KeyPool::new(keys.iter().map(|s| s.to_string()).collect()).with_jitter_max(30)
    }

    #[test]
    fn empty_pool_returns_none() {
        let mut p = pool(&[]);
        assert!(p.select(1000).is_none());
    }

    #[test]
    fn single_key_selected() {
        let mut p = pool(&["sref://k0"]);
        assert_eq!(p.select(1000), Some(0));
    }

    #[test]
    fn round_robin_cycles() {
        let mut p = pool(&["sref://k0", "sref://k1", "sref://k2"]);
        let a = p.select(1000).unwrap();
        let b = p.select(1000).unwrap();
        let c = p.select(1000).unwrap();
        let d = p.select(1000).unwrap();
        assert_eq!(a, 0);
        assert_eq!(b, 1);
        assert_eq!(c, 2);
        assert_eq!(d, 0);
    }

    #[test]
    fn terminal_error_blacklists_key() {
        let mut p = pool(&["sref://k0"]);
        let idx = p.select(1000).unwrap();
        p.record_result(idx, ResponseClass::TerminalKeyInvalid, 1000, 0);
        assert_eq!(*p.key_status(0), KeyStatus::Blacklisted);
        assert!(p.select(1000).is_none());
    }

    #[test]
    fn quota_exhausted_blacklists_key() {
        let mut p = pool(&["sref://k0"]);
        p.record_result(0, ResponseClass::TerminalQuotaExhausted, 1000, 0);
        assert_eq!(*p.key_status(0), KeyStatus::Blacklisted);
    }

    #[test]
    fn three_transients_enter_cooling() {
        let now = 1_000_000u64;
        let mut p = pool(&["sref://k0"]).with_jitter_max(30);
        p.record_result(0, ResponseClass::TransientRateLimit, now, 0);
        p.record_result(0, ResponseClass::TransientRateLimit, now, 0);
        p.record_result(0, ResponseClass::TransientRateLimit, now, 0);
        match p.key_status(0) {
            KeyStatus::Cooling {
                until_epoch_secs,
                failure_count,
            } => {
                assert_eq!(*until_epoch_secs, now + 60);
                assert_eq!(*failure_count, 3);
            }
            s => panic!("expected Cooling, got {s:?}"),
        }
    }

    #[test]
    fn cooling_jitter_range() {
        let now = 1_000_000u64;
        let mut p = pool(&["sref://k0"]).with_jitter_max(30);
        p.record_result(0, ResponseClass::TransientServer, now, 0);
        p.record_result(0, ResponseClass::TransientServer, now, 0);
        p.record_result(0, ResponseClass::TransientServer, now, 29);
        match p.key_status(0) {
            KeyStatus::Cooling {
                until_epoch_secs, ..
            } => {
                assert_eq!(*until_epoch_secs, now + 89);
                assert!(*until_epoch_secs >= now + 60);
                assert!(*until_epoch_secs < now + 90);
            }
            s => panic!("expected Cooling, got {s:?}"),
        }
    }

    #[test]
    fn cooling_key_skipped_until_expiry() {
        let now = 1_000_000u64;
        let mut p = pool(&["sref://k0"]).with_jitter_max(0);
        p.record_result(0, ResponseClass::TransientServer, now, 0);
        p.record_result(0, ResponseClass::TransientServer, now, 0);
        p.record_result(0, ResponseClass::TransientServer, now, 0);
        assert!(p.select(now + 59).is_none());
        assert_eq!(p.select(now + 60), Some(0));
    }

    #[test]
    fn success_restores_cooling_key() {
        let now = 1_000_000u64;
        let mut p = pool(&["sref://k0"]).with_jitter_max(0);
        p.record_result(0, ResponseClass::TransientServer, now, 0);
        p.record_result(0, ResponseClass::TransientServer, now, 0);
        p.record_result(0, ResponseClass::TransientServer, now, 0);
        p.record_result(0, ResponseClass::Success, now + 100, 0);
        assert_eq!(*p.key_status(0), KeyStatus::Active);
        assert_eq!(p.select(now + 100), Some(0));
    }

    #[test]
    fn all_keys_blacklisted_returns_none() {
        let mut p = pool(&["sref://k0", "sref://k1"]);
        p.record_result(0, ResponseClass::TerminalKeyInvalid, 1000, 0);
        p.record_result(1, ResponseClass::TerminalKeyInvalid, 1000, 0);
        assert!(p.select(1000).is_none());
    }

    #[test]
    fn second_key_used_when_first_blacklisted() {
        let mut p = pool(&["sref://k0", "sref://k1"]);
        p.record_result(0, ResponseClass::TerminalKeyInvalid, 1000, 0);
        let idx = p.select(1000).unwrap();
        assert_eq!(idx, 1);
    }

    #[test]
    fn two_transients_do_not_trigger_cooling_start() {
        let now = 1_000u64;
        let mut p = pool(&["sref://k0"]);
        p.record_result(0, ResponseClass::TransientServer, now, 0);
        p.record_result(0, ResponseClass::TransientServer, now, 0);
        assert_eq!(p.select(now), Some(0));
    }
}
