//! `POST /v1/migrations/run` — what it refuses, and what it leaves behind.
//!
//! Split from the executing suite because both outgrew one file. A refusal on
//! a WRITING surface has two halves and the second is the one that matters:
//! the status is visible, and "changed nothing" is what the operator finds out
//! only later. Every test here asserts both.

mod facade_support;
mod migration_support;

use axum::http::StatusCode;
use facade_support::{Fixture, Session, scrape, value_of};
use migration_support::{
    action_head, denial_head, plan_for, run, state_with_two_revisions, write_owing,
};

#[test]
fn cyclic_plan_is_refused_without_writes_and_releases_the_tenant() {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(assert_cycle_refusal());
        tx.send(()).unwrap();
    });
    rx.recv_timeout(std::time::Duration::from_secs(30))
        .expect("cyclic admission must terminate and release the tenant");
}

async fn assert_cycle_refusal() {
    let fixture = Fixture::new("run-cycle");
    let config = fixture.config();
    let mut state = state_with_two_revisions(&config);
    let tenant = state.tenants.get_mut("ten_acme").unwrap().get_mut();
    for engine in [
        &mut tenant.projection.registry_input,
        &mut tenant.projection.engine,
    ] {
        let id = data_ontology_kernel::EntityTypeId::new("ety_record").unwrap();
        let mut definition = engine.entity_type("ten_acme", &id).unwrap().clone();
        definition.revision = 3;
        engine.evolve_entity_type(definition).unwrap();
    }
    let session = Session::from_state(state);
    let token = Some(fixture.operator_token());
    let (status, body) = session.post(token, r#"{"object_ref":"ent_alpha","action_type":"aty_record_write","idempotency_key":"cycle-seed","occurred_at_epoch_seconds":1700000000,"properties":{"name":"Ada","note":"A","nickname":"B"}}"#).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let heads = (action_head(&config), denial_head(&config));
    let cycle = plan_for("ten_acme")
        .replace("\"from_revision\":1", "\"from_revision\":2")
        .replace("\"to_revision\":2", "\"to_revision\":3")
        .replace(r#"{"kind":"copy_as","from":"note","to":"nickname"}"#,
            r#"{"kind":"copy_as","from":"note","to":"nickname"},{"kind":"copy_as","from":"nickname","to":"note"}"#);
    let (status, body) = run(&session, token, &cycle).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body.contains("CyclicTransforms"), "{body}");
    assert_eq!((action_head(&config), denial_head(&config)), heads);
    let (status, body) = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        session.get(token, "/v1/objects/ent_alpha/history"),
    )
    .await
    .expect("tenant lock released");
    assert_eq!(status, StatusCode::OK, "{body}");
}

/// A refused caller changes nothing, in either log.
///
/// Both heads are asserted unchanged rather than merely "no error", and the
/// setup is what makes that discriminating: an object IS owed here, so a run
/// that reached the runner would certainly grow the action log. A gate moved
/// after `run_to_fixpoint` would refuse with the same status and the same
/// body, having already written every upcast.
///
/// The denial trail is asserted UNCHANGED, not grown. It records a submission
/// the domain refused (`writer.rs`), which is a different event from a
/// credential the PDP refused; asserting it grew here would invent a
/// behaviour the write path does not have either.
#[tokio::test]
async fn a_roleless_caller_may_not_run_and_the_log_is_untouched() {
    let fixture = Fixture::new("run-roleless");
    let config = fixture.config();
    let session = Session::from_state(state_with_two_revisions(&config));
    write_owing(
        &session,
        Some(fixture.operator_token()),
        "ent_alpha",
        "idem_1",
    )
    .await;
    let (actions, denials) = (action_head(&config), denial_head(&config));

    let (status, body) = run(
        &session,
        Some(fixture.roleless_token()),
        &plan_for("ten_acme"),
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(
        (action_head(&config), denial_head(&config)),
        (actions, denials),
        "a refused run writes NOTHING, to either log: {body}"
    );
}
#[tokio::test]
async fn a_plan_naming_another_tenant_is_refused_before_it_runs() {
    let fixture = Fixture::new("run-foreign");
    let config = fixture.config();
    let session = Session::from_state(state_with_two_revisions(&config));
    write_owing(
        &session,
        Some(fixture.operator_token()),
        "ent_alpha",
        "idem_1",
    )
    .await;
    let actions = action_head(&config);

    let (status, body) = run(
        &session,
        Some(fixture.operator_token()),
        &plan_for("ten_other"),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(
        body.contains("the plan names a tenant other than the credential's"),
        "{body}"
    );
    assert_eq!(action_head(&config), actions, "and nothing ran: {body}");
}
/// An invalid plan touches nothing. `run_to_fixpoint` validates first and
/// this asserts the consequence, not the call.
#[tokio::test]
async fn an_invalid_plan_touches_nothing() {
    let fixture = Fixture::new("run-invalid");
    let config = fixture.config();
    let session = Session::from_state(state_with_two_revisions(&config));
    write_owing(
        &session,
        Some(fixture.operator_token()),
        "ent_alpha",
        "idem_1",
    )
    .await;
    let actions = action_head(&config);
    let absent = plan_for("ten_acme").replace("ety_record", "ety_absent");

    let (status, body) = run(&session, Some(fixture.operator_token()), &absent).await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body.contains("UnknownEntityType"), "{body}");
    assert_eq!(action_head(&config), actions, "and nothing ran: {body}");
}
/// EVERY exit this surface takes is counted, and counted as a WRITE.
///
/// All nine of them: the surface mutates, so its outcomes belong to the
/// submission counters and not the read ones — an operator watching write
/// volume must see a migration in it. An earlier version of this test claimed
/// "every exit" while exercising five, leaving four `submit_refused()` calls
/// deletable with the suite still green.
///
/// Exact totals rather than "greater than", because a site that stops
/// counting cannot then hide behind one that starts, and because counting a
/// run under the read meter would leave both totals plausible and both wrong.
#[tokio::test]
async fn every_exit_is_counted_against_the_write_meters() {
    let fixture = Fixture::new("run-metering");
    let config = fixture.config();
    let session = Session::from_state(state_with_two_revisions(&config));
    let token = Some(fixture.operator_token());
    write_owing(&session, token, "ent_alpha", "idem_1").await;

    let unknown_conversion = plan_for("ten_acme").replace(
        r#"{"kind":"copy_as","from":"note","to":"nickname"}"#,
        r#"{"kind":"convert_as","from":"note","to":"nickname","conversion":"nope"}"#,
    );
    let outcomes = [
        // Served.
        run(&session, token, &plan_for("ten_acme")).await.0,
        // No credential, then one this process does not recognise.
        run(&session, None, &plan_for("ten_acme")).await.0,
        run(&session, Some("not-a-token"), &plan_for("ten_acme"))
            .await
            .0,
        // A body that is not a plan.
        run(&session, token, "{not a plan").await.0,
        // A credential the policy refuses.
        run(
            &session,
            Some(fixture.roleless_token()),
            &plan_for("ten_acme"),
        )
        .await
        .0,
        // A plan naming a tenant the credential does not carry.
        run(&session, token, &plan_for("ten_other")).await.0,
        // A credential whose own tenant this process does not serve.
        run(
            &session,
            Some(fixture.foreign_token()),
            &plan_for("ten_other"),
        )
        .await
        .0,
        // A transform vocabulary this process does not perform.
        run(&session, token, &unknown_conversion).await.0,
        // A plan the registry refuses.
        run(
            &session,
            token,
            &plan_for("ten_acme").replace("ety_record", "ety_absent"),
        )
        .await
        .0,
    ];

    assert_eq!(outcomes[0], StatusCode::OK, "the served run");
    assert!(
        outcomes[1..].iter().all(|status| status.is_client_error()),
        "the other eight must all refuse: {outcomes:?}"
    );
    let metrics = scrape(&session).await;
    assert_eq!(
        value_of(&metrics, "foundry_action_submit_served_total"),
        2,
        "the fixture write and the run: {metrics}"
    );
    assert_eq!(
        value_of(&metrics, "foundry_action_submit_refused_total"),
        8,
        "eight refusals, eight counted: {metrics}"
    );
}

/// A credential whose own tenant this process does not serve is refused, and
/// the refusal is counted.
///
/// It needs its own fixture: the foreign operator is refused earlier, by the
/// PDP, so the roster is varied instead — the credential is well-formed and
/// policy-clean, and the process simply does not serve its tenant. Without
/// this the site is unreachable, and an unreachable site is one whose counter
/// can be deleted unnoticed.
#[tokio::test]
async fn a_credential_for_an_unserved_tenant_is_refused_and_counted() {
    let fixture = Fixture::new("run-unserved-tenant");
    let session = fixture.unserved_session();

    let (status, body) = run(
        &session,
        Some(fixture.operator_token()),
        &plan_for("ten_acme"),
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert!(body.contains("does not serve"), "{body}");
    assert_eq!(
        value_of(
            &scrape(&session).await,
            "foundry_action_submit_refused_total"
        ),
        1,
        "the refusal this process made is a refusal it counted"
    );
}

/// A plan naming an action the registry does not hold is refused HERE.
///
/// The behavioural claim behind the `validate` change, at the surface it is
/// about. Before it, this plan passed validation, reached the writer, and was
/// refused one object at a time as a bare `refused` count with no reason in
/// it — while `attest` answered the same plan with pending objects, the
/// fixpoint claim over an unexecutable plan that module forbids itself.
#[tokio::test]
async fn a_plan_naming_an_unregistered_action_is_refused_before_it_writes() {
    let fixture = Fixture::new("run-unregistered-action");
    let config = fixture.config();
    let session = Session::from_state(state_with_two_revisions(&config));
    let token = Some(fixture.operator_token());
    write_owing(&session, token, "ent_alpha", "idem_1").await;
    let actions = action_head(&config);
    let plan = plan_for("ten_acme").replace("aty_record_write", "aty_never_registered");

    let (status, body) = run(&session, token, &plan).await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body.contains("UnknownActionType"), "{body}");
    assert_eq!(action_head(&config), actions, "and nothing ran: {body}");
}
