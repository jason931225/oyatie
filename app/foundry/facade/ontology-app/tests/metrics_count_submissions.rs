mod facade_support;
use facade_support as support;

use axum::http::StatusCode;
use support::{Fixture, WRITE_BODY as WRITE, scrape, value_of};

#[tokio::test]
async fn an_accepted_submission_increments_served_and_not_refused() {
    let fixture = Fixture::new("metrics-submit-served");
    let session = fixture.session();
    let (status, _) = session.post(Some(fixture.operator_token()), WRITE).await;
    assert_eq!(status, StatusCode::OK);
    let body = scrape(&session).await;
    assert_eq!(value_of(&body, "foundry_action_submit_served_total"), 1);
    assert_eq!(value_of(&body, "foundry_action_submit_refused_total"), 0);
}
#[tokio::test]
async fn a_refusal_before_the_writer_still_counts_against_availability() {
    let fixture = Fixture::new("metrics-submit-anon");
    let session = fixture.session();
    let (status, _) = session.post(None, WRITE).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let body = scrape(&session).await;
    assert_eq!(value_of(&body, "foundry_action_submit_refused_total"), 1);
    assert_eq!(value_of(&body, "foundry_action_submit_served_total"), 0);
}
#[tokio::test]
async fn a_policy_denial_counts_as_a_refused_submission() {
    let fixture = Fixture::new("metrics-submit-roleless");
    let session = fixture.session();
    let (status, _) = session.post(Some(fixture.roleless_token()), WRITE).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let body = scrape(&session).await;
    assert_eq!(value_of(&body, "foundry_action_submit_refused_total"), 1);
}
/// The two POST-WRITER arms, which every pre-writer case above misses.
///
/// Both arms sit after the writer returns, so no refusal reached before it
/// exercises either one. The refused arm is reachable only through a
/// divergent reuse of a spent idempotency key — the one failure this
/// surface answers with 409 rather than refusing up front.
#[tokio::test]
async fn the_writer_outcome_counts_on_both_arms() {
    let fixture = Fixture::new("metrics-writer-arms");
    let session = fixture.session();

    let served_before = value_of(
        &scrape(&session).await,
        "foundry_action_submit_served_total",
    );
    let (applied, _) = session.post(Some(fixture.operator_token()), WRITE).await;
    assert_eq!(applied, StatusCode::OK);
    let served_after = value_of(
        &scrape(&session).await,
        "foundry_action_submit_served_total",
    );
    assert_eq!(
        served_after,
        served_before + 1,
        "an applied submission must count as served"
    );

    // The SAME key, different content: the log refuses the append rather
    // than silently deduplicating, and that is a refused submission.
    let divergent = WRITE.replace(r#""name":"Ada""#, r#""name":"Grace""#);
    let refused_before = value_of(
        &scrape(&session).await,
        "foundry_action_submit_refused_total",
    );
    let (conflict, _) = session
        .post(Some(fixture.operator_token()), &divergent)
        .await;
    assert_eq!(conflict, StatusCode::CONFLICT);
    let refused_after = value_of(
        &scrape(&session).await,
        "foundry_action_submit_refused_total",
    );
    assert_eq!(
        refused_after,
        refused_before + 1,
        "a writer-level refusal must count against availability"
    );
}
#[tokio::test]
async fn each_submit_refusal_site_counts_exactly_once() {
    let fixture = Fixture::new("metrics-submit-sites");
    let session = fixture.session();
    for (label, token, body, expect) in [
        ("no credential", None, WRITE, StatusCode::UNAUTHORIZED),
        (
            "unrecognised credential",
            Some("nope"),
            WRITE,
            StatusCode::UNAUTHORIZED,
        ),
        (
            "malformed body",
            Some(fixture.operator_token()),
            "{not json",
            StatusCode::BAD_REQUEST,
        ),
        (
            "action type is not an action id",
            Some(fixture.operator_token()),
            r#"{"object_ref":"ent_a","action_type":"nope","idempotency_key":"k","occurred_at_epoch_seconds":1700000000,"properties":{"name":"A"}}"#,
            StatusCode::BAD_REQUEST,
        ),
        (
            "no representable edit",
            Some(fixture.operator_token()),
            r#"{"object_ref":"ent_a","action_type":"aty_record_write","idempotency_key":"k","occurred_at_epoch_seconds":1700000000,"properties":{"":"A"}}"#,
            StatusCode::BAD_REQUEST,
        ),
        (
            "policy denial",
            Some(fixture.roleless_token()),
            WRITE,
            StatusCode::FORBIDDEN,
        ),
        (
            "cross-tenant (a second exercise of the policy-denial site)",
            Some(fixture.foreign_token()),
            WRITE,
            StatusCode::FORBIDDEN,
        ),
    ] {
        let before = value_of(
            &scrape(&session).await,
            "foundry_action_submit_refused_total",
        );
        let (status, _) = session.post(token, body).await;
        assert_eq!(status, expect, "{label}");
        let after = value_of(
            &scrape(&session).await,
            "foundry_action_submit_refused_total",
        );
        assert_eq!(after, before + 1, "{label}: must count exactly one refusal");
    }
}
