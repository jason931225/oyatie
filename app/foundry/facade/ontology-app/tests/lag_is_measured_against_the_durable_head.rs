mod facade_support;
mod failing_log;
mod out_of_band;
use facade_support as support;

use axum::http::StatusCode;
use support::{Fixture, Session, scrape, value_of};

#[tokio::test]
async fn a_projection_behind_its_log_reports_the_lag() {
    let fixture = Fixture::new("lag-behind");
    let state = fixture.state();

    out_of_band::append_for(&fixture.action_log_path(), "ten_acme", "idem_out_of_band");

    let tenant = state.tenants.get("ten_acme").expect("the served tenant");
    let status = tenant
        .lock()
        .await
        .sync_status("ten_acme")
        .expect("the head is readable");

    assert_eq!(
        status.lag, 1,
        "one entry is durable and unfolded, so the lag is one: {status:?}"
    );
}

#[tokio::test]
async fn a_process_behind_its_log_is_not_ready() {
    let fixture = Fixture::new("lag-readiness");
    let state = fixture.state();
    assert!(
        foundry_ontology_app::observation::observe(&state).is_caught_up(),
        "a fresh process is caught up"
    );

    out_of_band::append_for(&fixture.action_log_path(), "ten_acme", "idem_unready");

    assert!(
        !foundry_ontology_app::observation::observe(&state).is_caught_up(),
        "a projection that has not folded a durable entry is not ready"
    );
}

/// The EXPOSITION must move, not just the accessor. A gauge whose value a
/// constant would satisfy is the state this signal was in before it had a
/// durable head, and the reason its objective was deleted.
#[tokio::test]
async fn the_exported_gauge_reports_the_real_lag() {
    let fixture = Fixture::new("lag-exported");
    let session = Session::from_state(fixture.state());
    assert_eq!(
        value_of(&scrape(&session).await, "foundry_projection_lag"),
        0,
        "a fresh process is caught up"
    );

    out_of_band::append_for(&fixture.action_log_path(), "ten_acme", "idem_exported");

    assert_eq!(
        value_of(&scrape(&session).await, "foundry_projection_lag"),
        1,
        "the durable entry this process has not folded must show in the exposition"
    );
}

#[tokio::test]
async fn a_tenant_that_cannot_be_sampled_is_counted_as_unknown() {
    let fixture = Fixture::new("lag-unknown");
    let session = Session::from_state(failing_log::state_with_a_failing_log(
        &fixture.config(),
        "the head is unreadable",
    ));

    let body = scrape(&session).await;
    assert_eq!(
        value_of(&body, "foundry_projection_lag_unknown"),
        1,
        "the one served tenant could not be sampled: {body}"
    );
    assert_eq!(
        value_of(&body, "foundry_projection_lag"),
        0,
        "an unsampled tenant contributes nothing to the total, which is why the \
         unknown gauge exists to qualify it: {body}"
    );
}

/// The totals are SUMS across tenants, and `unknown` counts tenants rather
/// than being a flag.
///
/// Two tenants is the smallest fixture that can tell `unknown = 1` from
/// `unknown += 1`, which is this test's subject: `unknown` counts tenants, so
/// equal load separates assignment from accumulation. The TOTALS need more
/// than that — equal lag cannot tell a sum of entries from a count of
/// tenants — which is why the two tests below load their tenants unequally.
#[tokio::test]
async fn the_totals_are_sums_over_every_served_tenant() {
    let fixture = Fixture::new("lag-two-tenants");
    let mut config = fixture.config();
    config.tenants = vec!["ten_acme".into(), "ten_second".into()];

    let state = failing_log::state_with_a_failing_log_from(config, "both heads are gone");
    let seen = foundry_ontology_app::observation::observe(&state);

    assert_eq!(
        seen.unreadable, 2,
        "each unreadable tenant counts, so two tenants are two unknowns: {seen:?}"
    );
    assert!(
        !foundry_ontology_app::observation::observe(&state).is_caught_up(),
        "no tenant could be read, so nothing supports a claim of readiness"
    );
}

#[tokio::test]
async fn the_lag_total_accumulates_across_tenants() {
    let fixture = Fixture::new("lag-two-lagging");
    let mut config = fixture.config();
    config.tenants = vec!["ten_acme".into(), "ten_second".into()];
    let state = foundry_ontology_app::compose(&config).expect("boots");

    append_unequally(&fixture.action_log_path(), "idem_lag");

    let seen = foundry_ontology_app::observation::observe(&state);
    assert_eq!(
        seen.lag, 3,
        "two entries behind on one tenant and one on the other sums to three: {seen:?}"
    );
    assert_eq!(
        (seen.unreadable, seen.contended),
        (0, 0),
        "both tenants were readable and neither was busy: {seen:?}"
    );
}

#[tokio::test]
async fn the_poison_total_accumulates_across_tenants() {
    let fixture = Fixture::new("poison-two-tenants");
    let mut config = fixture.config();
    config.tenants = vec!["ten_acme".into(), "ten_second".into()];

    append_unequally(&fixture.action_log_path(), "idem_poison");

    let state = foundry_ontology_app::compose(&config).expect("boots over poisoned logs");
    let seen = foundry_ontology_app::observation::observe(&state);

    assert_eq!(
        seen.poisoned, 3,
        "two refused entries on one tenant and one on the other sums to three: {seen:?}"
    );
    assert_eq!(
        seen.lag, 0,
        "a poisoned entry advances the fold, so it is consumed, not pending: {seen:?}"
    );
}

#[tokio::test]
async fn the_gauges_are_one_observation_not_several() {
    let fixture = Fixture::new("lag-torn-snapshot");
    out_of_band::append_for(&fixture.action_log_path(), "ten_acme", "idem_torn_poison");
    // The seed must ACTUALLY poison, or the third element below is vacuous:
    // every implementation renders zero when there is no poison to report,
    // and the element stops discriminating without anything going red.
    let baseline = foundry_ontology_app::compose(&fixture.config()).expect("boots");
    assert_eq!(
        foundry_ontology_app::observation::observe(&baseline).poisoned,
        1,
        "the fixture must actually poison, or this test proves less than it says"
    );
    // Released before the double is installed. Harmless to hold while the
    // path is read-only, but stating the intent beats relying on it.
    drop(baseline);
    let session = Session::from_state(failing_log::state_with_a_transiently_failing_head(
        &fixture.config(),
        7,
    ));

    let body = scrape(&session).await;

    assert_eq!(
        (
            value_of(&body, "foundry_projection_lag"),
            value_of(&body, "foundry_projection_lag_unknown"),
            value_of(&body, "foundry_poisoned_entries"),
        ),
        (0, 1, 0),
        "the one tenant could not be read on this scrape, so it contributes to \
         no total and is counted once as unknown; any other triple means the \
         gauges did not come from one observation of one tenant\n{body}"
    );
}

const ENTRIES_BEHIND_ON_ACME: usize = 2;
const ENTRIES_BEHIND_ON_SECOND: usize = 1;

fn append_unequally(log: &std::path::Path, key_prefix: &str) {
    for entry in 1..=ENTRIES_BEHIND_ON_ACME {
        out_of_band::append_for(log, "ten_acme", &format!("{key_prefix}_acme_{entry}"));
    }
    for entry in 1..=ENTRIES_BEHIND_ON_SECOND {
        out_of_band::append_for(log, "ten_second", &format!("{key_prefix}_second_{entry}"));
    }
}
