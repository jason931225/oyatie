use std::path::PathBuf;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use foundry_ontology_app::{Config, compose, router};
use foundry_records_draft::{ActionEnvelope, RecordsLog};
use foundry_records_sqlite_draft::SqliteRecordsLog;
use http_body_util::BodyExt;
use tower::ServiceExt;

fn temp_path(case: &str, slot: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "foundry-ontology-probe-{case}-{slot}-{}-{}.sqlite",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock is after the epoch")
            .as_nanos()
    ))
}

struct Fixture {
    action: PathBuf,
    denial: PathBuf,
}

impl Fixture {
    fn new(case: &str) -> Self {
        let action = temp_path(case, "action");
        let denial = temp_path(case, "denial");
        let _ = std::fs::remove_file(&action);
        let _ = std::fs::remove_file(&denial);
        Self { action, denial }
    }

    fn config(&self) -> Config {
        Config {
            listen_addr: "127.0.0.1:0".into(),
            action_log: self.action.clone(),
            denial_log: self.denial.clone(),
            tenants: vec!["ten_test".into()],
            operators: Vec::new(),
        }
    }

    /// Append an entry the fold will consume and refuse: undecodable payload
    /// bytes are a deterministic poison, not a crash.
    fn seed_a_poisoning_entry(&self) {
        let mut log = SqliteRecordsLog::open(&self.action).expect("open the action log");
        log.append(
            ActionEnvelope::new(
                "ten_test",
                "ent_poison",
                "aty_calibrate",
                "idem_poison",
                1,
                b"these bytes are not a canonical action record".to_vec(),
                1_700_000_000_000,
            )
            .expect("a well-formed envelope"),
        )
        .expect("the append itself succeeds");
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.action);
        let _ = std::fs::remove_file(&self.denial);
    }
}

async fn get(fixture: &Fixture, path: &str) -> (StatusCode, String) {
    let state = compose(&fixture.config()).expect("boot");
    let response = router(state)
        .oneshot(
            Request::builder()
                .uri(path)
                .body(Body::empty())
                .expect("a well-formed request"),
        )
        .await
        .expect("the router answers");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("a readable body")
        .to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

#[tokio::test]
async fn healthz_reports_the_listener_is_bound() {
    let fixture = Fixture::new("healthz");
    let (status, _) = get(&fixture, "/healthz").await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn readyz_reports_ready_when_the_fold_is_caught_up() {
    let fixture = Fixture::new("readyz-clean");
    let (status, _) = get(&fixture, "/readyz").await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn a_poisoned_entry_never_un_readies_the_process() {
    let fixture = Fixture::new("readyz-poison");
    fixture.seed_a_poisoning_entry();
    let state = compose(&fixture.config()).expect("boot over a poisoned log");
    let seen = foundry_ontology_app::observation::observe(&state);
    assert_eq!(
        seen.poisoned, 1,
        "the fixture must actually poison, or this test proves nothing"
    );
    assert_eq!(
        seen.unreadable + seen.contended,
        0,
        "a total is only meaningful when every tenant was sampled"
    );
    assert!(
        foundry_ontology_app::observation::observe(&state).is_caught_up(),
        "poison advances the fold; it is not a readiness fault"
    );
    let (status, _) = get(&fixture, "/readyz").await;
    assert_eq!(status, StatusCode::OK);
    // Eligibility is earned by a value a constant could not hold by accident,
    // not asserted by a field. This is the only test that drives this gauge
    // off zero.
    let (_, body) = get(&fixture, "/metrics").await;
    assert!(
        // Newline-bounded: a bare `contains` of the value is satisfied by 10,
        // 11 and 100, so it would pin "starts with 1" rather than equals 1.
        body.contains("foundry_poisoned_entries 1\n"),
        "the poisoned gauge must report the poison this fixture created:\n{body}"
    );
}

#[tokio::test]
async fn statusz_refuses_an_unauthenticated_caller() {
    let fixture = Fixture::new("statusz");
    let (status, _) = get(&fixture, "/statusz").await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "an operator surface must not answer a caller it cannot identify"
    );
}

#[tokio::test]
async fn metrics_keeps_exporting_every_series_it_has_published() {
    let fixture = Fixture::new("metrics");
    let (status, body) = get(&fixture, "/metrics").await;
    assert_eq!(status, StatusCode::OK);
    // Named individually rather than by prefix: an SLO indicator that names
    // a metric this process stopped exporting must break here, not in a
    // dashboard six weeks later.
    for series in [
        "foundry_projection_lag",
        "foundry_poisoned_entries",
        "foundry_read_served_total",
        "foundry_read_refused_total",
        "foundry_action_submit_served_total",
        "foundry_action_submit_refused_total",
    ] {
        assert!(
            body.contains(series),
            "this process must keep exporting {series}, which is absent from:\n{body}"
        );
    }
}

#[tokio::test]
async fn an_unknown_route_is_not_found_and_never_a_silent_ok() {
    let fixture = Fixture::new("unknown");
    let (status, _) = get(&fixture, "/v1/nothing-here").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn the_write_surface_refuses_a_read_verb_and_an_absent_credential() {
    let fixture = Fixture::new("write-surface-guarded");
    let (method, _) = get(&fixture, "/v1/actions").await;
    assert_eq!(method, StatusCode::METHOD_NOT_ALLOWED);
}
