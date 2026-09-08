//! LAW B, executable: a doc must not claim what its signature cannot perform.
//!
//! THE LAW. A doc block that makes a negation-or-exclusivity claim about who can
//! PRODUCE a verified value, attached to an item whose verifier arrives as
//! `&dyn`, is a violation unless the block also states the deployment
//! obligation. An out-of-crate type can implement the verifier and mint the
//! value by passing itself, so the refusal the prose asserts is a property of a
//! composition and never of the type system.
//!
//! WHY THIS FILE EXISTS. Nine rounds of review found this shape by hand, and
//! round 9 found a ninth site inside the candidate list a mechanical sweep had
//! already produced: the sweep keyed on the eight CORRECTED WORDINGS, so it
//! could only ever re-find what had already been fixed. A key built from the
//! fix cannot certify the class. This key is built from grammar and from
//! signatures instead.
//!
//! WHAT IT KEYS ON, in three conjuncts:
//!
//! 1. GRAMMAR. Inside one doc block, a negation-or-exclusivity token followed
//!    within fourteen tokens by a production verb, whose object within the next
//!    four tokens is either a minted wrapper type or the anaphor `one`/`any`.
//!    The object requirement is what separates a claim about production from
//!    prose that merely contains both a "not" and a "signs". Every production
//!    verb in the window is tried, not only the first: the ninth site found in
//!    round 9 has `Signed` standing between its negation and its verb, and a
//!    scan that stopped at the first verb would have missed it again.
//! 2. SIGNATURE. The minted set is computed from the tree: every type returned
//!    by a function that takes a `&dyn ...Verifier`. A block is in scope only
//!    when it is ATTACHED to such a function, to a minted type's declaration,
//!    to a trait declaring such a method, or is the `//!` doc of a file that
//!    declares one. Scope is never inferred from a file's name or from a
//!    mention in prose.
//! 3. DISCHARGE. The block states the obligation, in the wording the wave
//!    already settled on: `deployment obligation`, or a reference to
//!    `MovementActionResultAuthority`, where that reasoning is written once.
//!    Keying the DISCHARGE on a wording is legitimate precisely because the
//!    GENERATOR is not: the discharge is a marker this crate declares, while
//!    the violation is whatever grammar a future author invents.
//!
//! THE BLIND SPOT, stated because every key has one. This key cannot see:
//! (a) an exclusivity claim with no negation-or-exclusivity token in the window
//!     -- "the sole route is X" phrased as "X is the route";
//! (b) a claim whose production object is neither a minted type nor `one`/`any`
//!     -- "no adapter can hold that role" names a role, not a value;
//! (c) a claim in a doc block attached to a type alias, a constant, or a field,
//!     since none of those is a minter;
//! (d) a claim that is TRUE -- ten `verify_*` functions in this crate take no
//!     verifier at all, and an exclusivity claim about the wrappers they mint
//!     is sound. Conjunct 2 exists to keep those out: caveating a genuinely
//!     type-level refusal would be this same defect inverted.
//!
//! It asserts that the offending set is EMPTY and prints its members. It
//! asserts no count and no non-zero quantity: a live finding count goes red
//! exactly when the burn-down succeeds.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repo root")
}

/// Both crates of the wave. The law is stated over the wave and not over a
/// crate, so a key scoped to one directory would let a violation escape by
/// moving a file.
fn wave_source_files() -> Vec<PathBuf> {
    let root = repo_root();
    let mut out = Vec::new();
    for relative in [
        "cell/ports/placement/src",
        "tenancy/core/cell-assignment/src",
    ] {
        let directory = root.join(relative);
        let mut entries: Vec<PathBuf> = fs::read_dir(&directory)
            .unwrap_or_else(|error| panic!("read {}: {error}", directory.display()))
            .map(|entry| entry.expect("dir entry").path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
            .collect();
        entries.sort();
        assert!(
            !entries.is_empty(),
            "no sources under {} -- the sweep would certify a zero it never looked for",
            directory.display()
        );
        out.extend(entries);
    }
    out
}

fn words(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for character in text.chars() {
        if character.is_ascii_alphabetic() || character == '_' || character == '\'' {
            current.push(character);
        } else if !current.is_empty() {
            out.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

const NEGATIONS: &[&str] = &[
    "no",
    "not",
    "cannot",
    "never",
    "only",
    "sole",
    "solely",
    "nothing",
    "nobody",
    "none",
    "impossible",
    "unimplementable",
    "refuses",
    "refuse",
    "prevents",
    "forbids",
    "denies",
];

/// Production stems. A verb matches when the word equals the stem or the stem
/// plus one of the inflections below, so a future author's `minting` and
/// `constructs` are covered without listing them.
const PRODUCTION_STEMS: &[&str] = &[
    "produce",
    "mint",
    "construct",
    "obtain",
    "creat",
    "forge",
    "fabricat",
    "yield",
    "issu",
    "sign",
    "implement",
    "return",
    "inhabit",
];

const INFLECTIONS: &[&str] = &["", "s", "d", "e", "es", "ed", "ing", "ted", "ting"];

fn is_negation(word: &str) -> bool {
    let lowered = word.to_ascii_lowercase();
    NEGATIONS.contains(&lowered.as_str())
}

fn is_production(word: &str) -> bool {
    let lowered = word.to_ascii_lowercase();
    PRODUCTION_STEMS.iter().any(|stem| {
        INFLECTIONS.iter().any(|inflection| {
            lowered.len() == stem.len() + inflection.len()
                && lowered.starts_with(stem)
                && lowered.ends_with(inflection)
        })
    })
}

const NEGATION_TO_VERB_WINDOW: usize = 14;
const VERB_TO_OBJECT_WINDOW: usize = 4;

/// The grammatical conjunct: a negation, a production verb within the window,
/// and an object that is the produced value.
fn production_claim(text: &str, minted: &BTreeSet<String>) -> Option<String> {
    let tokens = words(text);
    for (index, token) in tokens.iter().enumerate() {
        if !is_negation(token) {
            continue;
        }
        let verb_limit = (index + 1 + NEGATION_TO_VERB_WINDOW).min(tokens.len());
        for verb_index in (index + 1)..verb_limit {
            if !is_production(&tokens[verb_index]) {
                continue;
            }
            let object_limit = (verb_index + 1 + VERB_TO_OBJECT_WINDOW).min(tokens.len());
            for object_index in (verb_index + 1)..object_limit {
                let object = &tokens[object_index];
                let is_anaphor =
                    object.eq_ignore_ascii_case("one") || object.eq_ignore_ascii_case("any");
                // A wrapper this wave has not yet routed through a `&dyn` seam is
                // still a wrapper: the naming convention is load-bearing here, and
                // keying only on the derived minted set would let a claim about a
                // wrapper minted inside an enum arm escape.
                let is_wrapper_name =
                    object.len() > "Verified".len() && object.starts_with("Verified");
                if is_anaphor || is_wrapper_name || minted.contains(object) {
                    let start = index.saturating_sub(3);
                    return Some(format!(
                        "`{}` .. `{}` .. `{}`  in: \"{}\"",
                        tokens[index],
                        tokens[verb_index],
                        object,
                        tokens[start..object_limit].join(" ")
                    ));
                }
            }
        }
    }
    None
}

fn discharges(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    lowered.contains("deployment obligation") || lowered.contains("movementactionresultauthority")
}

/// A function signature, taken from the declaration line to the first `;` or
/// `{`. Used for both the minted-set derivation and the scope test.
fn signature_at(lines: &[&str], start: usize) -> String {
    let mut out = String::new();
    for line in lines.iter().skip(start).take(40) {
        out.push_str(line);
        out.push(' ');
        if line.contains(';') || line.contains('{') {
            break;
        }
    }
    out
}

fn takes_dyn_verifier(signature: &str) -> bool {
    signature.contains("&dyn") && signature.contains("Verifier")
}

fn returned_type(signature: &str) -> Option<String> {
    let after_arrow = signature.split("->").nth(1)?;
    let inner = match after_arrow.find("Result<") {
        Some(position) => &after_arrow[position + "Result<".len()..],
        None => after_arrow,
    };
    let name: String = inner
        .trim()
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
        .collect();
    if name.is_empty() { None } else { Some(name) }
}

struct Wave {
    /// Every type minted by a function that takes a `&dyn ...Verifier`.
    minted: BTreeSet<String>,
    /// Files declaring at least one such function.
    minting_files: BTreeSet<PathBuf>,
}

fn scan_wave(files: &[PathBuf]) -> Wave {
    let mut minted = BTreeSet::new();
    let mut minting_files = BTreeSet::new();
    for path in files {
        let text = fs::read_to_string(path).expect("read source");
        let lines: Vec<&str> = text.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            if !line.trim_start().starts_with("fn ") && !line.trim_start().starts_with("pub fn ") {
                continue;
            }
            let signature = signature_at(&lines, index);
            if !takes_dyn_verifier(&signature) {
                continue;
            }
            minting_files.insert(path.clone());
            if let Some(name) = returned_type(&signature) {
                minted.insert(name);
            }
        }
    }
    Wave {
        minted,
        minting_files,
    }
}

/// The item a doc block is attached to: the first line after it that is neither
/// blank nor an attribute.
fn attached_item(lines: &[&str], after: usize) -> Option<(usize, String)> {
    let mut index = after;
    while index < lines.len() {
        let trimmed = lines[index].trim();
        if trimmed.is_empty() || trimmed.starts_with("#[") {
            index += 1;
            continue;
        }
        return Some((index, trimmed.to_owned()));
    }
    None
}

/// The body of a `pub trait`, to the closing brace in column zero.
fn trait_body(lines: &[&str], start: usize) -> String {
    let mut out = String::new();
    for (offset, line) in lines.iter().enumerate().skip(start) {
        out.push_str(line);
        out.push('\n');
        if offset > start && *line == "}" {
            break;
        }
    }
    out
}

fn body_mints(body: &str, minted: &BTreeSet<String>) -> bool {
    body.split("->").skip(1).any(|fragment| {
        let candidate = format!("-> {fragment}");
        returned_type(&candidate).is_some_and(|name| minted.contains(&name))
            || fragment
                .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
                .any(|word| minted.contains(word))
    })
}

/// Conjunct 2. `None` means the block governs nothing a `&dyn` verifier mints,
/// so an exclusivity claim in it may well be true and must not be caveated.
fn scope_of(
    path: &Path,
    marker: &str,
    item: &str,
    item_index: usize,
    lines: &[&str],
    wave: &Wave,
) -> Option<String> {
    if marker == "//!" {
        return wave
            .minting_files
            .contains(path)
            .then(|| "module declares a function taking `&dyn ...Verifier`".to_owned());
    }
    for keyword in ["pub struct ", "pub enum ", "struct ", "enum "] {
        if let Some(rest) = item.strip_prefix(keyword) {
            let name: String = rest
                .chars()
                .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
                .collect();
            if wave.minted.contains(&name) {
                return Some(format!("declaration of the minted type `{name}`"));
            }
            return None;
        }
    }
    if item.starts_with("fn ") || item.starts_with("pub fn ") {
        let signature = signature_at(lines, item_index);
        if takes_dyn_verifier(&signature) {
            return Some("function taking `&dyn ...Verifier`".to_owned());
        }
        if returned_type(&signature).is_some_and(|name| wave.minted.contains(&name)) {
            return Some("function returning a minted wrapper".to_owned());
        }
        return None;
    }
    if item.starts_with("pub trait ") || item.starts_with("trait ") {
        let body = trait_body(lines, item_index);
        if takes_dyn_verifier(&body) {
            return Some("trait declaring a method taking `&dyn ...Verifier`".to_owned());
        }
        if body_mints(&body, &wave.minted) {
            return Some("trait declaring a method returning a minted wrapper".to_owned());
        }
        return None;
    }
    None
}

struct Finding {
    file: String,
    line: usize,
    scope: String,
    item: String,
    claim: String,
}

fn sweep() -> (Vec<Finding>, usize, usize) {
    let files = wave_source_files();
    let wave = scan_wave(&files);
    assert!(
        !wave.minted.is_empty() && !wave.minting_files.is_empty(),
        "the minted set is empty: the signature conjunct would vacuously exonerate every block"
    );
    let root = repo_root();
    let mut findings = Vec::new();
    let mut blocks_read = 0usize;
    let mut in_scope = 0usize;

    for path in &files {
        let text = fs::read_to_string(path).expect("read source");
        let lines: Vec<&str> = text.lines().collect();
        let mut index = 0usize;
        while index < lines.len() {
            let trimmed = lines[index].trim_start();
            let marker = if trimmed.starts_with("//!") {
                "//!"
            } else if trimmed.starts_with("///") {
                "///"
            } else {
                index += 1;
                continue;
            };
            let start = index;
            let mut body = String::new();
            while index < lines.len() {
                let line = lines[index].trim_start();
                let Some(rest) = line.strip_prefix(marker) else {
                    break;
                };
                if marker == "///" && line.starts_with("////") {
                    break;
                }
                body.push_str(rest.trim_start());
                body.push('\n');
                index += 1;
            }
            blocks_read += 1;
            let Some((item_index, item)) = attached_item(&lines, index) else {
                continue;
            };
            let Some(scope) = scope_of(path, marker, &item, item_index, &lines, &wave) else {
                continue;
            };
            in_scope += 1;
            let Some(claim) = production_claim(&body, &wave.minted) else {
                continue;
            };
            if discharges(&body) {
                continue;
            }
            findings.push(Finding {
                file: path
                    .strip_prefix(&root)
                    .unwrap_or(path)
                    .display()
                    .to_string(),
                line: start + 1,
                scope,
                item: item.chars().take(72).collect(),
                claim,
            });
        }
    }
    findings.sort_by(|left, right| (&left.file, left.line).cmp(&(&right.file, right.line)));
    (findings, blocks_read, in_scope)
}

#[test]
fn the_instrument_discriminates_before_it_certifies() {
    let files = wave_source_files();
    let wave = scan_wave(&files);

    // The grammatical conjunct fires on the shape and not on a wording.
    let minted: BTreeSet<String> = ["VerifiedThing".to_owned()].into_iter().collect();
    assert!(
        production_claim(
            "no trait implementation outside this crate can produce one",
            &minted
        )
        .is_some(),
        "the key must see a negation governing a production verb with an anaphoric object"
    );
    assert!(
        production_claim("nothing here can fabricate a VerifiedThing", &minted).is_some(),
        "the key must see a minted type as the object"
    );
    // Prose that merely contains both a negation and a production verb is not a
    // claim about who may produce the value, and must not be condemned.
    assert!(
        production_claim(
            "the actor attestation is sound but its identity is not issued by the trust anchor",
            &minted
        )
        .is_none(),
        "a refusal condition is not an exclusivity claim about production"
    );
    assert!(
        production_claim("a bare record with no signature attached", &minted).is_none(),
        "a production verb with no production object is not a claim"
    );
    // The discharge is recognised, and its absence is not.
    assert!(discharges(
        "that is a deployment obligation, not a type-level refusal"
    ));
    assert!(discharges("see [`crate::MovementActionResultAuthority`]"));
    assert!(!discharges("that is refused by the type system"));

    // The signature conjunct is derived from the tree, not asserted.
    assert!(
        wave.minted.contains("VerifiedCellPromotionEvidence"),
        "the minted set must be derived from real `&dyn ...Verifier` signatures"
    );
}

#[test]
fn no_exclusivity_claim_outruns_its_signature() {
    let (findings, blocks_read, in_scope) = sweep();
    assert!(
        blocks_read > 0 && in_scope > 0,
        "the sweep read {blocks_read} doc blocks and found {in_scope} in scope; \
         a zero here is an instrument failure, not a clean tree"
    );
    let report: String = findings
        .iter()
        .map(|finding| {
            format!(
                "\n  {}:{}\n      scope: {}\n      item:  {}\n      claim: {}\n",
                finding.file, finding.line, finding.scope, finding.item, finding.claim
            )
        })
        .collect();
    assert!(
        findings.is_empty(),
        "LAW B. Each doc block below claims, in a scope whose verifier arrives as \
         `&dyn`, that something cannot produce a verified value. An out-of-crate type \
         can implement that verifier and mint by passing itself, so the claim is a \
         DEPLOYMENT OBLIGATION and never a type-level refusal. Either state the \
         obligation in the block, or narrow the claim to direct construction -- do NOT \
         caveat a refusal that is genuinely type-level.\
         \n(read {blocks_read} doc blocks, {in_scope} in scope){report}"
    );
}
