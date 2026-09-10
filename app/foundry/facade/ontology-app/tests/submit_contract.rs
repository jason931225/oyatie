#[path = "facade_support/mod.rs"]
mod support;

use axum::http::StatusCode;
use support::{Fixture, post};

#[tokio::test]
async fn an_authorized_operator_writes_through_the_gate() {
    let fixture = Fixture::new("submit-ok");
    let (status, body) = post(
        &fixture,
        Some(fixture.operator_token()),
        r#"{"object_ref":"ent_alpha","action_type":"aty_record_write","idempotency_key":"idem_1","occurred_at_epoch_seconds":1700000000,"properties":{"name":"Ada"}}"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert!(
        body.contains("\"applied\""),
        "an accepted submission reports what became of it: {body}"
    );
    assert!(
        body.contains("\"ordinal\":1"),
        "the caller learns the log position its write took: {body}"
    );
}

#[tokio::test]
async fn an_unauthenticated_caller_is_refused_before_any_policy_question() {
    let fixture = Fixture::new("submit-anon");
    let (status, _) = post(
        &fixture,
        None,
        r#"{"object_ref":"ent_alpha","action_type":"aty_record_write","idempotency_key":"idem_1","occurred_at_epoch_seconds":1700000000,"properties":{"name":"Ada"}}"#,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        fixture.log_head(),
        0,
        "an unauthenticated request must not reach the log"
    );
}

#[tokio::test]
async fn an_unknown_token_is_refused_and_appends_nothing() {
    let fixture = Fixture::new("submit-badtoken");
    let (status, _) = post(
        &fixture,
        Some("not-a-real-token"),
        r#"{"object_ref":"ent_alpha","action_type":"aty_record_write","idempotency_key":"idem_1","occurred_at_epoch_seconds":1700000000,"properties":{"name":"Ada"}}"#,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(fixture.log_head(), 0);
}

#[tokio::test]
async fn a_byte_identical_retry_deduplicates_to_the_original_outcome() {
    let fixture = Fixture::new("submit-retry");
    let payload = r#"{"object_ref":"ent_alpha","action_type":"aty_record_write","idempotency_key":"idem_same","occurred_at_epoch_seconds":1700000000,"properties":{"name":"Ada"}}"#;
    let (first, _) = post(&fixture, Some(fixture.operator_token()), payload).await;
    assert_eq!(first, StatusCode::OK);
    let (second, body) = post(&fixture, Some(fixture.operator_token()), payload).await;
    assert_eq!(second, StatusCode::OK);
    assert!(
        body.contains("\"deduplicated\":true"),
        "the retry must report itself as a dedup, not as a second write: {body}"
    );
    assert_eq!(
        fixture.log_head(),
        1,
        "a byte-identical retry appends nothing new"
    );
}

#[tokio::test]
async fn a_malformed_body_is_refused_without_touching_the_log() {
    let fixture = Fixture::new("submit-malformed");
    let (status, _) = post(&fixture, Some(fixture.operator_token()), "{not json").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(fixture.log_head(), 0);
}

#[tokio::test]
async fn an_undeclared_property_is_refused_by_the_writer_not_the_surface() {
    let fixture = Fixture::new("submit-undeclared");
    let (status, body) = post(
        &fixture,
        Some(fixture.operator_token()),
        r#"{"object_ref":"ent_alpha","action_type":"aty_record_write","idempotency_key":"idem_1","occurred_at_epoch_seconds":1700000000,"properties":{"name":"Ada","nonesuch":"x"}}"#,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body: {body}");
    assert_eq!(
        fixture.log_head(),
        0,
        "a refused submission appends nothing"
    );
}
