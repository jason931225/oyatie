//! Both sides of every fault class: the un-injected candidate is admitted and
//! the injected one refuses, naming the class injected.

use pipeline_failure_injection::{
    Candidate, FAULTS, Fault, ProofGap, REQUIRED_CONSUMERS, Refusal, inject, prove, verdict,
};
use pipeline_revision_view::{RevisionView, Unknown, ViewIdentity};

const REVISION: &str = "3a3b6ee38c0de1f2a4b5c6d7e8f90123456789ab";
const PROSE: &str = "pipeline/PLAN.md";
const NATIVE: &str = "pipeline/core/failure-injection/src/lib.rs";
const SURVIVOR: &str = "pipeline/SPEC.md";
const SECOND: &str = "pipeline/z-owner-notes.md";

fn view(inputs: &[(&str, &str)]) -> RevisionView {
    ViewIdentity {
        repository: "oyatie".to_owned(),
        revision: REVISION.to_owned(),
        producer: "pipeline-failure-injection".to_owned(),
        schema: "owner-migration-candidate".to_owned(),
    }
    .bind(inputs.iter().copied())
    .expect("a bound identity for the test fixture")
}

fn passing() -> Vec<(&'static str, bool)> {
    REQUIRED_CONSUMERS
        .iter()
        .map(|name| (*name, true))
        .collect()
}

fn bound_view() -> Option<RevisionView> {
    Some(view(&[(PROSE, "digest-prose"), (NATIVE, "digest-native")]))
}

fn candidate_with(deletions: &[&str], survivors: &[(&str, &str)]) -> Candidate {
    Candidate::new(deletions, survivors, bound_view(), REVISION, &passing())
}

fn with_view(deletions: &[&str], inputs: &[(&str, &str)]) -> Candidate {
    Candidate::new(deletions, &[], Some(view(inputs)), REVISION, &passing())
}

fn with_consumers(consumers: &[(&str, bool)]) -> Candidate {
    Candidate::new(&[PROSE], &[], bound_view(), REVISION, consumers)
}

fn candidate() -> Candidate {
    candidate_with(&[PROSE], &[(SURVIVOR, "this file names nothing deleted\n")])
}

fn offenders(candidate: &Candidate) -> Vec<String> {
    verdict(candidate)
        .err()
        .unwrap_or_default()
        .iter()
        .map(Refusal::reason)
        .collect()
}

fn gaps(candidate: &Candidate) -> Vec<String> {
    prove(candidate)
        .err()
        .unwrap_or_default()
        .iter()
        .map(ProofGap::reason)
        .collect()
}

/// Admitted un-injected. Every fault-class test starts here, so a verdict that
/// refused everything would fail the suite rather than pass it.
fn admitted() -> Candidate {
    let candidate = candidate();
    is_admitted(&candidate);
    candidate
}

fn is_admitted(candidate: &Candidate) {
    assert!(
        offenders(candidate).is_empty(),
        "{:?}",
        offenders(candidate)
    );
}

fn refusals(candidate: &Candidate, fault: Fault) -> Vec<Refusal> {
    verdict(&inject(candidate, PROSE, fault)).expect_err(fault.name())
}

#[test]
fn an_uninjected_candidate_is_admitted() {
    let admitted = admitted();
    assert!(gaps(&admitted).is_empty(), "{:?}", gaps(&admitted));
}

#[test]
fn a_failed_consumer_refuses_and_the_same_candidate_passing_is_admitted() {
    let admitted = admitted();
    let found = refusals(&admitted, Fault::ConsumerFailed);
    assert!(
        matches!(found.as_slice(), [Refusal::ConsumerFailed(name)] if name == REQUIRED_CONSUMERS[0]),
        "{found:?}"
    );
}

#[test]
fn a_missing_consumer_refuses_and_the_same_candidate_reporting_it_is_admitted() {
    let admitted = admitted();
    let found = refusals(&admitted, Fault::ConsumerMissing);
    assert!(
        matches!(found.as_slice(), [Refusal::ConsumerMissing(name)] if name == REQUIRED_CONSUMERS[0]),
        "{found:?}"
    );
}

#[test]
fn a_withheld_input_digest_refuses_and_the_bound_digest_is_admitted() {
    let admitted = admitted();
    let found = refusals(&admitted, Fault::MissingInput);
    assert!(
        matches!(found.as_slice(), [Refusal::UnattestedDeletion(path)] if path == PROSE),
        "{found:?}"
    );
}

/// A revision naming no object refuses for a different reason.
#[test]
fn an_injected_revision_is_a_genuine_mismatch_not_a_malformed_name() {
    let admitted = admitted();
    let found = refusals(&admitted, Fault::RevisionMismatch);
    let [Refusal::RevisionMismatch(Unknown::RevisionMismatch { bound, presented })] =
        found.as_slice()
    else {
        panic!("{found:?}");
    };
    assert_eq!(bound, REVISION);
    assert_ne!(presented, REVISION);
    assert_eq!(presented.len(), REVISION.len());
}

#[test]
fn an_unavailable_view_refuses_and_the_present_view_is_admitted() {
    let admitted = admitted();
    let found = refusals(&admitted, Fault::ViewUnavailable);
    assert!(
        matches!(found.as_slice(), [Refusal::ViewUnavailable]),
        "{found:?}"
    );
}

/// An injection that changed nothing leaves every proof vacuous.
#[test]
fn every_injection_changes_the_verdict_it_is_proving() {
    let admitted = candidate();
    let unchanged: Vec<Fault> = FAULTS
        .into_iter()
        .filter(|fault| offenders(&inject(&admitted, PROSE, *fault)).is_empty())
        .collect();
    assert!(unchanged.is_empty(), "{unchanged:?}");
}

#[test]
fn a_proof_over_a_refused_baseline_is_itself_refused() {
    let mut consumers = passing();
    consumers[0].1 = false;
    let refused = with_consumers(&consumers);
    let found = prove(&refused).expect_err("a refused baseline proves nothing");
    assert!(
        matches!(found.as_slice(), [ProofGap::BaselineRefused(_)]),
        "{found:?}"
    );
}

/// Withholding the only bound input leaves no view at all.
#[test]
fn a_view_binding_only_the_deletion_cannot_prove_a_missing_input() {
    let thin = with_view(&[PROSE], &[(PROSE, "digest-prose")]);
    is_admitted(&thin);
    let found = prove(&thin).expect_err("a thin view cannot distinguish the fault");
    assert!(
        matches!(
            found.as_slice(),
            [ProofGap::WrongClass {
                fault: Fault::MissingInput,
                ..
            }]
        ),
        "{found:?}"
    );
    let thick = candidate_with(&[PROSE], &[]);
    assert!(gaps(&thick).is_empty(), "{:?}", gaps(&thick));
}

#[test]
fn a_surviving_file_still_naming_the_deletion_refuses() {
    let dangling = candidate_with(&[PROSE], &[(SURVIVOR, format!("see {PROSE}.\n").as_str())]);
    let found = offenders(&dangling);
    assert!(
        found.iter().any(|reason| reason.contains(PROSE)),
        "{found:?}"
    );
    let clean = candidate_with(&[PROSE], &[(SURVIVOR, "names nothing deleted\n")]);
    assert!(offenders(&clean).is_empty(), "{:?}", offenders(&clean));
}

/// Untrimmed, the scan seeks a literal prose never spells.
#[test]
fn a_deletion_spelled_with_surrounding_space_still_finds_its_referrer() {
    let padded = candidate_with(
        &[&format!("  {PROSE}  ")],
        &[(SURVIVOR, format!("see {PROSE} for the plan\n").as_str())],
    );
    let found = offenders(&padded);
    assert!(
        found.iter().any(|reason| reason.contains(PROSE)),
        "{found:?}"
    );
}

#[test]
fn a_candidate_that_deletes_nothing_refuses_and_one_deletion_is_admitted() {
    let empty = candidate_with(&[], &[]);
    assert!(matches!(
        verdict(&empty).expect_err("nothing to delete").as_slice(),
        [Refusal::NoDeletions]
    ));
    assert!(gaps(&candidate()).is_empty(), "{:?}", gaps(&candidate()));
}

#[test]
fn a_consumer_reported_under_an_unrequired_name_neither_admits_nor_refuses_on_its_own() {
    let mut extra = passing();
    extra.push(("windows-smoke", false));
    let with_extra = with_consumers(&extra);
    is_admitted(&with_extra);
    let mut short = passing();
    short.retain(|(name, _)| *name != REQUIRED_CONSUMERS[3]);
    short.push(("windows-smoke", true));
    let missing_required = with_consumers(&short);
    let found = offenders(&missing_required);
    assert!(
        found
            .iter()
            .any(|reason| reason.contains(REQUIRED_CONSUMERS[3])),
        "{found:?}"
    );
}

/// One deletion cannot tell "proves this deletion" from "proves the first".
/// The covered set is asserted, so a proof that stopped after the first
/// deletion is not mistaken for a whole one.
#[test]
fn each_deletion_is_proved_against_its_own_digest_not_its_neighbour_s() {
    let both = with_view(
        &[PROSE, SECOND],
        &[
            (PROSE, "digest-prose"),
            (NATIVE, "digest-native"),
            (SECOND, "digest-second"),
        ],
    );
    assert!(offenders(&both).is_empty(), "{:?}", offenders(&both));
    let mut covered = prove(&both).expect("both deletions prove");
    covered.sort();
    let mut want: Vec<(String, Fault)> = Vec::new();
    for deleted in [PROSE, SECOND] {
        want.extend(FAULTS.map(|fault| (deleted.to_owned(), fault)));
    }
    want.sort();
    assert_eq!(covered, want);
    for deleted in [PROSE, SECOND] {
        let found = verdict(&inject(&both, deleted, Fault::MissingInput))
            .expect_err("a withheld digest refuses");
        assert!(
            matches!(found.as_slice(), [Refusal::UnattestedDeletion(path)] if path == deleted),
            "{deleted}: {found:?}"
        );
    }
}

/// Checking only the first deletion's digest would admit this candidate.
#[test]
fn an_unbound_deletion_refuses_beside_a_bound_neighbour() {
    let partial = with_view(
        &[PROSE, SECOND],
        &[(PROSE, "digest-prose"), (NATIVE, "digest-native")],
    );
    let found = verdict(&partial).expect_err("an unbound deletion refuses");
    let names_second = matches!(found.as_slice(), [Refusal::UnattestedDeletion(p)] if p == SECOND);
    assert!(names_second, "{found:?}");
    assert!(
        matches!(
            prove(&partial).expect_err("nothing is proved").as_slice(),
            [ProofGap::BaselineRefused(_)]
        ),
        "{:?}",
        gaps(&partial)
    );
}
