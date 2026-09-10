#[path = "facade_support/mod.rs"]
mod support;

use axum::http::StatusCode;
use support::{Fixture, Session, WRITE_BODY as WRITE, scrape, value_of};

#[tokio::test]
async fn an_answered_read_increments_served() {
    let fixture = Fixture::new("metrics-read-served");
    let session = fixture.session();
    let (write, _) = session.post(Some(fixture.operator_token()), WRITE).await;
    assert_eq!(write, StatusCode::OK);
    let (status, _) = session
        .get(
            Some(fixture.operator_token()),
            "/v1/objects/ent_alpha?revision=1",
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let body = scrape(&session).await;
    assert_eq!(value_of(&body, "foundry_read_served_total"), 1);
    assert_eq!(value_of(&body, "foundry_read_refused_total"), 0);
}

/// Each refusing site, pinned INDIVIDUALLY by a delta.
///
/// An aggregate total conflates sites: it passes as long as the sum is
/// right, so one site counting twice hides another counting never. A
/// per-case delta localizes, which means deleting any single counting call
/// fails a case that names its site.
///
/// Not every case has a site to itself, and the cross-tenant cases are the
/// deliberate exception: Cedar's forbid refuses a foreign tenant before the
/// roster is ever consulted, so they are a SECOND exercise of the
/// policy-denial site rather than a pin on the roster site. That is the
/// property worth keeping — it is why the roster refusal needs the separate
/// fixture below — but it means deleting the policy-denial call fails the
/// earlier of the two cases, not both.
async fn assert_read_refusal_delta(
    session: &Session,
    label: &str,
    request: impl AsRef<str>,
    token: Option<&str>,
    expect: StatusCode,
) {
    let before = value_of(&scrape(session).await, "foundry_read_refused_total");
    let (status, _) = session.get(token, request.as_ref()).await;
    assert_eq!(status, expect, "{label}");
    let after = value_of(&scrape(session).await, "foundry_read_refused_total");
    assert_eq!(after, before + 1, "{label}: must count exactly one refusal");
}

#[tokio::test]
async fn each_read_refusal_site_counts_exactly_once() {
    let fixture = Fixture::new("metrics-read-refused");
    let session = fixture.session();
    // The unretained-revision refusal needs a BINDING to reach: without one,
    // that request takes the unknown-object site instead and the two cases
    // silently pin the same call.
    let (write, _) = session.post(Some(fixture.operator_token()), WRITE).await;
    assert_eq!(write, StatusCode::OK);
    let path = "/v1/objects/ent_alpha?revision=1";
    assert_read_refusal_delta(
        &session,
        "no credential",
        path,
        None,
        StatusCode::UNAUTHORIZED,
    )
    .await;
    assert_read_refusal_delta(
        &session,
        "unrecognised credential",
        path,
        Some("nope"),
        StatusCode::UNAUTHORIZED,
    )
    .await;
    assert_read_refusal_delta(
        &session,
        "policy denial",
        path,
        Some(fixture.roleless_token()),
        StatusCode::FORBIDDEN,
    )
    .await;
    assert_read_refusal_delta(
        &session,
        "cross-tenant (a second exercise of the policy-denial site)",
        path,
        Some(fixture.foreign_token()),
        StatusCode::FORBIDDEN,
    )
    .await;
    assert_read_refusal_delta(
        &session,
        "unusable revision pin",
        "/v1/objects/ent_alpha?revision=abc",
        Some(fixture.operator_token()),
        StatusCode::BAD_REQUEST,
    )
    .await;
    assert_read_refusal_delta(
        &session,
        "unknown object",
        "/v1/objects/ent_ghost?revision=1",
        Some(fixture.operator_token()),
        StatusCode::NOT_FOUND,
    )
    .await;
    assert_read_refusal_delta(
        &session,
        "unretained revision",
        "/v1/objects/ent_alpha?revision=9",
        Some(fixture.operator_token()),
        StatusCode::CONFLICT,
    )
    .await;
}

/// The unserved-tenant refusal on BOTH surfaces, which no other case reaches.
///
/// The policy point permits this caller — it addresses an object in its own
/// tenant — and the roster then does not hold that tenant. An earlier
/// revision believed a foreign-tenant credential exercised this site; it
/// does not, because the Cedar cross-tenant forbid refuses first, so that
/// case was a second exercise of the policy-denial site and this one was
/// never executed at all.
#[tokio::test]
async fn the_unserved_tenant_refusal_counts() {
    let fixture = Fixture::new("metrics-unserved-tenant");
    let session = fixture.unserved_session();
    assert_read_refusal_delta(
        &session,
        "operator's tenant is not in the served roster",
        "/v1/objects/ent_alpha?revision=1",
        Some(fixture.operator_token()),
        StatusCode::FORBIDDEN,
    )
    .await;

    let before = value_of(
        &scrape(&session).await,
        "foundry_action_submit_refused_total",
    );
    let (status, _) = session.post(Some(fixture.operator_token()), WRITE).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let after = value_of(
        &scrape(&session).await,
        "foundry_action_submit_refused_total",
    );
    assert_eq!(
        after,
        before + 1,
        "an unserved tenant must count on submit too"
    );
}

#[tokio::test]
async fn each_read_serving_route_counts_exactly_once() {
    let fixture = Fixture::new("metrics-read-routes");
    let session = fixture.session();
    let (write, _) = session.post(Some(fixture.operator_token()), WRITE).await;
    assert_eq!(write, StatusCode::OK);
    for (label, path) in [
        ("object", "/v1/objects/ent_alpha?revision=1"),
        ("history", "/v1/objects/ent_alpha/history"),
        ("audit", "/v1/audit"),
        ("types", "/v1/types"),
    ] {
        let before = value_of(&scrape(&session).await, "foundry_read_served_total");
        let (status, _) = session.get(Some(fixture.operator_token()), path).await;
        assert_eq!(status, StatusCode::OK, "{label}");
        let after = value_of(&scrape(&session).await, "foundry_read_served_total");
        assert_eq!(after, before + 1, "{label}: a served read must count once");
    }
}

#[tokio::test]
async fn the_gauges_report_the_state_they_name() {
    let fixture = Fixture::new("metrics-gauges");
    let session = fixture.session();
    let body = scrape(&session).await;
    assert_eq!(value_of(&body, "foundry_served_tenants"), 1);
    assert_eq!(value_of(&body, "foundry_projection_lag"), 0);
    assert_eq!(value_of(&body, "foundry_poisoned_entries"), 0);
}
