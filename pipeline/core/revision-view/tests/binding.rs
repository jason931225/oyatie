//! Binding refusals, pinned at each boundary rather than at one example
//! inside it. A case that is merely too short cannot tell a length equality
//! from a lower bound, and a lowercase-only case cannot tell the hex alphabet
//! from any wider one.

use pipeline_revision_view::{RevisionView, Unknown, ViewIdentity};

const COMMIT: &str = "490d6f701a73e0b096f5c74cb7738a18ed902d96";
const OTHER: &str = "b834f4f9e1acfa4f51c0dda6e81f4772e5c435c0";
const NULL_OBJECT_NAME: &str = "0000000000000000000000000000000000000000";

fn identity() -> ViewIdentity {
    ViewIdentity {
        repository: "oyatie".to_owned(),
        revision: COMMIT.to_owned(),
        producer: "revision-view".to_owned(),
        schema: "v1".to_owned(),
    }
}

fn bound() -> RevisionView {
    identity().bind([("input", "digest")]).expect("bound view")
}

#[test]
fn a_complete_binding_answers_its_inputs_in_order() {
    let view = identity()
        .bind([("third", "d3"), ("first", "d1"), ("second", "d2")])
        .expect("complete binding");
    assert_eq!(
        view.inputs().collect::<Vec<_>>(),
        [("first", "d1"), ("second", "d2"), ("third", "d3")]
    );
    assert_eq!(view.digest_of("first"), Some("d1"));
    assert_eq!(view.digest_of("absent"), None);
    assert_eq!(view.identity().schema, "v1");
}

#[test]
fn every_identity_field_is_required() {
    for (field, candidate) in [
        (
            "repository",
            ViewIdentity {
                repository: String::new(),
                ..identity()
            },
        ),
        (
            "revision",
            ViewIdentity {
                revision: "  ".to_owned(),
                ..identity()
            },
        ),
        (
            "producer",
            ViewIdentity {
                producer: String::new(),
                ..identity()
            },
        ),
        (
            "schema",
            ViewIdentity {
                schema: "\t".to_owned(),
                ..identity()
            },
        ),
    ] {
        assert_eq!(
            candidate.bind([("input", "digest")]),
            Err(Unknown::MissingIdentity(field))
        );
    }
}

#[test]
fn only_an_immutable_object_name_is_a_revision() {
    let too_long = format!("{COMMIT}0");
    let outside_hex = "z".repeat(40);
    let one_past_hex = format!("{}g", &COMMIT[..39]);
    for movable in [
        "dev",
        "HEAD",
        "v1.2.3",
        &COMMIT[..39],
        &COMMIT[..9],
        &COMMIT.to_uppercase(),
        &too_long,
        &outside_hex,
        &one_past_hex,
        NULL_OBJECT_NAME,
    ] {
        let candidate = ViewIdentity {
            revision: movable.to_owned(),
            ..identity()
        };
        assert_eq!(
            candidate.bind([("input", "digest")]),
            Err(Unknown::MutableRevision(movable.to_owned())),
            "{movable}"
        );
    }
    assert!(identity().bind([("input", "digest")]).is_ok());
}

#[test]
fn an_input_classified_twice_refuses_even_when_the_digests_agree() {
    for second in ["different", "same-digest"] {
        assert_eq!(
            identity().bind([("same", "same-digest"), ("same", second)]),
            Err(Unknown::DuplicateInput("same".to_owned())),
            "{second}"
        );
    }
}

/// A name is what the view attests to, so the same name padded differently is
/// the same name — otherwise a caller feeding `git ls-tree` output classifies
/// one input three times and the view stores all three answers.
#[test]
fn surrounding_space_does_not_hide_a_second_classification() {
    assert_eq!(
        identity().bind([(" same", "d1"), ("same\n", "d2")]),
        Err(Unknown::DuplicateInput("same".to_owned()))
    );
}

/// A blank name refuses the way a blank identity field does: a view whose
/// central claim is which input names it covers cannot cover a nameless one.
#[test]
fn an_input_that_names_nothing_refuses() {
    for blank in ["", "   ", "\n", "\t "] {
        assert_eq!(
            identity().bind([(blank, " digest ")]),
            Err(Unknown::UnnamedInput("digest".to_owned())),
            "{blank:?}"
        );
    }
    let unnamed = identity()
        .bind([("", "")])
        .expect_err("a nameless entry is judged before its digest");
    assert_eq!(unnamed, Unknown::UnnamedInput(String::new()));
    assert!(
        Unknown::UnnamedInput("d1".to_owned())
            .reason()
            .contains("d1")
    );
}

#[test]
fn a_blank_digest_and_an_empty_set_both_refuse() {
    assert_eq!(
        identity().bind([(" named ", "   ")]),
        Err(Unknown::MissingDigest("named".to_owned()))
    );
    assert_eq!(
        identity().bind(Vec::<(String, String)>::new()),
        Err(Unknown::NoInputs)
    );
}

#[test]
fn surrounding_space_does_not_split_one_view_into_two() {
    let padded = ViewIdentity {
        repository: " oyatie ".to_owned(),
        revision: format!(" {COMMIT}\n"),
        producer: "\trevision-view ".to_owned(),
        schema: " v1 ".to_owned(),
    };
    assert_eq!(padded.bind([(" input\t", "\ndigest ")]), Ok(bound()));
    let view = bound();
    assert_eq!(view.digest_of(" input\n"), Some("digest"));
    assert_eq!(view.assert_revision(&format!(" {COMMIT}\n")), Ok(()));
}

/// The policy is trim, not strip: space and case inside a name are part of it,
/// so a view that normalized them away would answer for an input it never
/// consumed.
#[test]
fn space_and_case_inside_a_name_are_part_of_it() {
    let view = identity()
        .bind([("A b", "D e"), ("a b", "d e")])
        .expect("distinct names");
    assert_eq!(view.digest_of("A b"), Some("D e"));
    assert_eq!(view.digest_of("ab"), None);
}

#[test]
fn a_view_of_another_revision_refuses_the_one_presented() {
    let view = bound();
    assert_eq!(view.assert_revision(COMMIT), Ok(()));
    let mismatch = view.assert_revision(OTHER).expect_err("mismatch");
    assert_eq!(
        mismatch,
        Unknown::RevisionMismatch {
            bound: COMMIT.to_owned(),
            presented: OTHER.to_owned(),
        }
    );
    assert!(mismatch.reason().contains(OTHER));
}

#[test]
fn the_same_commit_spelled_loosely_is_not_reported_as_a_stale_view() {
    let view = bound();
    for spelling in [COMMIT.to_uppercase(), COMMIT[..9].to_owned()] {
        assert_eq!(
            view.assert_revision(&spelling),
            Err(Unknown::MutableRevision(spelling.clone())),
            "{spelling}"
        );
    }
}
