mod facade_support;
mod failing_log;
use facade_support as support;

use axum::http::StatusCode;
use support::{Fixture, Session, WRITE_BODY as WRITE, scrape, value_of};

fn session_with_a_failing_log(fixture: &Fixture, detail: &'static str) -> Session {
    Session::from_state(failing_log::state_with_a_failing_log(
        &fixture.config(),
        detail,
    ))
}

#[tokio::test]
async fn a_storage_fault_is_not_reported_as_an_idempotency_conflict() {
    let fixture = Fixture::new("storage-fault");
    let session = session_with_a_failing_log(&fixture, "the disk went away");

    let (status, body) = session.post(Some(fixture.operator_token()), WRITE).await;

    assert_ne!(
        status,
        StatusCode::CONFLICT,
        "a storage fault must not be answered as the caller's key conflict: {body}"
    );
    // The EXACT status, not merely 5xx. Which code a storage fault gets is
    // the whole decision this test exists to defend, and `is_server_error`
    // admits 500 and 502 without complaint.
    assert_eq!(
        status,
        StatusCode::SERVICE_UNAVAILABLE,
        "a storage fault is the service's failure and is reported as such, got {status}: {body}"
    );
    assert!(
        !body.contains("idempotency key"),
        "the refusal must not name a cause that did not occur: {body}"
    );
    // The EXACT body, not the absence of one rendering of the detail. A
    // substring guard is evaded by any transform — a truncation, a case
    // change, a well-meaning sanitiser that strips boilerplate and keeps the
    // path-bearing tail — and the property being defended is that NO adapter
    // detail escapes in any rendering.
    assert_eq!(
        body,
        r#"{"gate":"log","cause":"the action log could not be written; the submission was not accepted"}"#,
        "the refusal body must be exactly the authored one"
    );
}

#[tokio::test]
async fn a_divergent_key_reuse_is_still_the_callers_conflict() {
    let fixture = Fixture::new("storage-fault-conflict");
    let session = fixture.session();

    let (first, _) = session.post(Some(fixture.operator_token()), WRITE).await;
    assert_eq!(first, StatusCode::OK);
    let divergent = WRITE.replace(r#""name":"Ada""#, r#""name":"Grace""#);
    let (second, body) = session
        .post(Some(fixture.operator_token()), &divergent)
        .await;

    assert_eq!(second, StatusCode::CONFLICT, "{body}");
    assert!(
        body.contains("idempotency key"),
        "the conflict must still name the spent key: {body}"
    );
}

#[tokio::test]
async fn a_storage_fault_counts_as_a_refused_submission() {
    let fixture = Fixture::new("storage-fault-counted");
    let session = session_with_a_failing_log(&fixture, "the disk went away");

    let before = value_of(
        &scrape(&session).await,
        "foundry_action_submit_refused_total",
    );
    let (status, _) = session.post(Some(fixture.operator_token()), WRITE).await;
    assert!(status.is_server_error());
    let after = value_of(
        &scrape(&session).await,
        "foundry_action_submit_refused_total",
    );

    assert_eq!(after, before + 1, "a storage fault must count exactly once");
}
