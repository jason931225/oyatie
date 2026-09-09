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
//!    by a function HANDED a `dyn ...Verifier`. A block is in scope only when it
//!    is ATTACHED to such a function, to a minted type's declaration, to a trait
//!    declaring a method HANDED one, to a trait declaring a method RETURNING a
//!    minted wrapper, to a trait whose own doc LINKS such a function, to a
//!    function returning a minted wrapper, or is the `//!` doc of a file that
//!    declares one. Those are the arms `scope_of` implements and no others; the
//!    trait arms differ in what they key on, and an earlier wording spelled them
//!    the same, which is how a list can be complete and still unreadable. Scope
//!    is never inferred from a file's name or from a mention in prose.
//!
//!    WHAT "HANDED" MEANS, AND WHY IT IS NOT `&dyn` ANY MORE. The first draft
//!    keyed on the literal token `&dyn` anywhere in the signature text. Three
//!    things were wrong with that and each is now closed, each with the finding
//!    that closed it:
//!      * RETURN POSITION COUNTED. `proof_verifier(&self) -> &dyn
//!        CellProofVerifier` made an `&self` accessor a minting function and its
//!        file a minting file. The test now reads the PARAMETER LIST only. That
//!        NARROWS scope, so `promotion_economics_source.rs` staying a minting
//!        file is asserted below rather than assumed.
//!      * OWNERSHIP COUNTED AS DIFFERENCE. `Box`, `Arc` and `Rc<dyn ...Verifier>`
//!        were invisible. An out-of-crate type can pass itself boxed as easily as
//!        by reference, so the predicate now keys on `dyn ` plus `Verifier` in
//!        the parameters under any pointer.
//!      * A VERIFIER REACHED THROUGH A STRUCT WAS INVISIBLE, and that is the one
//!        that mattered. `advance_cell_promotion_economics` — the wave's flagship
//!        minter — takes `&PromotionEconomicsReplayPortsV1`, whose fourth PUBLIC
//!        FIELD is `proof_verifier: &'a dyn CellProofVerifier`. Its own signature
//!        contains neither token, and the file its wrapper is declared in has
//!        zero `verify_`, so every route the old key had was closed against it
//!        while an out-of-crate rogue verifier composed at exit 0. One level of
//!        struct indirection is now a route, declared as the bound.
//!
//!    A fourth, on the other side of the same seam: a wrapper minted INSIDE AN
//!    ENUM ARM (`...StepOutcomeV1::Complete(Box<VerifiedCellPromotionEconomics>)`)
//!    left the returned-type test seeing only the enum, so the wrapper's own
//!    declaration was never in scope. The minted set now also takes what a
//!    returned enum's variants carry.
//!
//!    The trait-link clause is the one the first draft lacked, and a
//!    perturbation control is what found it: deleting the caveat from
//!    `PromotionEconomicsCheckpointStore` -- a port that mints nothing itself
//!    and states clause (a) anyway -- left the sweep silent. A trait is the only
//!    item kind that HAS implementers, and clause (a) is a claim about what an
//!    implementer can do, so a port whose doc names the verifier its caller goes
//!    through is squarely in scope. A type alias linking the same verifier is
//!    not: it has no implementer for the claim to be about.
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
//!     -- "no adapter can hold that role" names a role, not a value. That is
//!     right for THIS law, whose subject is a value a foreign verifier vouches
//!     into existence. It is wrong where the role IS a declared type, and the
//!     one live instance of that -- "a read-authorized invocation cannot reach
//!     a write path by type" -- is LAW D at the foot of this file rather than a
//!     widening here;
//! (b2) the SCOPE conjunct is one hop: a wrapper reachable only by calling an
//!     inherent method on a minted wrapper is not in the minted set. Closing
//!     that with a transitive closure over inherent returns was tried and
//!     REJECTED: it condemned two true statements
//!     (`PlacementPersistenceAuthorityV1::read_authority` and its binding twin
//!     saying "one invocation mints exactly one authority", which is a fact
//!     about `Clone` and `self`-consuming constructors) and found nothing that
//!     was false. The one undischarged claim in the transitive population is
//!     the LAW D site, and its object conjunct blocks it here regardless of
//!     scope, so the widening would have cost two inverted defects for nothing;
//! (c) a claim in a doc block attached to a type alias, a constant, or a field,
//!     since none of those is a minter;
//! (c2) a verifier reached through TWO levels of struct — a field of a field.
//!     One level is the declared bound; two would need a closure walk, and the
//!     tree has no instance of it;
//! (e) THE OBJECT KEY WIDENS WHAT IS CONDEMNED, not only what is seen, and no
//!     document said so. `production_claim` accepts an object when it is the
//!     anaphor `one`/`any`, OR is in the derived minted set, OR merely STARTS
//!     WITH `Verified`. That last is a name prefix, in a wave whose doctrine is
//!     that name prefixes are blind where it matters — it is kept because it
//!     catches a wrapper the structural enum-arm pass still cannot reach, and it
//!     is declared here because it can condemn a TRUE claim about a
//!     `Verified*`-named wrapper appearing in a mixed file's `//!` doc. The
//!     anaphor arm can do the same, so removing the prefix would not close the
//!     hazard; both are stated rather than either being hidden.
//! (d) a claim that is TRUE -- some `verify_*` functions in this crate take no
//!     verifier at all, and an exclusivity claim about the wrappers they mint
//!     is sound. Conjunct 2 exists to keep those out: caveating a genuinely
//!     type-level refusal would be this same defect inverted. The set is the
//!     one `takes_dyn_verifier` returns false for, computed below; this doc
//!     states no tally, because a tally here would be the very species the
//!     file exists to refuse, written into the file that refuses it.
//!
//! It asserts that the offending set is EMPTY and prints its members. It
//! asserts no count and no non-zero quantity: a live finding count goes red
//! exactly when the burn-down succeeds.

use std::collections::{BTreeMap, BTreeSet};
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
    // Obtainment is production for this law's purposes: "cannot reach a
    // verified value" and "cannot mint one" make the same claim about the same
    // composition. Added with LAW D below, which needs the verb; on its own it
    // changed no verdict here, because the object conjunct still decides.
    "reach",
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

/// The parameter list of a signature: the first `(` to its matching `)`.
///
/// Everything the verifier conjunct asks is a question about what a function is
/// HANDED. Reading the whole signature answered a different one: a `&dyn
/// ...Verifier` in RETURN position made an accessor a minter --
/// `promotion_economics_source.rs`'s `proof_verifier(&self) -> &dyn
/// CellProofVerifier` was classified as one -- and that widening was accidental
/// rather than designed, so a refactor removing the accessor would have silently
/// narrowed the law.
fn parameter_list(signature: &str) -> &str {
    let Some(open) = signature.find('(') else {
        return "";
    };
    let mut depth = 0usize;
    for (offset, character) in signature[open..].char_indices() {
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return &signature[open + 1..open + offset];
                }
            }
            _ => {}
        }
    }
    &signature[open + 1..]
}

/// A verifier handed in directly, under any pointer: `&dyn`, `&'a dyn`, and
/// `Box`/`Arc`/`Rc<dyn ...Verifier>` alike.
///
/// The first draft keyed on the literal `&dyn`, which is one of two ways this
/// tree already hands a verifier over. Ownership does not change the argument:
/// an out-of-crate type can implement the verifier and pass itself boxed just as
/// easily as by reference.
fn hands_over_a_verifier(text: &str) -> bool {
    text.contains("dyn ") && text.contains("Verifier")
}

/// A verifier reached through a STRUCT the function is handed.
///
/// This is the route that hid the wave's flagship minter.
/// `advance_cell_promotion_economics` takes
/// `&PromotionEconomicsReplayPortsV1`, a public struct whose fourth public field
/// is `proof_verifier: &'a dyn CellProofVerifier`; its own signature contains
/// neither `dyn` nor `Verifier`, so a lexical test on the signature alone could
/// never see it, and the file it mints into declares no `verify_` at all. One
/// level of indirection, declared as the bound: a struct field, not a struct
/// field of a struct field.
fn carries_a_verifier(parameters: &str, carriers: &BTreeSet<String>) -> bool {
    parameters
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .any(|word| carriers.contains(word))
}

fn takes_dyn_verifier(signature: &str, carriers: &BTreeSet<String>) -> bool {
    let parameters = parameter_list(signature);
    hands_over_a_verifier(parameters) || carries_a_verifier(parameters, carriers)
}

/// For every enum in the wave, the types its variants carry.
///
/// A minter that hands its wrapper back inside an enum arm --
/// `PromotionEconomicsVerificationStepOutcomeV1::Complete(Box<VerifiedCellPromotionEconomics>)`
/// is the wave's flagship -- returns the ENUM, so a returned-type test alone
/// never sees the wrapper and the wrapper's own declaration is never in scope.
/// This is what the `Verified` name prefix in `production_claim` was standing in
/// for; the prefix stays, because it also catches a wrapper this pass cannot
/// reach, but the structural route is what a wrapper named otherwise depends on.
fn enum_payloads(files: &[PathBuf]) -> BTreeMap<String, BTreeSet<String>> {
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for path in files {
        let text = fs::read_to_string(path).expect("read source");
        let lines: Vec<&str> = text.lines().collect();
        let mut index = 0usize;
        while index < lines.len() {
            let declared = lines[index]
                .strip_prefix("pub enum ")
                .or_else(|| lines[index].strip_prefix("enum "));
            let Some(rest) = declared else {
                index += 1;
                continue;
            };
            let name: String = rest
                .chars()
                .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
                .collect();
            let mut carried = BTreeSet::new();
            let mut cursor = index + 1;
            while cursor < lines.len() && lines[cursor] != "}" {
                for word in lines[cursor]
                    .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
                {
                    if word.starts_with(|character: char| character.is_ascii_uppercase()) {
                        carried.insert(word.to_owned());
                    }
                }
                cursor += 1;
            }
            out.entry(name).or_default().extend(carried);
            index = cursor + 1;
        }
    }
    out
}

/// Every struct in the wave with a member that is a `dyn ...Verifier`.
fn verifier_carriers(files: &[PathBuf]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for path in files {
        let text = fs::read_to_string(path).expect("read source");
        let lines: Vec<&str> = text.lines().collect();
        let mut index = 0usize;
        while index < lines.len() {
            let line = lines[index];
            let declared = line
                .strip_prefix("pub struct ")
                .or_else(|| line.strip_prefix("struct "));
            let Some(rest) = declared else {
                index += 1;
                continue;
            };
            let name: String = rest
                .chars()
                .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
                .collect();
            let mut cursor = index + 1;
            while cursor < lines.len() && lines[cursor] != "}" {
                if hands_over_a_verifier(lines[cursor]) {
                    out.insert(name.clone());
                }
                cursor += 1;
            }
            index = cursor + 1;
        }
    }
    out
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
    /// Structs with a `dyn ...Verifier` member, computed first because the
    /// signature conjunct needs them.
    carriers: BTreeSet<String>,
    /// Every type minted by a function that takes a `dyn ...Verifier`.
    minted: BTreeSet<String>,
    /// The names of those functions.
    minting_fns: BTreeSet<String>,
    /// Files declaring at least one such function.
    minting_files: BTreeSet<PathBuf>,
}

fn scan_wave(files: &[PathBuf]) -> Wave {
    let carriers = verifier_carriers(files);
    let payloads = enum_payloads(files);
    let mut minted = BTreeSet::new();
    let mut minting_fns = BTreeSet::new();
    let mut minting_files = BTreeSet::new();
    for path in files {
        let text = fs::read_to_string(path).expect("read source");
        let lines: Vec<&str> = text.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            if !line.trim_start().starts_with("fn ") && !line.trim_start().starts_with("pub fn ") {
                continue;
            }
            let signature = signature_at(&lines, index);
            if !takes_dyn_verifier(&signature, &carriers) {
                continue;
            }
            minting_files.insert(path.clone());
            let declared: String = line
                .trim_start()
                .trim_start_matches("pub ")
                .trim_start_matches("fn ")
                .chars()
                .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
                .collect();
            if !declared.is_empty() {
                minting_fns.insert(declared);
            }
            if let Some(name) = returned_type(&signature) {
                if let Some(carried) = payloads.get(&name) {
                    minted.extend(carried.iter().cloned());
                }
                minted.insert(name);
            }
        }
    }
    Wave {
        carriers,
        minted,
        minting_fns,
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
    text: &str,
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
        if takes_dyn_verifier(&signature, &wave.carriers) {
            return Some(
                "function handed a `dyn ...Verifier`, directly or through a struct".to_owned(),
            );
        }
        if returned_type(&signature).is_some_and(|name| wave.minted.contains(&name)) {
            return Some("function returning a minted wrapper".to_owned());
        }
        return None;
    }
    if item.starts_with("pub trait ") || item.starts_with("trait ") {
        let body = trait_body(lines, item_index);
        if body
            .lines()
            .any(|line| takes_dyn_verifier(line, &wave.carriers))
            || hands_over_a_verifier(&body)
        {
            return Some("trait declaring a method handed a `dyn ...Verifier`".to_owned());
        }
        if body_mints(&body, &wave.minted) {
            return Some("trait declaring a method returning a minted wrapper".to_owned());
        }
        // A trait is the only item kind that HAS implementers, and clause (a) is a
        // claim about what an implementer can do. So a port that mints nothing
        // itself is still in scope when its own doc links the minter its caller
        // goes through: an out-of-crate type may implement both this trait and
        // that verifier. This is the shape at
        // `PromotionEconomicsCheckpointStore`, which returns only public records
        // and still states the clause.
        if doc_links(text)
            .iter()
            .any(|target| wave.minting_fns.contains(target) || wave.minted.contains(target))
        {
            return Some("trait whose doc links a `&dyn ...Verifier` minter".to_owned());
        }
        return None;
    }
    None
}

/// Intra-doc link targets in a block, reduced to the final path segment.
fn doc_links(doc: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut rest = doc;
    while let Some(position) = rest.find("[`") {
        rest = &rest[position + 2..];
        let Some(end) = rest.find("`]") else { break };
        let target = &rest[..end];
        rest = &rest[end + 2..];
        if let Some(head) = target.rsplit("::").find(|part| *part != "crate") {
            let name: String = head
                .chars()
                .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
                .collect();
            if !name.is_empty() {
                out.insert(name);
            }
        }
    }
    out
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
            let Some(scope) = scope_of(path, marker, &item, item_index, &lines, &body, &wave)
            else {
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
        "the minted set must be derived from real `dyn ...Verifier` signatures"
    );

    // The predicate reads PARAMETERS, not the whole signature: an accessor
    // handing a verifier BACK is not a minter.
    assert_eq!(
        parameter_list("pub fn proof_verifier(&self) -> &dyn CellProofVerifier {"),
        "&self",
        "the parameter list must stop at the closing parenthesis"
    );
    assert!(
        !takes_dyn_verifier(
            "pub fn proof_verifier(&self) -> &dyn CellProofVerifier {",
            &wave.carriers
        ),
        "a `&dyn ...Verifier` in RETURN position must not make an accessor a minter"
    );
    // Narrowing must not have cost the file that accessor lives in. Its `admit`
    // is handed the closure issuer, whose `proof_verifier` is a
    // `Box<dyn CellProofVerifier>`, so the file stays in scope through the
    // struct route rather than through the accessor.
    assert!(
        wave.minting_files
            .iter()
            .any(|path| path.ends_with("promotion_economics_source.rs")),
        "narrowing to the parameter list must not drop a file that really is handed a verifier"
    );

    // Ownership is not a difference.
    assert!(hands_over_a_verifier("issuer: Box<dyn CellProofVerifier>"));
    assert!(hands_over_a_verifier(
        "issuer: Arc<dyn BindingProofVerifier>"
    ));
    assert!(!hands_over_a_verifier("record: &CellPromotionEconomicsV1"));

    // A verifier reached through one struct is a route, and the struct that
    // carries it is found in the tree rather than named here.
    assert!(
        wave.carriers.contains("PromotionEconomicsReplayPortsV1"),
        "the ports struct holding a public `dyn CellProofVerifier` field must be \
         found as a carrier"
    );
    assert!(
        takes_dyn_verifier(
            "pub fn advance(_ports: &'a PromotionEconomicsReplayPortsV1<'a>, _now: u64,) -> X {",
            &wave.carriers
        ),
        "a verifier arriving as a field of a struct parameter must put the function in scope"
    );
    assert!(
        !takes_dyn_verifier(
            "pub fn plain(_record: &CellPromotionEconomicsV1) -> X {",
            &wave.carriers
        ),
        "a parameter carrying no verifier must not put a function in scope"
    );

    // A wrapper minted inside an enum arm reaches the minted set structurally,
    // so its own declaration is in scope without the name prefix doing the work.
    assert!(
        wave.minted.contains("VerifiedCellPromotionEconomics"),
        "a wrapper handed back inside an enum arm must be in the derived minted set"
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

// ============================================================================
// LAW D, executable: an INVOCATION is not a separation.
//
// THE LAW. A doc block may not claim that a read-authorized INVOCATION cannot
// reach a write path by type, in a wave where one verified invocation type
// mints BOTH authorities through unconditional conversions. The separation the
// two authority newtypes make is real and is between the AUTHORITY VALUES: a
// `*ReadAuthorityV1` has no method producing its persistence twin, so a holder
// of one cannot become a writer. It is not between INVOCATIONS: an invocation
// is one type whatever `action` its signed payload names, and
// `into_persistence_authority` and `into_read_authority` are both available on
// it, gated on nothing.
//
// WHY THIS FILE AND NOT A NEW ONE. Same population as LAW B -- every doc block
// in both crates -- read with a different question, and it reuses this file's
// `words`, `is_negation` and window constants. A second file would fork the
// population, and this file's own doc already says why that is how a wire gets
// lost.
//
// WHY LAW B CANNOT SEE IT, on all three of its conjuncts at once. The verb:
// `PRODUCTION_STEMS` had no "reach" until this law needed one. The object:
// "write path" is a role, which is LAW B's declared blind spot (b) -- and the
// blind spot is right for LAW B, because a role is not a value a verifier
// vouches into existence. The scope: `ServingAuthorityPersistenceAuthorityV1`
// is minted by `into_persistence_authority`, which is handed no verifier, so it
// is not in LAW B's one-hop minted set at all. Three misses, one site.
//
// THE STRUCTURAL HALF IS DERIVED, NOT ASSERTED. `invocations_that_widen` reads
// every `impl Verified*Invocation` block in the wave and keeps the types whose
// own `pub fn`s hand back BOTH a `*PersistenceAuthorityV1` and a
// `*ReadAuthorityV1`. If a future author gates one conversion -- a type
// parameter, an action proof, a second verification -- that invocation leaves
// the set and a claim about it stops being condemned, which is the correct
// answer. The law has a subject only while the tree makes the claim false.
//
// THE FIX IS A REWORDING, NOT A CAVEAT. Stamping a deployment obligation on
// this sentence would be wrong twice over: the separation between the two
// AUTHORITY types is genuinely type-level, and LAW B's own doc says caveating a
// genuinely type-level refusal is that law's defect inverted. Say "a read
// AUTHORITY cannot reach a write path by type" -- true, and checked by the
// negative control below -- or gate the conversion.
//
// It asserts that the offending set is EMPTY and prints its members. It asserts
// no count and no non-zero quantity.

/// Verbs of obtainment. A claim that something cannot REACH a value is the same
/// claim as that it cannot mint one.
const REACHING_STEMS: &[&str] = &[
    "reach", "obtain", "become", "widen", "escalat", "acquir", "convert", "mint", "produce",
];

/// Objects that name a WRITE ROLE which this wave declares as a type.
///
/// LAW B's blind spot (b) -- "names a role, not a value" -- is right for LAW B
/// and wrong here. This wave gives every write role a newtype, one
/// `*PersistenceAuthorityV1` per signed invocation family, so "a write path" is
/// a claim about who can hold a declared value.
const WRITE_ROLE_OBJECTS: &[&[&str]] = &[
    &["write", "path"],
    &["write", "authority"],
    &["persistence", "authority"],
    &["write", "side"],
];

/// How far back from the negation the claim's SUBJECT is looked for.
const NEGATION_TO_SUBJECT_WINDOW: usize = 5;

fn is_reaching(word: &str) -> bool {
    let lowered = word.to_ascii_lowercase();
    REACHING_STEMS.iter().any(|stem| {
        INFLECTIONS.iter().any(|inflection| {
            lowered.len() == stem.len() + inflection.len()
                && lowered.starts_with(stem)
                && lowered.ends_with(inflection)
        })
    })
}

/// The grammatical conjunct: a subject that is an INVOCATION, a negation, a
/// verb of obtainment within the window, and a write-role object after it.
fn invocation_widening_claim(text: &str) -> Option<String> {
    let tokens = words(text);
    for (index, token) in tokens.iter().enumerate() {
        if !is_negation(token) {
            continue;
        }
        let subject_start = index.saturating_sub(NEGATION_TO_SUBJECT_WINDOW);
        if !tokens[subject_start..index]
            .iter()
            .any(|word| word.eq_ignore_ascii_case("invocation"))
        {
            continue;
        }
        let verb_limit = (index + 1 + NEGATION_TO_VERB_WINDOW).min(tokens.len());
        for verb_index in (index + 1)..verb_limit {
            if !is_reaching(&tokens[verb_index]) {
                continue;
            }
            let object_limit = (verb_index + 1 + VERB_TO_OBJECT_WINDOW).min(tokens.len());
            for object_index in (verb_index + 1)..object_limit {
                let Some(role) = WRITE_ROLE_OBJECTS.iter().find(|role| {
                    tokens.len() >= object_index + role.len()
                        && role
                            .iter()
                            .zip(&tokens[object_index..])
                            .all(|(expected, actual)| actual.eq_ignore_ascii_case(expected))
                }) else {
                    continue;
                };
                return Some(format!(
                    "`{}` .. `{}` .. `{}`  in: \"{}\"",
                    tokens[index],
                    tokens[verb_index],
                    role.join(" "),
                    tokens[subject_start..object_limit].join(" ")
                ));
            }
        }
    }
    None
}

/// Every `Verified*Invocation` in the wave whose own `pub fn`s hand back BOTH a
/// persistence authority and a read authority, with the methods that do it.
fn invocations_that_widen(files: &[PathBuf]) -> BTreeMap<String, Vec<String>> {
    let mut out = BTreeMap::new();
    for path in files {
        let text = fs::read_to_string(path).expect("read source");
        let lines: Vec<&str> = text.lines().collect();
        let mut index = 0usize;
        while index < lines.len() {
            let Some(rest) = lines[index].strip_prefix("impl ") else {
                index += 1;
                continue;
            };
            let subject: String = rest
                .chars()
                .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
                .collect();
            if !(subject.starts_with("Verified") && subject.ends_with("Invocation")) {
                index += 1;
                continue;
            }
            let mut persistence = Vec::new();
            let mut read = Vec::new();
            let mut cursor = index + 1;
            while cursor < lines.len() && lines[cursor] != "}" {
                if lines[cursor].starts_with("    pub fn ") {
                    let name: String = lines[cursor]["    pub fn ".len()..]
                        .chars()
                        .take_while(|character| {
                            character.is_ascii_alphanumeric() || *character == '_'
                        })
                        .collect();
                    let signature = signature_at(&lines, cursor);
                    if let Some(handed_back) = returned_type(&signature) {
                        if handed_back.ends_with("PersistenceAuthorityV1") {
                            persistence.push(name);
                        } else if handed_back.ends_with("ReadAuthorityV1") {
                            read.push(name);
                        }
                    }
                }
                cursor += 1;
            }
            if !persistence.is_empty() && !read.is_empty() {
                let mut both = persistence;
                both.extend(read);
                out.insert(subject, both);
            }
            index = cursor + 1;
        }
    }
    out
}

fn law_d_sweep() -> (Vec<Finding>, BTreeMap<String, Vec<String>>) {
    let files = wave_source_files();
    let widening = invocations_that_widen(&files);
    let root = repo_root();
    let mut findings = Vec::new();
    if widening.is_empty() {
        return (findings, widening);
    }
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
            let Some(claim) = invocation_widening_claim(&body) else {
                continue;
            };
            let item = attached_item(&lines, index)
                .map(|(_, item)| item)
                .unwrap_or_default();
            findings.push(Finding {
                file: path
                    .strip_prefix(&root)
                    .unwrap_or(path)
                    .display()
                    .to_string(),
                line: start + 1,
                scope: widening.keys().cloned().collect::<Vec<_>>().join(", "),
                item: item.chars().take(72).collect(),
                claim,
            });
        }
    }
    findings.sort_by(|left, right| (&left.file, left.line).cmp(&(&right.file, right.line)));
    (findings, widening)
}

#[test]
fn law_d_discriminates_before_it_certifies() {
    let widening = invocations_that_widen(&wave_source_files());
    assert!(
        widening.len() >= 5,
        "only {} invocation types were found minting both authorities -- the law would \
         have almost no subject, and a subject-less law exonerates by silence",
        widening.len()
    );
    assert!(
        widening.contains_key("VerifiedServingAuthorityInvocation"),
        "the invocation the false claim is written about must be derived from the tree"
    );

    // The claim the tree carried, and the rewording that makes it true. These
    // two sentences differ in ONE WORD, and the law must split them.
    assert!(
        invocation_widening_claim(
            "Distinct from ServingAuthorityReadAuthorityV1 so that a read-authorized \
             invocation cannot reach a write path by type"
        )
        .is_some(),
        "positive control: the claim about an INVOCATION is what the types do not make"
    );
    assert!(
        invocation_widening_claim(
            "the converse never holds, and that is the whole point of the two types -- a \
             read authority cannot reach a write path by type"
        )
        .is_none(),
        "negative control: the claim about an AUTHORITY VALUE is true, and condemning it \
         would be LAW B's defect inverted"
    );
    // The verb and the object are both load-bearing.
    assert!(
        invocation_widening_claim("an invocation cannot be used for a write path").is_none(),
        "a negation with no verb of obtainment is not this claim"
    );
    assert!(
        invocation_widening_claim("an invocation cannot reach a caller-facing facade").is_none(),
        "a verb of obtainment with no write-role object is not this claim"
    );
    assert!(
        invocation_widening_claim("no adapter can obtain a write authority").is_none(),
        "a claim whose subject is not an invocation belongs to a different law"
    );
    assert!(is_reaching("reaches") && is_reaching("obtained") && !is_reaching("read"));
}

#[test]
fn no_invocation_is_described_as_a_write_barrier() {
    let (findings, widening) = law_d_sweep();
    let derived: String = widening
        .iter()
        .map(|(invocation, methods)| format!("\n  {invocation}: {}", methods.join(", ")))
        .collect();
    let report: String = findings
        .iter()
        .map(|finding| {
            format!(
                "\n  {}:{}\n      item:  {}\n      claim: {}\n",
                finding.file, finding.line, finding.item, finding.claim
            )
        })
        .collect();
    assert!(
        findings.is_empty(),
        "LAW D. Each doc block below says a read-authorized INVOCATION cannot reach a \
         write path BY TYPE. The invocation types below each mint BOTH authorities \
         through conversions gated on nothing, so an invocation whose payload names a \
         read action converts to write authority at exit 0 and the sentence is false of \
         the type system. What IS type-level is the separation between the two AUTHORITY \
         VALUES: say \"a read AUTHORITY cannot reach a write path by type\", or gate the \
         conversion on the action. Do NOT caveat it as a deployment obligation -- the \
         separation between the authority types is real, and caveating a genuine \
         type-level refusal is LAW B's defect inverted.\
         \n(invocations minting both authorities:{derived}){report}"
    );
}
