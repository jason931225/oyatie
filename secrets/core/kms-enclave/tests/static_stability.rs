//! Static-stability failure injection for the bounded-TTL DEK cache.

use std::cell::Cell;
use std::num::{NonZeroU64, NonZeroUsize};

use secrets_kms_enclave::{
    BoundedTtlDekCache, ClockSource, ControlPlaneUnavailable, DekCacheError, DekCacheKey, DekId,
    FetchSource, KekId, KekMaterial, KekVersion, SystemClockSource,
};

struct FakeClock(Cell<u64>);

impl ClockSource for &FakeClock {
    fn now_epoch_millis(&self) -> u64 {
        self.0.get()
    }
}

fn cache_key(dek: &str) -> DekCacheKey {
    DekCacheKey {
        kek_id: KekId::new("kek/ten_alpha").expect("kek id"),
        kek_version: KekVersion::INITIAL,
        dek_id: DekId::new(dek).expect("dek id"),
    }
}

fn fresh_dek(dek: &str) -> secrets_kms_enclave::DekMaterial {
    let kek = KekMaterial::generate(
        KekId::new("kek/ten_alpha").expect("id"),
        KekVersion::INITIAL,
    )
    .expect("kek");
    let (material, _) = kek.generate_dek(DekId::new(dek).expect("id")).expect("dek");
    material
}

const TTL_MS: u64 = 300_000; // 5-minute static-stability window

fn cache(clock: &FakeClock) -> BoundedTtlDekCache<&FakeClock> {
    BoundedTtlDekCache::new(
        NonZeroU64::new(TTL_MS).expect("ttl"),
        NonZeroUsize::new(4).expect("cap"),
        clock,
    )
}

#[test]
fn green_control_plane_down_within_ttl_keeps_serving() {
    let clock = FakeClock(Cell::new(1_000));
    let mut dek_cache = cache(&clock);
    let key = cache_key("dek/obj_1");

    let fetches = Cell::new(0u32);
    let (_, source) = dek_cache
        .get_or_fetch(&key, || {
            fetches.set(fetches.get() + 1);
            Ok(fresh_dek("dek/obj_1"))
        })
        .expect("warm fetch");
    assert_eq!(source, FetchSource::ControlPlane);
    assert_eq!(fetches.get(), 1);

    for offset in [1u64, TTL_MS / 2, TTL_MS - 1] {
        clock.0.set(1_000 + offset);
        let (dek, source) = dek_cache
            .get_or_fetch(&key, || {
                fetches.set(fetches.get() + 1);
                Err(ControlPlaneUnavailable)
            })
            .expect("static-stability read");
        assert_eq!(source, FetchSource::Cache, "offset {offset}");
        assert_eq!(dek.dek_id().value(), "dek/obj_1");
    }
    assert_eq!(fetches.get(), 1, "no control-plane call during the window");
}

#[test]
fn red_control_plane_down_past_ttl_fails_closed() {
    let clock = FakeClock(Cell::new(1_000));
    let mut dek_cache = cache(&clock);
    let key = cache_key("dek/obj_1");

    dek_cache
        .get_or_fetch(&key, || Ok(fresh_dek("dek/obj_1")))
        .expect("warm fetch");

    clock.0.set(1_000 + TTL_MS);
    let err = dek_cache
        .get_or_fetch(&key, || Err(ControlPlaneUnavailable))
        .expect_err("expired entry must not serve");
    match err {
        DekCacheError::ControlPlaneUnavailable {
            expired_at_epoch_millis,
            ..
        } => {
            assert_eq!(expired_at_epoch_millis, Some(1_000 + TTL_MS));
        }
    }

    // The expired entry was scrubbed: a later failure reports no prior entry.
    let err = dek_cache
        .get_or_fetch(&key, || Err(ControlPlaneUnavailable))
        .expect_err("still failing closed");
    match err {
        DekCacheError::ControlPlaneUnavailable {
            expired_at_epoch_millis,
            ..
        } => {
            assert_eq!(expired_at_epoch_millis, None);
        }
    }
    assert!(dek_cache.is_empty());
}

#[test]
fn recovery_after_outage_refreshes_the_window() {
    let clock = FakeClock(Cell::new(0));
    let mut dek_cache = cache(&clock);
    let key = cache_key("dek/obj_1");

    dek_cache
        .get_or_fetch(&key, || Ok(fresh_dek("dek/obj_1")))
        .expect("warm");

    clock.0.set(TTL_MS + 1);
    assert!(
        dek_cache
            .get_or_fetch(&key, || Err(ControlPlaneUnavailable))
            .is_err()
    );

    let (_, source) = dek_cache
        .get_or_fetch(&key, || Ok(fresh_dek("dek/obj_1")))
        .expect("recovered");
    assert_eq!(source, FetchSource::ControlPlane);

    clock.0.set(TTL_MS + 1 + TTL_MS - 1);
    let (_, source) = dek_cache
        .get_or_fetch(&key, || Err(ControlPlaneUnavailable))
        .expect("served within re-armed window");
    assert_eq!(source, FetchSource::Cache);
}

#[test]
fn cardinality_cap_evicts_oldest_inserted() {
    let clock = FakeClock(Cell::new(0));
    let mut dek_cache = BoundedTtlDekCache::new(
        NonZeroU64::new(TTL_MS).expect("ttl"),
        NonZeroUsize::new(2).expect("cap"),
        &clock,
    );

    for (at, dek) in [(0u64, "dek/obj_1"), (10, "dek/obj_2"), (20, "dek/obj_3")] {
        clock.0.set(at);
        dek_cache
            .get_or_fetch(&cache_key(dek), || Ok(fresh_dek(dek)))
            .expect("insert");
    }
    assert_eq!(dek_cache.len(), 2);

    clock.0.set(30);
    let err = dek_cache
        .get_or_fetch(&cache_key("dek/obj_1"), || Err(ControlPlaneUnavailable))
        .expect_err("evicted entry needs refetch");
    assert!(matches!(err, DekCacheError::ControlPlaneUnavailable { .. }));

    for dek in ["dek/obj_2", "dek/obj_3"] {
        let (_, source) = dek_cache
            .get_or_fetch(&cache_key(dek), || Err(ControlPlaneUnavailable))
            .expect("cached");
        assert_eq!(source, FetchSource::Cache);
    }
}

#[test]
fn distinct_kek_versions_are_distinct_cache_entries() {
    let clock = FakeClock(Cell::new(0));
    let mut dek_cache = cache(&clock);
    let v1_key = cache_key("dek/obj_1");
    let v2_key = DekCacheKey {
        kek_version: KekVersion::INITIAL.next().expect("v2"),
        ..cache_key("dek/obj_1")
    };

    dek_cache
        .get_or_fetch(&v1_key, || Ok(fresh_dek("dek/obj_1")))
        .expect("v1");
    let err = dek_cache
        .get_or_fetch(&v2_key, || Err(ControlPlaneUnavailable))
        .expect_err("version-scoped key must miss");
    assert!(matches!(err, DekCacheError::ControlPlaneUnavailable { .. }));
    assert_eq!(dek_cache.len(), 1);
}

#[test]
fn system_clock_source_is_monotonic_enough_for_ttl() {
    const EPOCH_MILLIS_2020_01_01: u64 = 1_577_836_800_000;
    let first = SystemClockSource.now_epoch_millis();
    let second = SystemClockSource.now_epoch_millis();
    assert!(second >= first);
    assert!(first > EPOCH_MILLIS_2020_01_01);
}
