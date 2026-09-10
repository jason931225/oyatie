use dependency_declarations_reconcile::{
    DeclarationRefusal, ExecutionChannel, ExecutionToolchainAnalysisRefusal, ExecutionVersionDelta,
    PatchOnlyExecutionToolchainDecision, PatchOnlyExecutionToolchainRefusal, ToolchainSide,
};

use crate::support::{analyze, apply_policy, declaration};

const NIGHTLY: &str = "nightly-2026-09-09";

fn channel_of(channel: &str) -> ExecutionChannel {
    analyze(
        &declaration(channel),
        &declaration(channel),
        "1.96.0",
        "1.96.0",
    )
    .unwrap_or_else(|refusal| panic!("{channel} must be admitted, got {refusal:?}"))
    .candidate()
    .execution()
    .clone()
}

fn refusal_of(channel: &str) -> DeclarationRefusal {
    match analyze(
        &declaration("1.98.0"),
        &declaration(channel),
        "1.96.0",
        "1.96.0",
    ) {
        Err(ExecutionToolchainAnalysisRefusal::InvalidToolchain(
            ToolchainSide::Candidate,
            reason,
        )) => reason,
        other => panic!("{channel} must be refused as a candidate toolchain, got {other:?}"),
    }
}

#[test]
fn exact_dated_nightly_channels_are_admitted_as_their_own_identity() {
    let mut rejected = Vec::new();
    for channel in [
        "nightly-2026-09-09",
        "nightly-2026-01-01",
        "nightly-2024-02-29",
        "nightly-2026-12-31",
    ] {
        match analyze(
            &declaration(channel),
            &declaration(channel),
            "1.96.0",
            "1.96.0",
        ) {
            Ok(analysis) => {
                let execution = analysis.candidate().execution();
                if execution.dated_nightly().is_none()
                    || execution.stable().is_some()
                    || execution.to_string() != channel
                {
                    rejected.push(format!("{channel}: rendered as {execution}"));
                }
            }
            Err(refusal) => rejected.push(format!("{channel}: {refusal}")),
        }
    }
    assert!(
        rejected.is_empty(),
        "dated nightlies refused: {rejected:#?}"
    );
}

#[test]
fn a_floating_nightly_channel_is_refused_by_its_own_variant() {
    assert!(matches!(
        refusal_of("nightly"),
        DeclarationRefusal::FloatingNightlyChannel("toolchain.channel", _)
    ));
    let rendered = refusal_of("nightly").to_string();
    assert!(rendered.contains("floats"), "{rendered}");
    assert!(rendered.contains("nightly-YYYY-MM-DD"), "{rendered}");
}

#[test]
fn nightly_channels_without_an_exact_calendar_date_are_refused() {
    let mut admitted = Vec::new();
    for channel in [
        "nightly-",
        "nightly-2026",
        "nightly-2026-09",
        "nightly-2026-9-4",
        "nightly-2026-13-01",
        "nightly-2026-00-09",
        "nightly-2026-09-00",
        "nightly-2026-09-31",
        "nightly-2026-02-29",
        "nightly-2026-02-30",
        "nightly-2100-02-29",
        "nightly-2026-09-09-aarch64-apple-darwin",
        "nightly-2026-09-09 ",
        "nightly-20x6-09-09",
    ] {
        match refusal_of(channel) {
            DeclarationRefusal::InvalidNightlyDate("toolchain.channel", _) => {}
            other => admitted.push(format!("{channel}: {other:?}")),
        }
    }
    assert!(
        admitted.is_empty(),
        "malformed nightly dates not refused as invalid dates: {admitted:#?}"
    );
}

#[test]
fn malformed_stable_channels_stay_refused_as_invalid_stable_versions() {
    let mut escaped = Vec::new();
    for channel in [
        "1.98.0-aarch64-apple-darwin",
        "1.98.0-alpha.1",
        "1.98.0+build",
        "1.98",
        "stable",
        "beta",
        "nightlyish",
        "custom-toolchain",
    ] {
        match refusal_of(channel) {
            DeclarationRefusal::InvalidStableVersion("toolchain.channel", _) => {}
            other => escaped.push(format!("{channel}: {other:?}")),
        }
    }
    assert!(
        escaped.is_empty(),
        "malformed stable channels no longer refused as invalid stable versions: {escaped:#?}"
    );
}

#[test]
fn transitions_between_channel_identities_are_classified_without_a_total_order() {
    let mut wrong = Vec::new();
    for (protected, candidate, expected) in [
        (NIGHTLY, NIGHTLY, ExecutionVersionDelta::Unchanged),
        (
            NIGHTLY,
            "nightly-2026-09-10",
            ExecutionVersionDelta::NightlyDateForward,
        ),
        (
            NIGHTLY,
            "nightly-2026-09-08",
            ExecutionVersionDelta::NightlyDateBackward,
        ),
        (
            NIGHTLY,
            "nightly-2027-01-01",
            ExecutionVersionDelta::NightlyDateForward,
        ),
        ("1.98.0", NIGHTLY, ExecutionVersionDelta::ChannelChanged),
        (NIGHTLY, "1.98.0", ExecutionVersionDelta::ChannelChanged),
        ("2.0.0", NIGHTLY, ExecutionVersionDelta::ChannelChanged),
        (NIGHTLY, "1.0.0", ExecutionVersionDelta::ChannelChanged),
    ] {
        let actual = analyze(
            &declaration(protected),
            &declaration(candidate),
            "1.96.0",
            "1.96.0",
        )
        .unwrap()
        .delta()
        .execution();
        if actual != expected {
            wrong.push(format!(
                "{protected} -> {candidate}: {actual:?} != {expected:?}"
            ));
        }
    }
    assert!(wrong.is_empty(), "misclassified transitions: {wrong:#?}");
}

#[test]
fn a_dated_nightly_satisfies_every_stable_msrv_floor() {
    let nightly = channel_of(NIGHTLY);
    let mut unmet = Vec::new();
    for msrv in ["1.0.0", "1.96.0", "1.98.0", "99.99.99"] {
        let floor = channel_of(msrv);
        let floor = floor.stable().expect("stable channel exposes its version");
        if !nightly.meets_msrv(floor) {
            unmet.push(msrv.to_owned());
        }
    }
    assert!(
        unmet.is_empty(),
        "nightly reported below MSRV floors: {unmet:#?}"
    );
}

#[test]
fn a_stable_channel_still_answers_the_msrv_floor_by_semver() {
    let stable = channel_of("1.98.0");
    let mut wrong = Vec::new();
    for (floor, expected) in [("1.96.0", true), ("1.98.0", true), ("1.98.1", false)] {
        let floor_channel = channel_of(floor);
        let floor_version = floor_channel
            .stable()
            .expect("stable channel exposes its version");
        if stable.meets_msrv(floor_version) != expected {
            wrong.push(format!("floor {floor}: expected {expected}"));
        }
    }
    assert!(wrong.is_empty(), "stable MSRV answers changed: {wrong:#?}");
}

#[test]
fn a_nightly_older_than_the_msrv_release_is_still_admitted_by_the_floor() {
    let stale = channel_of("nightly-2020-01-01");
    let floor = channel_of("1.98.0");
    let floor = floor.stable().expect("stable channel exposes its version");
    assert!(
        stale.meets_msrv(floor),
        "the MSRV floor is not evaluable against a nightly, so this admission \
         is vacuous; a stale nightly is caught by compilation, not by this gate"
    );
}

#[test]
fn the_patch_only_policy_holds_an_unchanged_dated_nightly_and_refuses_every_move() {
    let unchanged = apply_policy(
        &declaration(NIGHTLY),
        &declaration(NIGHTLY),
        "1.98.0",
        "1.98.0",
    )
    .unwrap();
    let PatchOnlyExecutionToolchainDecision::Unchanged(channel) = unchanged else {
        panic!("an unchanged dated nightly must be admitted unchanged")
    };
    assert_eq!(channel.to_string(), NIGHTLY);

    let mut admitted = Vec::new();
    for (protected, candidate, expected) in [
        (
            NIGHTLY,
            "nightly-2026-09-10",
            ExecutionVersionDelta::NightlyDateForward,
        ),
        (
            NIGHTLY,
            "nightly-2026-09-08",
            ExecutionVersionDelta::NightlyDateBackward,
        ),
        ("1.98.0", NIGHTLY, ExecutionVersionDelta::ChannelChanged),
        (NIGHTLY, "1.98.0", ExecutionVersionDelta::ChannelChanged),
    ] {
        match apply_policy(
            &declaration(protected),
            &declaration(candidate),
            "1.98.0",
            "1.98.0",
        ) {
            Err(PatchOnlyExecutionToolchainRefusal::VersionDeltaNotAdmitted(delta, _, _))
                if delta == expected => {}
            other => admitted.push(format!("{protected} -> {candidate}: {other:?}")),
        }
    }
    assert!(
        admitted.is_empty(),
        "channel moves not refused as unqualified: {admitted:#?}"
    );
}
