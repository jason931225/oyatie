#![allow(dead_code)]

use std::path::PathBuf;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use foundry_ontology_app::{AppState, Config, OperatorCredential, compose, router, router_from};
use foundry_records_draft::RecordsLog;
use foundry_records_sqlite_draft::SqliteRecordsLog;
use http_body_util::BodyExt;
use tower::ServiceExt;

pub const TENANT: &str = "ten_acme";
const OPERATOR_TOKEN: &str = "operator-token-for-tests";
const FOREIGN_TOKEN: &str = "foreign-token-for-tests";
const ROLELESS_TOKEN: &str = "roleless-token-for-tests";

pub struct Fixture {
    action: PathBuf,
    denial: PathBuf,
}

impl Fixture {
    pub fn new(case: &str) -> Self {
        let stamp = format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock is after the epoch")
                .as_nanos()
        );
        let temp = std::env::temp_dir();
        let fixture = Self {
            action: temp.join(format!("foundry-facade-{case}-action-{stamp}.sqlite")),
            denial: temp.join(format!("foundry-facade-{case}-denial-{stamp}.sqlite")),
        };
        let _ = std::fs::remove_file(&fixture.action);
        let _ = std::fs::remove_file(&fixture.denial);
        fixture
    }

    /// A process serving a DIFFERENT tenant from the one its operator
    /// credential names. The policy decision point permits the caller — the
    /// object it addresses belongs to the caller's own tenant — and the
    /// roster then does not hold it, which is the only way to reach the
    /// unserved-tenant refusal. Reachable in production through a config
    /// whose operator list and tenant roster disagree.
    pub fn config_with_unserved_operator_tenant(&self) -> Config {
        let mut config = self.config();
        config.tenants = vec!["ten_elsewhere".into()];
        config
    }

    pub fn unserved_session(&self) -> Session {
        Session {
            router: router(compose(&self.config_with_unserved_operator_tenant()).expect("boots")),
        }
    }

    pub fn config(&self) -> Config {
        Config {
            listen_addr: "127.0.0.1:0".into(),
            action_log: self.action.clone(),
            denial_log: self.denial.clone(),
            tenants: vec![TENANT.into()],
            operators: vec![
                OperatorCredential {
                    token: OPERATOR_TOKEN.into(),
                    tenant_id: TENANT.into(),
                    principal_id: "prn_alice".into(),
                    roles: vec!["foundry-operator".into()],
                },
                OperatorCredential {
                    token: FOREIGN_TOKEN.into(),
                    tenant_id: "ten_other".into(),
                    principal_id: "prn_mallory".into(),
                    roles: vec!["foundry-operator".into()],
                },
                OperatorCredential {
                    token: ROLELESS_TOKEN.into(),
                    tenant_id: TENANT.into(),
                    principal_id: "prn_nobody".into(),
                    roles: Vec::new(),
                },
            ],
        }
    }

    pub fn state(&self) -> AppState {
        compose(&self.config()).expect("the fixture must boot")
    }

    pub fn operator_token(&self) -> &'static str {
        OPERATOR_TOKEN
    }

    pub fn foreign_token(&self) -> &'static str {
        FOREIGN_TOKEN
    }

    pub fn roleless_token(&self) -> &'static str {
        ROLELESS_TOKEN
    }

    pub fn action_log_path(&self) -> std::path::PathBuf {
        self.action.clone()
    }

    /// The action log's head, read from the durable store itself rather
    /// than from anything the process reports about itself.
    pub fn log_head(&self) -> u64 {
        SqliteRecordsLog::open(&self.action)
            .expect("the action log opens")
            .head(TENANT)
            .expect("head is readable")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.action);
        let _ = std::fs::remove_file(&self.denial);
    }
}

pub async fn post(fixture: &Fixture, token: Option<&str>, body: &str) -> (StatusCode, String) {
    let mut request = Request::builder()
        .method("POST")
        .uri("/v1/actions")
        .header("content-type", "application/json");
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let response = router(fixture.state())
        .oneshot(
            request
                .body(Body::from(body.to_owned()))
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

pub async fn get(fixture: &Fixture, token: Option<&str>, path: &str) -> (StatusCode, String) {
    let mut request = Request::builder().method("GET").uri(path);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let response = router(fixture.state())
        .oneshot(request.body(Body::empty()).expect("a well-formed request"))
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

pub async fn write_a_record(fixture: &Fixture, object_ref: &str, key: &str) {
    let body = format!(
        r#"{{"object_ref":"{object_ref}","action_type":"aty_record_write","idempotency_key":"{key}","occurred_at_epoch_seconds":1700000000,"properties":{{"name":"Ada"}}}}"#
    );
    let (status, reply) = post(fixture, Some(fixture.operator_token()), &body).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "the fixture write must land: {reply}"
    );
}

/// One booted process, driven across several requests.
///
/// The per-request helpers above compose a FRESH `AppState` each call, which
/// is right for isolation but makes process-lifetime counters unobservable —
/// every request would start from zero. A session builds the router once and
/// clones it per request, so what one request counted the next one can see.
pub struct Session {
    router: axum::Router,
}

impl Fixture {
    pub fn session(&self) -> Session {
        Session {
            router: router(self.state()),
        }
    }
}

impl Session {
    pub fn from_shared(state: std::sync::Arc<AppState>) -> Self {
        Session {
            router: router_from(state),
        }
    }

    pub fn from_state(state: AppState) -> Self {
        Session {
            router: router(state),
        }
    }

    pub async fn post(&self, token: Option<&str>, body: &str) -> (StatusCode, String) {
        let mut request = Request::builder()
            .method("POST")
            .uri("/v1/actions")
            .header("content-type", "application/json");
        if let Some(token) = token {
            request = request.header("authorization", format!("Bearer {token}"));
        }
        self.send(
            request
                .body(Body::from(body.to_owned()))
                .expect("a request"),
        )
        .await
    }

    pub async fn get(&self, token: Option<&str>, path: &str) -> (StatusCode, String) {
        let mut request = Request::builder().method("GET").uri(path);
        if let Some(token) = token {
            request = request.header("authorization", format!("Bearer {token}"));
        }
        self.send(request.body(Body::empty()).expect("a request"))
            .await
    }

    pub async fn send(&self, request: Request<Body>) -> (StatusCode, String) {
        let response = self
            .router
            .clone()
            .oneshot(request)
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
}

pub const WRITE_BODY: &str = r#"{"object_ref":"ent_alpha","action_type":"aty_record_write","idempotency_key":"idem_1","occurred_at_epoch_seconds":1700000000,"properties":{"name":"Ada"}}"#;

/// Scrape the exposition. UNAUTHENTICATED on purpose: were `/metrics` behind
/// the refusal counter, every scrape would inflate the very number the
/// delta assertions read.
pub async fn scrape(session: &Session) -> String {
    let (status, body) = session.get(None, "/metrics").await;
    assert_eq!(status, StatusCode::OK);
    body
}

pub fn value_of(body: &str, metric: &str) -> u64 {
    body.lines()
        .find_map(|line| line.strip_prefix(&format!("{metric} ")))
        .unwrap_or_else(|| panic!("{metric} has no value line in:\n{body}"))
        .trim()
        .parse()
        .expect("a metric value is a number")
}
