#[path = "facade_support/mod.rs"]
mod support;

use axum::http::StatusCode;
use support::{Fixture, get, post, write_a_record};

#[tokio::test]
async fn an_object_reads_back_at_the_revision_it_was_written_under() {
    let fixture = Fixture::new("read-object");
    write_a_record(&fixture, "ent_alpha", "idem_1").await;
    let (status, body) = get(
        &fixture,
        Some(fixture.operator_token()),
        "/v1/objects/ent_alpha?revision=1",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert!(body.contains("\"written_revision\":1"), "body: {body}");
    // Exact, because `body.contains("Ada")` is also satisfied by a Rust
    // Debug rendering — `String(\"Ada\")` — which is the wrong value in an
    // unstable format. The assertion has to be able to tell them apart.
    assert!(
        body.contains(r#""value":"Ada""#),
        "the value must round-trip as it was written: {body}"
    );
    assert!(
        body.contains(r#""value_type":"string""#),
        "a reader needs the declared type to interpret the value: {body}"
    );
    assert!(
        !body.contains("String("),
        "no Debug rendering may reach the wire: {body}"
    );
    assert!(
        body.contains(r#""data_class":"INTERNAL_ONLY""#),
        "the classification label must be canonical: {body}"
    );
    assert!(
        !body.contains("PrivacyDataClass"),
        "no Debug rendering of the carrier may reach the wire: {body}"
    );
    assert!(
        body.contains("\"upcast_state\":\"current\""),
        "an object at its own revision owes nothing: {body}"
    );
}

#[tokio::test]
async fn an_unretained_pin_is_a_typed_refusal_not_a_guess() {
    let fixture = Fixture::new("read-unretained");
    write_a_record(&fixture, "ent_alpha", "idem_1").await;
    let (status, _) = get(
        &fixture,
        Some(fixture.operator_token()),
        "/v1/objects/ent_alpha?revision=9",
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "a revision the type never accepted is a caller error, not an empty read"
    );
}

#[tokio::test]
async fn an_unknown_object_is_not_found() {
    let fixture = Fixture::new("read-missing");
    let (status, body) = get(
        &fixture,
        Some(fixture.operator_token()),
        "/v1/objects/ent_ghost?revision=1",
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    // Assert the typed body, not just the status: a bare 404 is also what
    // the router's fallback produces, so status alone would pass with the
    // whole handler removed.
    assert!(
        body.contains(r#""gate":"surface""#) && body.contains("no applied entry ever bound"),
        "the refusal must be the handler's typed one: {body}"
    );
}

#[tokio::test]
async fn history_shows_the_write_attributed_to_its_author() {
    let fixture = Fixture::new("read-history");
    write_a_record(&fixture, "ent_alpha", "idem_1").await;
    let (status, body) = get(
        &fixture,
        Some(fixture.operator_token()),
        "/v1/objects/ent_alpha/history",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("prn_alice"), "who wrote it: {body}");
    assert!(body.contains("record.written"), "under what event: {body}");
    assert!(body.contains("\"ordinal\":1"), "at what position: {body}");
}

#[tokio::test]
async fn the_audit_view_reports_every_consumed_entry() {
    let fixture = Fixture::new("read-audit");
    write_a_record(&fixture, "ent_alpha", "idem_1").await;
    let (status, body) = get(&fixture, Some(fixture.operator_token()), "/v1/audit").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("\"ordinal\":1"), "body: {body}");
    assert!(
        body.contains("\"disposition\":\"applied\""),
        "an audit view states each entry's disposition: {body}"
    );
}

#[tokio::test]
async fn the_type_registry_serves_what_the_writer_stamps_against() {
    let fixture = Fixture::new("read-types");
    let (status, body) = get(&fixture, Some(fixture.operator_token()), "/v1/types").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("ety_record"), "body: {body}");
    assert!(
        body.contains("\"revision\":1"),
        "a reader needs the revision to pin against: {body}"
    );
}

#[tokio::test]
async fn every_read_requires_a_credential() {
    let fixture = Fixture::new("read-anon");
    for path in [
        "/v1/objects/ent_alpha?revision=1",
        "/v1/objects/ent_alpha/history",
        "/v1/audit",
        "/v1/types",
    ] {
        let (status, _) = get(&fixture, None, path).await;
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "{path} must not answer an unauthenticated caller"
        );
    }
}

#[tokio::test]
async fn a_roleless_caller_reads_nothing() {
    let fixture = Fixture::new("read-roleless");
    write_a_record(&fixture, "ent_alpha", "idem_1").await;
    let (status, _) = get(
        &fixture,
        Some(fixture.roleless_token()),
        "/v1/objects/ent_alpha?revision=1",
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "the read action is its own authority; recognition is not permission"
    );
    let (write, _) = post(
        &fixture,
        Some(fixture.roleless_token()),
        r#"{"object_ref":"ent_beta","action_type":"aty_record_write","idempotency_key":"idem_2","occurred_at_epoch_seconds":1700000000,"properties":{"name":"Bea"}}"#,
    )
    .await;
    assert_eq!(write, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn the_tenant_wide_views_are_authorized_not_merely_authenticated() {
    // `/v1/audit` and `/v1/types` are the widest-blast-radius routes: the
    // audit view carries every object_ref, principal and decision id in the
    // tenant. A recognized caller holding no role must reach neither, or
    // authentication would be standing in for authorization.
    let fixture = Fixture::new("read-tenant-views");
    write_a_record(&fixture, "ent_alpha", "idem_1").await;
    for path in ["/v1/audit", "/v1/types"] {
        let (status, body) = get(&fixture, Some(fixture.roleless_token()), path).await;
        assert_eq!(
            status,
            StatusCode::FORBIDDEN,
            "{path} must refuse a roleless caller"
        );
        assert!(
            !body.contains("prn_alice") && !body.contains("ety_record"),
            "{path} must leak nothing to a refused caller: {body}"
        );
    }
}

#[tokio::test]
async fn an_anonymous_caller_is_refused_before_the_query_is_judged() {
    let fixture = Fixture::new("read-anon-noquery");
    let (status, _) = get(&fixture, None, "/v1/objects/ent_alpha").await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "authentication comes before parameter validation"
    );
    let (with_credential, body) = get(
        &fixture,
        Some(fixture.operator_token()),
        "/v1/objects/ent_alpha",
    )
    .await;
    assert_eq!(with_credential, StatusCode::BAD_REQUEST);
    assert!(body.contains(r#""gate":"surface""#), "body: {body}");
}

/// Every unusable revision shape, from both sides of the credential.
///
/// This is what defends the in-handler parse. A typed extractor rejects all
/// of these BEFORE the handler runs, so reverting to one would answer an
/// unauthenticated stranger with a 400 describing the API instead of asking
/// for a credential — and without these assertions that revert passes CI
/// silently. The authorized half pins the other direction: an unusable pin
/// is a typed surface refusal, never a silent fallback to some default
/// revision.
#[tokio::test]
async fn every_unusable_revision_shape_answers_by_credential_not_by_parser() {
    let fixture = Fixture::new("read-unusable-shapes");
    write_a_record(&fixture, "ent_alpha", "idem_1").await;
    const UNUSABLE: [&str; 6] = [
        "/v1/objects/ent_alpha",
        "/v1/objects/ent_alpha?revision=abc",
        "/v1/objects/ent_alpha?revision=-1",
        "/v1/objects/ent_alpha?revision=99999999999",
        "/v1/objects/ent_alpha?revision=",
        "/v1/objects/ent_alpha?revision=1&revision=2",
    ];
    for path in UNUSABLE {
        let (anonymous, _) = get(&fixture, None, path).await;
        assert_eq!(
            anonymous,
            StatusCode::UNAUTHORIZED,
            "{path}: the credential is asked for before the query is judged"
        );
        let (authorized, body) = get(&fixture, Some(fixture.operator_token()), path).await;
        assert_eq!(
            authorized,
            StatusCode::BAD_REQUEST,
            "{path}: an unusable pin is refused, never defaulted"
        );
        assert!(
            body.contains(r#""gate":"surface""#),
            "{path}: the refusal is the handler's typed one: {body}"
        );
        assert!(
            !body.contains("\"value\""),
            "{path}: a refused read serves no object: {body}"
        );
    }
}
