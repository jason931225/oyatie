//! Proof that the kernel's compile-time neutrality rule (ADR-0637 D1) is capable of going RED.
//!
//! A const assertion cannot be demonstrated failing without breaking the build, so this file is
//! the demonstration: it drives the SAME predicates over the SAME needle sets the kernel's own
//! `const` block asserts with. Sharing the predicate is deliberate — a test that reimplemented
//! the search could stay green while the enforced one was broken.
//!
//! The needles are safe to spell out HERE because nothing scans this file. That is also why they
//! live as bytes in the kernel: a needle written as text in the scanned file would be a needle in
//! the haystack.
//!
//! NOT ENFORCED: source/target LANGUAGE names. A language-name scan is unsound as written — the
//! plausible slugs are ordinary English substrings — so the kernel carries language neutrality
//! structurally instead, with `LanguagePair` as data.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "an integration test asserts its invariants by panicking"
)]

use port_engine_kernel::{
    FORBIDDEN_CORPUS_TOKENS, UNSCANNED_CODE_KEYWORDS, contains_token, contains_word,
};

/// The exact bytes the compile-time assertion evaluated. Read the same way, so this file cannot
/// certify a different copy of the kernel from the one that was built.
const KERNEL_SOURCE: &str = include_str!("../src/lib.rs");

/// The seam test's bytes. The build corpus-scans this file at compile time; the keyword rule over
/// it is asserted HERE instead, because two more const passes crossed rustc's
/// `long_running_const_eval` budget (measured). Its real completeness guarantee is structural
/// anyway: the seam test's Buck target names exactly one source file.
const SEAM_TEST_SOURCE: &str = include_str!("seams.rs");

/// The forbidden corpus tokens present in `haystack`, rendered for a readable failure.
fn corpus_tokens_in(haystack: &str) -> Vec<String> {
    FORBIDDEN_CORPUS_TOKENS
        .into_iter()
        .filter(|token| contains_token(haystack.as_bytes(), token))
        .map(|token| String::from_utf8(token.to_vec()).unwrap())
        .collect()
}

/// The unscanned-code keywords present in `haystack`, rendered for a readable failure. Uses
/// `contains_word`, exactly as the build does — the boundary rule IS the fix this file exists to
/// hold in place.
fn unscanned_code_keywords_in(haystack: &str) -> Vec<String> {
    UNSCANNED_CODE_KEYWORDS
        .into_iter()
        .filter(|keyword| contains_word(haystack.as_bytes(), keyword))
        .map(|keyword| String::from_utf8(keyword.to_vec()).unwrap())
        .collect()
}

#[test]
fn the_kernel_source_the_build_asserted_over_is_corpus_free() {
    assert!(
        corpus_tokens_in(KERNEL_SOURCE).is_empty(),
        "kernel carries corpus token(s) {:?}; corpus-specific behaviour and vocabulary belong in \
         corpus policy (specs/k8s-port/**), never in the neutral engine (ADR-0637 D1)",
        corpus_tokens_in(KERNEL_SOURCE)
    );
}

#[test]
fn the_seam_test_the_build_asserted_over_is_corpus_free_and_self_contained() {
    assert!(
        corpus_tokens_in(SEAM_TEST_SOURCE).is_empty(),
        "seam test carries corpus token(s) {:?}; a corpus-specific fixture is a corpus-specific \
         branch with extra steps (ADR-0637 D1)",
        corpus_tokens_in(SEAM_TEST_SOURCE)
    );
    assert!(
        unscanned_code_keywords_in(SEAM_TEST_SOURCE).is_empty(),
        "seam test would compile code from an unscanned file via {:?}; exempting it would just \
         move the unscanned file one directory over",
        unscanned_code_keywords_in(SEAM_TEST_SOURCE)
    );
}

#[test]
fn the_kernel_is_provably_one_file_so_the_scan_is_complete() {
    // The defect the first draft of this gate had: it asserted a hand-kept file COUNT, which a
    // new `pub mod second;` never touches. This asserts the property that makes the count
    // irrelevant — no construct in the kernel can pull in code from a file the scan never reads.
    assert!(
        unscanned_code_keywords_in(KERNEL_SOURCE).is_empty(),
        "kernel would compile code from an unscanned file via {:?}",
        unscanned_code_keywords_in(KERNEL_SOURCE)
    );
}

#[test]
fn a_planted_corpus_token_is_caught() {
    // A gate that has never been shown to go red is not evidence of anything.
    let planted = "//! this seam is specialised for the Kubernetes control plane\npub fn f() {}\n";
    assert_eq!(corpus_tokens_in(planted), vec!["kube".to_owned()]);
}

#[test]
fn the_bare_prefix_catches_every_compound_including_the_two_the_earlier_list_missed() {
    // `kubeadm` and `kube-scheduler` passed the earlier compound-enumerating list. `apimachinery`
    // is the fourth token the admission plan names and the earlier list had no entry for it at all.
    for compound in [
        "kubernetes",
        "kubelet",
        "kubectl",
        "kubeadm",
        "kube-proxy",
        "kube-apiserver",
        "kube-scheduler",
    ] {
        assert_eq!(
            corpus_tokens_in(compound),
            vec!["kube".to_owned()],
            "{compound} escaped the prefix rule"
        );
    }
    assert_eq!(
        corpus_tokens_in("k8s.io/apimachinery/pkg/runtime"),
        vec!["k8s".to_owned(), "apimachinery".to_owned()]
    );
}

#[test]
fn the_scan_is_case_insensitive_and_finds_every_token_not_just_the_first() {
    assert_eq!(
        corpus_tokens_in("K8S and KubeLet and ETCD"),
        vec!["kube".to_owned(), "k8s".to_owned(), "etcd".to_owned()]
    );
}

#[test]
fn every_submodule_form_and_a_source_splice_are_caught() {
    // The defect proven against the SECOND draft of this gate, by execution: it spelled the needle
    // `"mod "` with a trailing space, so `pub mod\nsecond;` plus a corpus-carrying src/second.rs
    // BUILT and reported `Pass 2 / 27 passed / 0 failed`. One byte from the planted defect the
    // same draft's commit message claimed to have closed. Every whitespace form is asserted here
    // because the grammar accepts every one of them.
    for declaration in [
        "pub mod second;",
        "pub mod\nsecond;",
        "mod\tsecond;",
        "mod\r\nsecond;",
        "pub(crate) mod  second;",
        "mod/*c*/second;",
        "#[path = \"elsewhere.rs\"]\nmod hidden;",
        // The INLINE form is refused too. Its body would be scanned, so this is stricter than the
        // completeness argument strictly needs — deliberately, because separating the two forms
        // means parsing, and an edge case in a neutrality rule is a place to hide a token.
        "#[cfg(test)]\nmod tests {\n}\n",
    ] {
        assert_eq!(
            unscanned_code_keywords_in(declaration),
            vec!["mod".to_owned()],
            "submodule declaration escaped the boundary rule: {declaration:?}"
        );
    }

    // Same class on the other needle: `"include!"` with a fixed bang missed the space form and the
    // path-qualified form, both of which are valid Rust.
    for splice in [
        "include!(\"generated.rs\");",
        "include !(\"generated.rs\");",
        "include\n!(\"generated.rs\");",
        "core::include!(\"generated.rs\");",
        "std::include!(\"generated.rs\");",
    ] {
        assert_eq!(
            unscanned_code_keywords_in(splice),
            vec!["include".to_owned()],
            "source splice escaped the boundary rule: {splice:?}"
        );
    }
}

#[test]
fn neutral_source_is_not_falsely_accused() {
    assert!(corpus_tokens_in("pub struct LanguagePair { source: String }").is_empty());
    // The identifier-boundary rule is what keeps ordinary prose and longer identifiers out of the
    // net — without it the needles would have to guess the following byte, which is the hole.
    assert!(unscanned_code_keywords_in("the canonical semantic model of the corpus").is_empty());
    assert!(unscanned_code_keywords_in("a modular renderer and its submodules").is_empty());
    assert!(unscanned_code_keywords_in("let modulus = a % b;").is_empty());
    // The scan itself is built on include_str!, so this one is load-bearing, not decorative.
    assert!(unscanned_code_keywords_in("const S: &str = include_str!(\"lib.rs\");").is_empty());
    assert!(unscanned_code_keywords_in("include_bytes!(\"blob\")").is_empty());
}

#[test]
fn the_substring_predicate_behaves_at_its_edges() {
    assert!(!contains_token(b"", b"kube"));
    assert!(!contains_token(b"ku", b"kube"));
    assert!(contains_token(b"kube", b"kube"));
    assert!(contains_token(b"xxkube", b"kube"));
    assert!(!contains_token(b"anything", b""));
}

#[test]
fn the_word_predicate_anchors_on_identifier_boundaries_at_both_ends() {
    // Whole word at the ends of the haystack, where there is no neighbouring byte at all.
    assert!(contains_word(b"mod", b"mod"));
    assert!(contains_word(b"; mod", b"mod"));
    assert!(contains_word(b"mod ;", b"mod"));
    // Non-identifier neighbours are boundaries; identifier neighbours are not.
    assert!(contains_word(b"::mod!", b"mod"));
    assert!(!contains_word(b"amod", b"mod"));
    assert!(!contains_word(b"mods", b"mod"));
    assert!(!contains_word(b"_mod_", b"mod"));
    assert!(!contains_word(b"mod9", b"mod"));
    // A later whole-word occurrence is still found after an earlier non-word one — the scan does
    // not stop at the first byte match, which is how `include_str!` can precede a real `include!`.
    assert!(contains_word(b"include_str! then include!", b"include"));
    // Case folding matches contains_token; degenerate inputs are false, not a panic.
    assert!(contains_word(b"MOD x", b"mod"));
    assert!(!contains_word(b"", b"mod"));
    assert!(!contains_word(b"anything", b""));
}
