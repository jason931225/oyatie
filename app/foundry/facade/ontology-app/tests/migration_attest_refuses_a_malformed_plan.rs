//! What the attest surface does with a plan it cannot execute, and what it
//! counts while doing it.
//!
//! Split from the tenancy suite because that file reached the 300-line budget
//! with these unwritten — and they are the half an independent review found
//! missing, not a rounding-out. A surface whose whole job is to answer "is
//! this migration owed?" must never answer "no" because it failed to read
//! the question.

mod facade_support;
mod migration_support;

use axum::http::StatusCode;
use facade_support::{Fixture, Session, scrape, value_of};
use migration_support::{attest, plan_for, state_with_two_revisions};

/// THE FALSE FIXPOINT. A one-character typo in the `transforms` key must not
/// be discarded into an empty default.
///
/// Without `deny_unknown_fields` the unknown key vanished, `#[serde(default)]`
/// supplied an empty list, `validate`'s transform loop passed over nothing,
/// and the surface answered `{"fixpoint":true,"pending":[]}` — a green light
/// to skip a migration that IS owed. The state here is the one where the
/// correct answer is known to be the opposite: `ent_alpha` carries `note` and
/// no `nickname`, which is exactly what the plan's `copy_as` owes.
#[tokio::test]
async fn a_misspelled_transforms_key_is_refused_not_defaulted_to_a_fixpoint() {
    let fixture = Fixture::new("attest-typo");
    let session = Session::from_state(state_with_two_revisions(&fixture.config()));
    let (written, reply) = session
        .post(
            Some(fixture.operator_token()),
            r#"{"object_ref":"ent_alpha","action_type":"aty_record_write",
                "idempotency_key":"idem_1","occurred_at_epoch_seconds":1700000000,
                "properties":{"name":"Ada","note":"Countess"}}"#,
        )
        .await;
    assert_eq!(written, StatusCode::OK, "{reply}");
    let typo = plan_for("ten_acme").replace("\"transforms\"", "\"transform\"");

    let (status, body) = attest(&session, Some(fixture.operator_token()), &typo).await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(
        !body.contains(r#""fixpoint":true"#),
        "a plan this surface could not read is never a fixpoint: {body}"
    );
}

/// The same body, spelled correctly, is the control: it proves the refusal
/// above is the typo and not something else about the request.
#[tokio::test]
async fn the_same_plan_spelled_correctly_is_answered() {
    let fixture = Fixture::new("attest-typo-control");
    let session = Session::from_state(state_with_two_revisions(&fixture.config()));

    let (status, body) = attest(
        &session,
        Some(fixture.operator_token()),
        &plan_for("ten_acme"),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
}

/// EVERY refusal this surface makes is counted, not one of four.
///
/// The metric was asserted only at the tenancy check, so the parse, the
/// transform-vocabulary and the validate refusals could each stop counting
/// without any test noticing. A refusal no counter saw is a refusal no
/// operator sees.
#[tokio::test]
async fn every_refusal_site_increments_the_refusal_counter() {
    let fixture = Fixture::new("attest-refusal-accounting");
    let session = Session::from_state(state_with_two_revisions(&fixture.config()));
    let token = Some(fixture.operator_token());

    // 1: unreadable body.
    let (parse, _) = attest(&session, token, "{not a plan").await;
    // 2: a conversion this process does not perform.
    let unknown_conversion = plan_for("ten_acme").replace(
        r#"{"kind":"copy_as","from":"note","to":"nickname"}"#,
        r#"{"kind":"convert_as","from":"note","to":"nickname","conversion":"nope"}"#,
    );
    let (conversion, _) = attest(&session, token, &unknown_conversion).await;
    // 3: a plan the registry refuses.
    let absent = plan_for("ten_acme").replace("ety_record", "ety_absent");
    let (validate, _) = attest(&session, token, &absent).await;

    assert_eq!(
        (parse, conversion, validate),
        (
            StatusCode::BAD_REQUEST,
            StatusCode::BAD_REQUEST,
            StatusCode::BAD_REQUEST
        ),
        "all three must refuse for the count below to mean anything"
    );
    assert_eq!(
        value_of(&scrape(&session).await, "foundry_read_refused_total"),
        3,
        "three refusals, three counted — an exact total, so a site that stops \
         counting cannot hide behind another that starts"
    );
}

/// The `default_to` vocabulary is reachable and carried through.
///
/// The whole `WireDefault` enum had no test: it was neither exercised nor
/// shown to be unreachable, which is the state in which a mapping arm can be
/// wrong indefinitely. A default fills an ABSENCE, so an object lacking
/// `nickname` is owed one.
#[tokio::test]
async fn a_default_to_transform_is_carried_into_the_attestation() {
    let fixture = Fixture::new("attest-default-to");
    let session = Session::from_state(state_with_two_revisions(&fixture.config()));
    let (written, reply) = session
        .post(
            Some(fixture.operator_token()),
            r#"{"object_ref":"ent_alpha","action_type":"aty_record_write",
                "idempotency_key":"idem_1","occurred_at_epoch_seconds":1700000000,
                "properties":{"name":"Ada"}}"#,
        )
        .await;
    assert_eq!(written, StatusCode::OK, "{reply}");
    let plan = plan_for("ten_acme").replace(
        r#"{"kind":"copy_as","from":"note","to":"nickname"}"#,
        r#"{"kind":"default_to","to":"nickname","value":{"type":"string","value":"Countess"}}"#,
    );

    let (status, body) = attest(&session, Some(fixture.operator_token()), &plan).await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        body.contains(r#""pending":["ent_alpha"]"#),
        "an object with no nickname is owed the default: {body}"
    );
}

/// A default whose `type` is not in the vocabulary is refused as unreadable,
/// rather than silently dropped by a tolerant deserializer.
#[tokio::test]
async fn a_default_of_an_unknown_type_is_refused() {
    let fixture = Fixture::new("attest-default-unknown");
    let session = Session::from_state(state_with_two_revisions(&fixture.config()));
    let plan = plan_for("ten_acme").replace(
        r#"{"kind":"copy_as","from":"note","to":"nickname"}"#,
        r#"{"kind":"default_to","to":"nickname","value":{"type":"duration","value":5}}"#,
    );

    let (status, body) = attest(&session, Some(fixture.operator_token()), &plan).await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(!body.contains(r#""fixpoint""#), "{body}");
}

/// A DEFAULT FILLS AN ABSENCE AND NEVER OVERWRITES.
///
/// The held value here DIFFERS from the default, and that difference is the
/// whole test. An object holding the default already is not owed it either,
/// but that case cannot tell the law from its absence: the general
/// equal-value check returns the same answer whether or not the never-
/// overwrite rule exists. With a different value held, only the rule keeps
/// the object out of `pending` — removing it makes the object owed, which is
/// the surface telling an operator to overwrite data the plan must not touch.
#[tokio::test]
async fn a_default_does_not_overwrite_a_value_the_object_already_holds() {
    let fixture = Fixture::new("attest-default-satisfied");
    let session = Session::from_state(state_with_two_revisions(&fixture.config()));
    let (written, reply) = session
        .post(
            Some(fixture.operator_token()),
            r#"{"object_ref":"ent_alpha","action_type":"aty_record_write",
                "idempotency_key":"idem_1","occurred_at_epoch_seconds":1700000000,
                "properties":{"name":"Ada","nickname":"Duchess"}}"#,
        )
        .await;
    assert_eq!(written, StatusCode::OK, "{reply}");
    let plan = plan_for("ten_acme").replace(
        r#"{"kind":"copy_as","from":"note","to":"nickname"}"#,
        r#"{"kind":"default_to","to":"nickname","value":{"type":"string","value":"Countess"}}"#,
    );

    let (status, body) = attest(&session, Some(fixture.operator_token()), &plan).await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        body.contains(r#""fixpoint":true"#) && body.contains(r#""pending":[]"#),
        "the object holds a value already, so the default is not owed it: {body}"
    );
}

/// A default's VARIANT is checked against the target's declared type.
///
/// `check_transform` compares the declared scalar against
/// `DefaultValue::scalar_type()`. An untyped target cannot see this — it
/// carries the legacy String contract, so every non-string default refuses
/// alike. `counter` is declared `Integer` so the arms stop being
/// interchangeable.
#[tokio::test]
async fn a_default_whose_variant_does_not_match_the_declared_type_is_refused() {
    let fixture = Fixture::new("attest-typed-default-wrong");
    let session = Session::from_state(state_with_two_revisions(&fixture.config()));
    let plan = plan_for("ten_acme").replace(
        r#"{"kind":"copy_as","from":"note","to":"nickname"}"#,
        r#"{"kind":"default_to","to":"counter","value":{"type":"timestamp","epoch_millis":5}}"#,
    );

    let (status, body) = attest(&session, Some(fixture.operator_token()), &plan).await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(
        body.contains("TypeIncompatible"),
        "a timestamp is not an integer, and the registry is what says so: {body}"
    );
}

/// The control for the test above: the matching variant IS admitted, so the
/// refusal there is the type mismatch and not the typed target itself.
#[tokio::test]
async fn a_default_whose_variant_matches_the_declared_type_is_admitted() {
    let fixture = Fixture::new("attest-typed-default-right");
    let session = Session::from_state(state_with_two_revisions(&fixture.config()));
    let plan = plan_for("ten_acme").replace(
        r#"{"kind":"copy_as","from":"note","to":"nickname"}"#,
        r#"{"kind":"default_to","to":"counter","value":{"type":"integer","value":5}}"#,
    );

    let (status, body) = attest(&session, Some(fixture.operator_token()), &plan).await;

    assert_eq!(status, StatusCode::OK, "{body}");
}

/// An unknown FIELD inside a default is refused — not what an unknown TYPE
/// tests, which tag matching rejects with or without `deny_unknown_fields`.
/// A well-formed `integer` default with a stray `epoch_millis` beside it is
/// the shape a half-edited plan takes, and was admitted silently until the
/// attribute landed on both wire enums.
#[tokio::test]
async fn a_default_carrying_a_field_from_another_variant_is_refused() {
    let fixture = Fixture::new("attest-default-mixed-fields");
    let session = Session::from_state(state_with_two_revisions(&fixture.config()));
    let plan = plan_for("ten_acme").replace(
        r#"{"kind":"copy_as","from":"note","to":"nickname"}"#,
        r#"{"kind":"default_to","to":"counter","value":{"type":"integer","value":5,"epoch_millis":9}}"#,
    );

    let (status, body) = attest(&session, Some(fixture.operator_token()), &plan).await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(!body.contains(r#""fixpoint""#), "{body}");
}

/// The same law on the transform enum, which the F1 fix also widened and
/// which nothing else pins.
#[tokio::test]
async fn a_transform_carrying_a_field_from_another_kind_is_refused() {
    let fixture = Fixture::new("attest-transform-mixed-fields");
    let session = Session::from_state(state_with_two_revisions(&fixture.config()));
    let plan = plan_for("ten_acme").replace(
        r#"{"kind":"copy_as","from":"note","to":"nickname"}"#,
        r#"{"kind":"copy_as","from":"note","to":"nickname","conversion":"integer_to_string"}"#,
    );

    let (status, body) = attest(&session, Some(fixture.operator_token()), &plan).await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(!body.contains(r#""fixpoint""#), "{body}");
}

/// The remaining two arms, pinned the same way. `Boolean` and `Double` were
/// the last wire defaults whose variant nothing checked: mapping either onto
/// `Integer` admitted a plan the registry refuses.
#[tokio::test]
async fn a_boolean_or_double_default_is_refused_by_an_integer_target() {
    let fixture = Fixture::new("attest-typed-default-others");
    let session = Session::from_state(state_with_two_revisions(&fixture.config()));
    for value in [
        r#"{"type":"boolean","value":true}"#,
        r#"{"type":"double","value":1.5}"#,
    ] {
        let plan = plan_for("ten_acme").replace(
            r#"{"kind":"copy_as","from":"note","to":"nickname"}"#,
            &format!(r#"{{"kind":"default_to","to":"counter","value":{value}}}"#),
        );

        let (status, body) = attest(&session, Some(fixture.operator_token()), &plan).await;

        assert_eq!(status, StatusCode::BAD_REQUEST, "{value}: {body}");
        assert!(body.contains("TypeIncompatible"), "{value}: {body}");
    }
}
