//! LAW A, executable: reachable is not present.
//!
//! THE LAW. A compare-and-set precondition required BY VALUE on a write set,
//! for a row that no surface in these crates can have created before that write
//! runs, has no legal value at its first execution. The caller must then either
//! invent one or let a missing row launder into a clean first write, which is
//! the shape where two workers each believe they are opening the ledger and
//! each overwrites the other's opening.
//!
//! WHY THIS FILE EXISTS. Seven-plus sites of this shape were found by hand
//! across four review seats, and in round 9 two seats independently found
//! different ones with no contact between them. A script plus a human
//! adjudication of its candidate list has not converged in nine rounds. This
//! test decides the population; nobody adjudicates it.
//!
//! WHAT IT KEYS ON. The subject is the ROW, never the field name. For every
//! record type `R` declared in these crates that some write set carries as a
//! bare member under a name that is not itself a precondition -- that is, every
//! row these write sets actually write -- the test asks whether `R` has a BIRTH
//! PATH:
//!
//!   (i)   some carrier's `R`-precondition can represent absence; or
//!   (ii)  some carrier proposes `R` with no `R`-precondition at all, so that
//!         write can open the row unconditionally; or
//!   (iii) some method taking a write set that does NOT propose `R` yields `R`,
//!         so an earlier, different write created it; or
//!   (iv)  some READ -- a method taking no write set -- hands `R` back outside
//!         `Option`, which is the contract saying the store materialises the row
//!         rather than waiting for a write to open it.
//!
//! If none holds, every carrier's `R`-precondition is named. This is the
//! criterion the seats actually applied rather than the narrower "its only
//! backing read returns `Option`": under that narrower wording
//! `AppendTransferAuthorizationWriteSetPartsV1::expected_revision` would be
//! condemned, and a seat tested and REJECTED it because `put_manifest` returns
//! the journal. Clause (iii) is that rejection, made mechanical.
//!
//! "CAN REPRESENT ABSENCE" IS A SHAPE, NOT A NAME. `Option<_>`, or an enum with
//! a variant that asserts no version of the row -- no non-optional member whose
//! name or type carries a revision, digest, epoch, generation or ordinal. That
//! admits `Absent`, `Unbound`, `Unmapped` and `Uninstalled { rejection_high_water:
//! Option<_> }` without ever listing those names, and refuses an enum all of
//! whose arms assert a version.
//!
//! EVERY PRECONDITION MUST BE RESOLVABLE TO ITS ROW. A key that maps by name
//! has a blind spot in exactly the places it is most needed: an earlier proto
//! sweep in this wave keyed on Rust members named `next_*` and missed five named
//! exactly `next`, and mapped Rust types to proto messages by name so that a
//! type with no exact-name twin was invisible regardless of spelling. So this
//! test does not skip what it cannot map: `unresolved_preconditions` is asserted
//! empty alongside `violations`. A precondition the instrument cannot attribute
//! to a row is a hole in the instrument, and it is also a real one -- an
//! unattributable absence-capable precondition cannot discharge clause (i) for
//! the row it actually guards, which is how a false positive gets manufactured.
//!
//! Resolution routes, in order, each structural and each recorded in the
//! failure message so a reader can see which one fired:
//!   * the field's type IS a row;
//!   * a scalar newtype whose name is a row's name with a version role appended
//!     (`SourceFenceDirectiveLedgerRevision` -> `SourceFenceDirectiveLedgerV1`);
//!   * a `*Precondition` type whose name stem is a row's name, or is the unique
//!     row-name prefix;
//!   * a `*Precondition` struct whose own members resolve to exactly one row;
//!   * a field name whose stem names exactly one row carried on the same write
//!     set (`expected_outbox_revision` beside `next_outbox`);
//!   * intra-doc links to rows, on the field or on the field's type
//!     declaration. This last route is a contract statement -- "this
//!     compare-and-set is on that row" -- and is the only one an author writes
//!     by hand. EVERY linked row is taken as a subject, so a precondition that
//!     dispatches over several rows resolves to all of them; links to anything
//!     that is not a row these write sets propose are ignored.
//!
//! A second blind spot follows from that last route: absence capability is
//! judged for the whole precondition TYPE, not per arm, so a dispatching
//! precondition with one absent arm discharges clause (i) for every row it
//! names. Splitting that would need per-arm subjects, which no declaration
//! here carries.
//!
//! THE SCOPE, stated because every key has one. This test sees only the face of
//! the law where NO VALUE EXISTS YET. The other face -- a value that exists but
//! is UNREADABLE under the authority the write itself takes -- is LAW C, at the
//! foot of this file, and it is no longer handled by hand: deciding it requires
//! ordering the authority types, and that ordering is now stated in the tree at
//! `BindingPersistenceAuthorityV1::read_authority` and derived from it there.
//! `RecordTransferExecutionOutcomeWriteSetPartsV1::item_precondition` was the
//! instance this paragraph used to name; it is one of the obligations Law C
//! judges.
//!
//! It asserts that the offending sets are EMPTY and prints their members. It
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

#[derive(Clone)]
struct Field {
    name: String,
    ty: String,
    line: usize,
    doc: String,
}

struct Struct {
    file: String,
    line: usize,
    doc: String,
    fields: Vec<Field>,
}

struct Enum {
    doc: String,
    /// One entry per variant: the variant's non-optional members, as
    /// `(field name, field type)`.
    variants: Vec<Vec<(String, String)>>,
    /// One entry per variant, positionally parallel to `variants`: the types of
    /// a TUPLE arm, which carries no member names. Law A does not read this --
    /// widening its absence test would change its verdicts -- and Law C needs
    /// it, because `Request(BindingPersistenceAuthorityV1)` is a tuple arm and
    /// the authority a write set's arm holds is otherwise invisible.
    variant_tuples: Vec<Vec<String>>,
}

#[derive(Default)]
struct Model {
    structs: BTreeMap<String, Struct>,
    enums: BTreeMap<String, Enum>,
    /// `(method name, whole signature)` for every trait method in the wave.
    methods: Vec<(String, String)>,
    /// Where each of those was declared, positionally parallel to `methods`.
    method_sites: Vec<(String, usize)>,
    /// `pub struct Name(Inner);` -- the inner type of every tuple newtype. Law C
    /// derives the authority pairs from this rather than from a list of names.
    newtypes: BTreeMap<String, String>,
    /// For each type with an inherent `impl`, the types its `pub fn`s hand back.
    /// Law C requires the ordering it relies on to be PERFORMABLE, not merely
    /// nameable: a persistence authority subsumes a read authority only if it
    /// declares a method that produces one.
    inherent_returns: BTreeMap<String, BTreeSet<String>>,
}

fn strip_crate(text: &str) -> String {
    text.trim().trim_start_matches("crate::").trim().to_owned()
}

/// The type inside one layer of `Option`, `Vec` or `Box`, else the type itself.
fn unwrap_container(ty: &str) -> String {
    let bare = strip_crate(ty);
    for container in ["Option<", "Vec<", "Box<"] {
        if let Some(rest) = bare.strip_prefix(container) {
            return strip_crate(rest.trim_end_matches('>'));
        }
    }
    bare
}

fn take_doc(pending: &mut Vec<String>) -> String {
    let joined = pending.join("\n");
    pending.clear();
    joined
}

fn parse(files: &[PathBuf], root: &Path) -> Model {
    let mut model = Model::default();
    for path in files {
        let text = fs::read_to_string(path).expect("read source");
        let lines: Vec<&str> = text.lines().collect();
        let display = path
            .strip_prefix(root)
            .unwrap_or(path)
            .display()
            .to_string();
        let mut pending: Vec<String> = Vec::new();
        let mut index = 0usize;
        while index < lines.len() {
            let line = lines[index];
            if let Some(rest) = line
                .strip_prefix("/// ")
                .or_else(|| line.strip_prefix("///"))
            {
                pending.push(rest.to_owned());
                index += 1;
                continue;
            }
            if line.starts_with("#[") {
                index += 1;
                continue;
            }

            if let Some(name) = declared_tuple(line) {
                if let Some(open) = line.find('(') {
                    let inner: String = line[open + 1..]
                        .trim_start_matches("pub ")
                        .chars()
                        .take_while(|character| {
                            character.is_ascii_alphanumeric() || *character == '_'
                        })
                        .collect();
                    if !inner.is_empty() {
                        model.newtypes.insert(name.clone(), inner);
                    }
                }
                // A newtype carries no members, but its doc can declare the row
                // it versions, which is the only route some scalar preconditions
                // have.
                model.structs.entry(name).or_insert(Struct {
                    file: display.clone(),
                    line: index + 1,
                    doc: take_doc(&mut pending),
                    fields: Vec::new(),
                });
                index += 1;
                continue;
            }

            if let Some(name) = declared(line, "pub struct ", " {") {
                let doc = take_doc(&mut pending);
                let mut fields = Vec::new();
                let mut field_doc: Vec<String> = Vec::new();
                let mut cursor = index + 1;
                while cursor < lines.len() && lines[cursor] != "}" {
                    let body = lines[cursor];
                    if let Some(rest) = body
                        .strip_prefix("    /// ")
                        .or_else(|| body.strip_prefix("    ///"))
                    {
                        field_doc.push(rest.to_owned());
                    } else if let Some((field_name, field_type)) = member(body) {
                        fields.push(Field {
                            name: field_name,
                            ty: field_type,
                            line: cursor + 1,
                            doc: take_doc(&mut field_doc),
                        });
                    } else {
                        field_doc.clear();
                    }
                    cursor += 1;
                }
                model.structs.insert(
                    name,
                    Struct {
                        file: display.clone(),
                        line: index + 1,
                        doc,
                        fields,
                    },
                );
                index = cursor;
                continue;
            }

            if let Some(name) = declared(line, "pub enum ", " {") {
                let doc = take_doc(&mut pending);
                let mut variants: Vec<Vec<(String, String)>> = Vec::new();
                let mut variant_tuples: Vec<Vec<String>> = Vec::new();
                let mut cursor = index + 1;
                while cursor < lines.len() && lines[cursor] != "}" {
                    let body = lines[cursor];
                    let is_variant = body.len() > 4
                        && body.starts_with("    ")
                        && !body.starts_with("     ")
                        && body[4..].starts_with(|character: char| character.is_ascii_uppercase());
                    if is_variant {
                        variants.push(Vec::new());
                        variant_tuples.push(tuple_arm_types(&body[4..]));
                    } else if let Some(current) = variants.last_mut()
                        && let Some(rest) = body.strip_prefix("        ")
                        && let Some((field_name, field_type)) = member(&format!("    {rest}"))
                    {
                        current.push((field_name, field_type));
                    }
                    cursor += 1;
                }
                model.enums.insert(
                    name,
                    Enum {
                        doc,
                        variants,
                        variant_tuples,
                    },
                );
                index = cursor;
                continue;
            }

            if line.trim_start().starts_with("fn ") && line.starts_with("    fn ") {
                let mut signature = String::new();
                let mut cursor = index;
                while cursor < lines.len() {
                    signature.push_str(lines[cursor].trim());
                    signature.push(' ');
                    if lines[cursor].trim_end().ends_with(';')
                        || lines[cursor].trim_end().ends_with('{')
                    {
                        break;
                    }
                    cursor += 1;
                }
                let name: String = line.trim_start()["fn ".len()..]
                    .chars()
                    .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
                    .collect();
                model.methods.push((name, signature));
                model.method_sites.push((display.clone(), index + 1));
                index = cursor + 1;
                pending.clear();
                continue;
            }

            if let Some(rest) = line.strip_prefix("impl ")
                && let Some(subject) = rest.strip_suffix(" {")
            {
                let subject = subject.trim().to_owned();
                let mut cursor = index + 1;
                while cursor < lines.len() && lines[cursor] != "}" {
                    if lines[cursor].starts_with("    pub fn ") {
                        let mut signature = String::new();
                        let mut scan = cursor;
                        while scan < lines.len() && scan < cursor + 12 {
                            signature.push_str(lines[scan].trim());
                            signature.push(' ');
                            if lines[scan].trim_end().ends_with('{') {
                                break;
                            }
                            scan += 1;
                        }
                        if let Some(handed_back) = returned_inner(&signature) {
                            model
                                .inherent_returns
                                .entry(subject.clone())
                                .or_default()
                                .insert(handed_back);
                        }
                    }
                    cursor += 1;
                }
                index = cursor;
                pending.clear();
                continue;
            }

            pending.clear();
            index += 1;
        }
    }
    model
}

/// The types inside `Variant(A, B)`, or empty for a braced or unit variant.
fn tuple_arm_types(variant: &str) -> Vec<String> {
    let Some(open) = variant.find('(') else {
        return Vec::new();
    };
    let Some(close) = variant.rfind(')') else {
        return Vec::new();
    };
    variant[open + 1..close]
        .split(',')
        .filter_map(|part| {
            let bare = unwrap_container(part);
            (!bare.is_empty()).then_some(bare)
        })
        .collect()
}

/// `pub struct Name(...);` or `pub struct Name;`.
fn declared_tuple(line: &str) -> Option<String> {
    let rest = line.strip_prefix("pub struct ")?;
    let name: String = rest
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
        .collect();
    if name.is_empty() {
        return None;
    }
    let tail = &rest[name.len()..];
    (tail.starts_with('(') || tail == ";").then_some(name)
}

fn declared(line: &str, prefix: &str, suffix: &str) -> Option<String> {
    let rest = line.strip_prefix(prefix)?;
    let name = rest.strip_suffix(suffix)?;
    name.chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_')
        .then(|| name.to_owned())
}

/// `    pub name: Type,` or `    name: Type,`.
fn member(line: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix("    ")?;
    if rest.starts_with(' ') {
        return None;
    }
    let rest = rest.strip_prefix("pub ").unwrap_or(rest);
    let body = rest.strip_suffix(',')?;
    let (name, ty) = body.split_once(": ")?;
    name.chars()
        .all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
        .then(|| (name.to_owned(), ty.trim().to_owned()))
}

fn is_precondition_field(name: &str) -> bool {
    name.starts_with("expected_") || name.contains("precondition")
}

const VERSION_ROLES: &[&str] = &[
    "revision",
    "digest",
    "epoch",
    "generation",
    "ordinal",
    "version",
];

/// Does this member pin a version of the row? A member that delegates to a
/// nested precondition pins one exactly when that nested precondition cannot
/// itself say the row is absent -- otherwise an arm could launder a required
/// value through one indirection.
fn asserts_a_version(field_name: &str, field_type: &str, model: &Model, depth: u8) -> bool {
    let bare = strip_crate(field_type);
    if bare.starts_with("Option<") {
        return false;
    }
    if depth > 0 && bare.contains("Precondition") {
        return !can_represent_absence_at(&bare, model, depth - 1);
    }
    let name = field_name.to_ascii_lowercase();
    let ty = bare.to_ascii_lowercase();
    VERSION_ROLES
        .iter()
        .any(|role| name.contains(role) || ty.contains(role))
}

fn can_represent_absence_at(ty: &str, model: &Model, depth: u8) -> bool {
    let bare = strip_crate(ty);
    if bare.starts_with("Option<") {
        return true;
    }
    let Some(declaration) = model.enums.get(&bare) else {
        return false;
    };
    declaration.variants.iter().any(|variant| {
        !variant
            .iter()
            .any(|(name, ty)| asserts_a_version(name, ty, model, depth))
    })
}

fn can_represent_absence(ty: &str, model: &Model) -> bool {
    can_represent_absence_at(ty, model, 3)
}

const VERSION_SUFFIXES: &[&str] = &[
    "Revision",
    "RecordDigest",
    "Digest",
    "Epoch",
    "Ordinal",
    "Generation",
];

fn camel(snake: &str) -> String {
    snake
        .split('_')
        .map(|part| {
            let mut characters = part.chars();
            match characters.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + characters.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

fn doc_links(doc: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut rest = doc;
    while let Some(position) = rest.find("[`") {
        rest = &rest[position + 2..];
        let Some(end) = rest.find("`]") else { break };
        let target = &rest[..end];
        rest = &rest[end + 2..];
        let head = target.rsplit("::").find(|part| *part != "crate");
        let Some(head) = head else { continue };
        let name: String = head
            .chars()
            .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
            .collect();
        if name.starts_with(|character: char| character.is_ascii_uppercase()) {
            out.insert(name);
        }
    }
    out
}

struct Resolution {
    row: String,
    route: &'static str,
}

fn resolve(
    write_set: &str,
    field: &Field,
    rows: &BTreeSet<String>,
    model: &Model,
) -> Vec<Resolution> {
    let ty = unwrap_container(&field.ty);

    if rows.contains(&ty) {
        return vec![Resolution {
            row: ty,
            route: "the field's type is the row",
        }];
    }
    for suffix in VERSION_SUFFIXES {
        if let Some(stem) = ty.strip_suffix(suffix) {
            let candidate = format!("{stem}V1");
            if rows.contains(&candidate) {
                return vec![Resolution {
                    row: candidate,
                    route: "scalar newtype naming the row plus a version role",
                }];
            }
        }
    }
    for suffix in ["PreconditionV1", "Precondition"] {
        let Some(stem) = ty.strip_suffix(suffix) else {
            continue;
        };
        let exact = format!("{stem}V1");
        if rows.contains(&exact) {
            return vec![Resolution {
                row: exact,
                route: "precondition type named for the row",
            }];
        }
        let prefixed: Vec<&String> = rows.iter().filter(|row| row.starts_with(stem)).collect();
        if let [single] = prefixed.as_slice() {
            return vec![Resolution {
                row: (*single).clone(),
                route: "precondition type is the unique prefix of a row's name",
            }];
        }
        if let Some(declaration) = model.structs.get(&ty) {
            let mut inner = BTreeSet::new();
            for member in &declaration.fields {
                let member_type = unwrap_container(&member.ty);
                if rows.contains(&member_type) {
                    inner.insert(member_type.clone());
                }
                for version in VERSION_SUFFIXES {
                    if let Some(stem) = member_type.strip_suffix(version) {
                        let candidate = format!("{stem}V1");
                        if rows.contains(&candidate) {
                            inner.insert(candidate);
                        }
                    }
                }
            }
            if inner.len() == 1 {
                return vec![Resolution {
                    row: inner.into_iter().next().expect("one"),
                    route: "precondition struct whose members resolve to one row",
                }];
            }
        }
    }
    let mut stem = field.name.as_str();
    stem = stem.strip_prefix("expected_").unwrap_or(stem);
    for role in ["_record_digest", "_revision", "_digest", "_epoch"] {
        stem = stem.strip_suffix(role).unwrap_or(stem);
    }
    let needle = camel(stem);
    if !needle.is_empty()
        && let Some(declaration) = model.structs.get(write_set)
    {
        let hits: BTreeSet<String> = declaration
            .fields
            .iter()
            .map(|sibling| unwrap_container(&sibling.ty))
            .filter(|candidate| rows.contains(candidate) && candidate.contains(&needle))
            .collect();
        if hits.len() == 1 {
            return vec![Resolution {
                row: hits.into_iter().next().expect("one"),
                route: "field name stems to the one row this write set carries",
            }];
        }
    }
    // Last route: the author states it. EVERY row this block links is taken as a
    // subject of the compare-and-set, so a precondition that dispatches over
    // several rows -- a repair target, say -- resolves to all of them, and a
    // link to a row must not be written here for any other reason. Links to
    // anything that is not a row these write sets propose are ignored, so
    // pointing at a verifier or an error variant costs nothing.
    for doc in [
        field.doc.clone(),
        model
            .structs
            .get(&ty)
            .map(|declaration| declaration.doc.clone())
            .or_else(|| {
                model
                    .enums
                    .get(&ty)
                    .map(|declaration| declaration.doc.clone())
            })
            .unwrap_or_default(),
    ] {
        let declared: Vec<String> = doc_links(&doc)
            .into_iter()
            .filter(|name| model.structs.contains_key(name) || model.enums.contains_key(name))
            .collect();
        // Prefer rows these write sets propose. A subject that is NOT one of
        // those -- a capacity ledger written through a verified wrapper, an
        // admission term this wave only reads, an audience policy owned
        // elsewhere -- still RESOLVES: the member is attributed, and the birth
        // question simply does not arise for a row no write set here opens.
        let preferred: Vec<String> = declared
            .iter()
            .filter(|name| rows.contains(*name))
            .cloned()
            .collect();
        let chosen = if preferred.is_empty() {
            declared
        } else {
            preferred
        };
        if !chosen.is_empty() {
            return chosen
                .into_iter()
                .map(|row| Resolution {
                    row,
                    route: "an intra-doc link declaring the row this compare-and-set is on",
                })
                .collect();
        }
    }
    Vec::new()
}

fn write_set_parts(name: &str) -> bool {
    ["WriteSetPartsV1", "MutationSetPartsV1", "MutationPartsV1"]
        .iter()
        .any(|suffix| name.ends_with(suffix))
}

/// The write-set parts a method takes, if any. Only a WRITE creates a row, so a
/// service facade with no write set can never be the creator clause (iii) looks
/// for; it delegates to one of these.
fn parts_taken_by(signature: &str) -> Option<String> {
    for suffix in ["WriteSetV1", "MutationSetV1"] {
        let mut rest = signature;
        while let Some(position) = rest.find(suffix) {
            let head = &rest[..position];
            let start = head
                .rfind(|character: char| !character.is_ascii_alphanumeric() && character != '_')
                .map_or(0, |index| index + 1);
            let name = format!("{}{suffix}", &head[start..]);
            if name.len() > suffix.len() {
                return Some(name.replace(suffix, &suffix.replace("V1", "PartsV1")));
            }
            rest = &rest[position + suffix.len()..];
        }
    }
    None
}

fn returned(signature: &str) -> Option<String> {
    let after = signature.split("->").nth(1)?;
    let inner = match after.find("Result<") {
        Some(position) => &after[position + "Result<".len()..],
        None => after,
    };
    let name: String = strip_crate(inner)
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
        .collect();
    (!name.is_empty()).then_some(name)
}

fn yields_row(returned_type: &str, row: &str, model: &Model) -> bool {
    if returned_type == row {
        return true;
    }
    model.structs.get(returned_type).is_some_and(|declaration| {
        declaration.fields.iter().any(|field| {
            let bare = strip_crate(&field.ty);
            bare == row || bare == format!("Vec<{row}>")
        })
    })
}

struct Violation {
    row: String,
    write_set: String,
    field: String,
    ty: String,
    file: String,
    line: usize,
    route: &'static str,
    carriers: Vec<String>,
}

struct Unresolved {
    write_set: String,
    field: String,
    ty: String,
    file: String,
    line: usize,
}

struct Sweep {
    violations: Vec<Violation>,
    unresolved: Vec<Unresolved>,
    rows: BTreeSet<String>,
    preconditions: usize,
}

fn sweep(model: &Model) -> Sweep {
    let parts: Vec<&String> = model
        .structs
        .keys()
        .filter(|name| write_set_parts(name))
        .collect();

    // Carriers: every write set naming a record as a bare member. A record
    // carried ONLY under a precondition name is an input this wave reads, not a
    // row it writes, so it is not part of the population.
    let mut carriers: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    let mut rows: BTreeSet<String> = BTreeSet::new();
    for write_set in &parts {
        for field in &model.structs[*write_set].fields {
            let bare = strip_crate(&field.ty);
            // A row is something a store DERIVES. A `Signed*` or `Verified*`
            // member never is: this wave's own law forbids a store signing its
            // own write, so such a member is always an input the caller carried
            // in or a value another party minted, and its revision is nobody's
            // to compare-and-set here. A `*PreconditionV1` member is the
            // comparison itself, not the row.
            if !model.structs.contains_key(&bare)
                || bare.ends_with("PreconditionV1")
                || bare.starts_with("Signed")
                || bare.starts_with("Verified")
            {
                continue;
            }
            carriers
                .entry(bare.clone())
                .or_default()
                .push(((*write_set).clone(), field.name.clone()));
            if !is_precondition_field(&field.name) {
                rows.insert(bare);
            }
        }
    }

    let mut violations = Vec::new();
    let mut unresolved = Vec::new();
    let mut preconditions = 0usize;

    // Attribute every precondition field to a row, or record it as unresolved.
    let mut attributed: BTreeMap<String, Vec<(String, Field, &'static str)>> = BTreeMap::new();
    for write_set in &parts {
        for field in &model.structs[*write_set].fields {
            if !is_precondition_field(&field.name) {
                continue;
            }
            preconditions += 1;
            let resolutions = resolve(write_set, field, &rows, model);
            if resolutions.is_empty() {
                unresolved.push(Unresolved {
                    write_set: (*write_set).clone(),
                    field: field.name.clone(),
                    ty: field.ty.clone(),
                    file: model.structs[*write_set].file.clone(),
                    line: field.line,
                });
                continue;
            }
            for resolution in resolutions {
                attributed.entry(resolution.row).or_default().push((
                    (*write_set).clone(),
                    field.clone(),
                    resolution.route,
                ));
            }
        }
    }

    for row in &rows {
        let Some(carried_by) = carriers.get(row) else {
            continue;
        };
        let carrier_names: BTreeSet<String> = carried_by
            .iter()
            .map(|(write_set, _)| write_set.clone())
            .collect();
        let on_carriers: Vec<&(String, Field, &'static str)> = attributed
            .get(row)
            .map(|entries| {
                entries
                    .iter()
                    .filter(|(write_set, _, _)| carrier_names.contains(write_set))
                    .collect()
            })
            .unwrap_or_default();
        if on_carriers.is_empty() {
            continue;
        }

        // (i) a carrier's precondition can say the row is not there.
        let absence = on_carriers
            .iter()
            .any(|(_, field, _)| can_represent_absence(&field.ty, model));
        // (ii) a carrier proposes the row with no precondition on it.
        let guarded: BTreeSet<&String> = on_carriers
            .iter()
            .map(|(write_set, _, _)| write_set)
            .collect();
        let unconditional = carrier_names.iter().any(|name| !guarded.contains(name));
        // (iii) a different write yields the row.
        let foreign = model.methods.iter().any(|(_, signature)| {
            returned(signature).is_some_and(|value| yields_row(&value, row, model))
                && parts_taken_by(signature).is_some_and(|taken| !carrier_names.contains(&taken))
        });

        // (iv) a READ hands the row back unconditionally. A method that takes no
        // write set and returns the row outside `Option` is a contract statement
        // that the row is always there -- the store materialises it rather than
        // waiting for a write to open it -- so a required precondition on it has
        // a value at every call. This is the "its only backing read returns
        // `Option`" half of the law, read from the other side.
        // A caller-facing service RPC is excluded, and a perturbation control is
        // what found that: `TenancyMigrationCoordinationService::
        // append_participant_receipt` returns the receipt ledger and takes no
        // write set, so without this it exonerated the very row the wave was
        // asked to fix. A facade is a mirror of a store write, never an
        // independent creator, and it is identifiable by the verified invocation
        // it takes -- which is exactly what makes it caller-facing.
        let materialised = model.methods.iter().any(|(_, signature)| {
            parts_taken_by(signature).is_none()
                && !signature.contains("Invocation")
                && returned(signature).is_some_and(|value| value == *row)
        });

        if absence || unconditional || foreign || materialised {
            continue;
        }
        for (write_set, field, route) in on_carriers {
            violations.push(Violation {
                row: row.clone(),
                write_set: write_set.clone(),
                field: field.name.clone(),
                ty: field.ty.clone(),
                file: model.structs[write_set].file.clone(),
                line: field.line,
                route,
                carriers: carried_by
                    .iter()
                    .map(|(name, member)| format!("{name}::{member}"))
                    .collect(),
            });
        }
    }

    violations.sort_by(|left, right| {
        (&left.row, &left.write_set, &left.field).cmp(&(&right.row, &right.write_set, &right.field))
    });
    unresolved.sort_by(|left, right| {
        (&left.write_set, &left.field).cmp(&(&right.write_set, &right.field))
    });
    Sweep {
        violations,
        unresolved,
        rows,
        preconditions,
    }
}

#[test]
fn the_instrument_discriminates_before_it_certifies() {
    let root = repo_root();
    let model = parse(&wave_source_files(), &root);

    assert!(
        model.structs.len() > 400 && model.enums.len() > 100 && model.methods.len() > 100,
        "the parser found {} structs, {} enums, {} trait methods -- it is not reading the wave",
        model.structs.len(),
        model.enums.len(),
        model.methods.len()
    );

    // Absence is judged by shape. An enum with an arm asserting no version can
    // represent absence; one whose every arm asserts a version cannot.
    assert!(can_represent_absence("Option<u64>", &model));
    assert!(
        can_represent_absence("crate::TransferExecutionItemPreconditionV1", &model),
        "an enum with an arm that asserts no version of the row represents absence"
    );
    assert!(
        can_represent_absence("BindingWritePrecondition", &model),
        "`Unbound` asserts no version and must be recognised without naming it"
    );
    assert!(
        !can_represent_absence("SourceFenceDirectiveLedgerRevision", &model),
        "a scalar newtype cannot say the row is absent"
    );
    assert!(
        !can_represent_absence("crate::InstalledServingAuthorityV1", &model),
        "a whole record required by value cannot say the row is absent"
    );
    assert!(
        !can_represent_absence("MovementBudgetAuthorityPreconditionV1", &model),
        "a precondition struct pinning a revision and a record digest cannot say the \
         row is absent"
    );

    let sweep = sweep(&model);
    assert!(
        sweep.preconditions > 100 && sweep.rows.len() > 50,
        "the sweep saw {} precondition members over {} rows -- it is not reading the wave",
        sweep.preconditions,
        sweep.rows.len()
    );
    assert!(
        sweep.rows.contains("TransferExecutionLedgerV1")
            && sweep.rows.contains("SourceFenceDirectiveLedgerV1"),
        "the row population must contain the rows the seats reasoned about"
    );
}

#[test]
fn every_required_precondition_has_a_reachable_first_value() {
    let root = repo_root();
    let model = parse(&wave_source_files(), &root);
    let sweep = sweep(&model);

    let unresolved: String = sweep
        .unresolved
        .iter()
        .map(|item| {
            format!(
                "\n  {}:{}  {}::{}: {}",
                item.file, item.line, item.write_set, item.field, item.ty
            )
        })
        .collect();
    let violations_first: String = sweep
        .violations
        .iter()
        .map(|item| {
            format!(
                "\n  {}:{}\n      {}::{}: {}\n      row:      {}  (resolved by: {})\n      carried by: {}\n",
                item.file,
                item.line,
                item.write_set,
                item.field,
                item.ty,
                item.row,
                item.route,
                item.carriers.join(", ")
            )
        })
        .collect();
    assert!(
        sweep.unresolved.is_empty(),
        "LAW A. The rows below are already named as violations, and are printed first so \
         one run is the whole work list:{violations_first}\
         \nLAW A, instrument coverage. The compare-and-set members below could not be \
         attributed to the row they guard by any structural route, so the law was never \
         applied to them. This is not a licence to skip them: an unattributed \
         absence-capable precondition also cannot discharge clause (i) for the row it \
         really guards, which manufactures a false positive elsewhere. Widen a route, or \
         state the row in a doc: a single intra-doc link on the member -- or on its type's \
         declaration -- naming the record this compare-and-set is on.{unresolved}"
    );

    let violations: String = sweep
        .violations
        .iter()
        .map(|item| {
            format!(
                "\n  {}:{}\n      {}::{}: {}\n      row:      {}  (resolved by: {})\n      carried by: {}\n",
                item.file,
                item.line,
                item.write_set,
                item.field,
                item.ty,
                item.row,
                item.route,
                item.carriers.join(", ")
            )
        })
        .collect();
    assert!(
        sweep.violations.is_empty(),
        "LAW A. Each member below is a compare-and-set required BY VALUE on a row that no \
         write in these crates can have created first: no carrier's precondition can say \
         the row is absent, no carrier proposes it unconditionally, and no other write \
         yields it. At the first execution the caller has no legal value. Give the \
         precondition an absent representation -- an `Option` or an enum arm -- say what \
         absence asserts, name the refusal the store raises when the assertion is false, \
         and mirror the optionality on the wire.{violations}"
    );
}

// ============================================================================
// LAW C, executable: a required value must be READABLE under the authority the
// write itself takes.
//
// THE LAW. A compare-and-set precondition required BY VALUE on a write set has
// no legal value at any execution -- not only the first -- if no surface in
// these crates yields its row under an authority the write's own arm can hold.
// The caller must then either invent the value or reach for a second authority
// nothing says it may hold. This is the OTHER FACE of Law A. Law A asks whether
// a value ever exists; Law C asks whether the party performing the write can
// see it.
//
// WHY THIS FILE AND NOT A NEW ONE. Law C is the same population as Law A --
// write-set parts, precondition members, rows -- read with a different
// question, so it reuses this file's parser, its `resolve` routes and its
// `can_represent_absence` shape test. A second file would fork the population,
// and a population that two instruments disagree about is how round 8 lost a
// wire.
//
// THE SUBJECT IS THE (PRECONDITION, AUTHORITY ARM) PAIR, NOT THE PRECONDITION.
// A write set whose `authority` member is an enum -- `Request(..) |
// Reconciler(..)` -- takes a different authority on each arm, and the seats
// found exactly the shape where one arm is served and the other is not. Judging
// the precondition once, over the union of its arms, hides that. Every arm is
// an obligation of its own and is printed as its own line.
//
// THE DISCHARGE ROUTES, each named in the output so a reader can see which one
// fired. `fn discharge` tries them READ, SUBJECT, OTHER WRITE, LEASE-GATED
// READ, DECLARED, OFF-AXIS, UNAUTHENTICATED, and first match wins, so the
// order is load-bearing; the letters below are labels for reference, NOT the
// try order. This heading once said FIVE above seven routes and claimed the
// list gave the try order, which it did not -- the last three were reversed.
// It states no count now: the routes are the arms of `fn discharge` and a
// tally here would be the census this file's own sweep exists to delete.
//
//   (a) READ -- a method that takes no write set, is not a caller-facing
//       facade, is not gated on a lease, does not return a `Committed*ClaimV1`,
//       and yields the row under an authority the arm holds.
//   (b) SUBJECT -- the row is reachable from the reconciliation subject the
//       arm's reconciler is handed.
//   (c) OTHER WRITE -- a different write, taken under an authority the arm
//       holds, yields the row, so an earlier call in the same sequence had it.
//   (d) LEASE-GATED READ -- as (a) but the read also demands a lease. Accepted,
//       and reported separately, because a lease is a coordination fact rather
//       than an authority one, and an arm that can hold the lease can perform
//       the read.
//   (e) UNAUTHENTICATED READ -- a read that demands no authority at all.
//       Accepted, and reported separately: it discharges this law trivially and
//       is a finding for a different one.
//   (f) OFF-AXIS ARM -- the arm holds something that is not half of any derived
//       read/persistence pair, or the write set declares no `authority` member
//       at all. There is no ordering to measure, so the law has no subject.
//       Both members this fires on carry a written ruling.
//   (g) DECLARED ON THE MEMBER -- the member's own doc says the authority
//       question is discharged and names the surface. The escape for a real
//       route deeper than route (a) looks; see `declares_its_route`.
//
// "AN AUTHORITY THE ARM HOLDS" IS THE ORDERING, AND IT IS DERIVED, NOT LISTED.
// A read taken under `X...ReadAuthorityV1` is performable by a holder of
// `X...PersistenceAuthorityV1`, because persistence subsumes read. The pairs are
// computed from the tree: two tuple newtypes over the SAME inner type whose
// names agree once `PersistenceAuthorityV1` and `ReadAuthorityV1` are removed,
// AND where the persistence type declares a method that actually hands back its
// read twin. The second half is what makes the conjunct load-bearing rather than
// decorative: delete `BindingPersistenceAuthorityV1::read_authority` and every
// obligation resting on that pair goes open, which is the correct answer,
// because without it a writer has no way to perform the read.
// Typing the seven pairs in as a table would be the name-keyed census this wave
// keeps being caught by; deriving them means a pair born tomorrow is in scope
// the day it is declared. The ordering runs ONE WAY: a read authority never
// discharges a write.
//
// THE DEPTH ASYMMETRY IS DELIBERATE, AND IS THIS TEST'S SHARPEST BLIND SPOT.
// Route (a) uses Law A's own `yields_row`: the returned type IS the row, or the
// row is a direct member of it. One level. Route (b) walks the subject's type
// graph TRANSITIVELY. The two are different because the objects are different:
// a read hands back a record and the caller reads a field of it, while a
// reconciliation subject is a whole object graph handed over for the reconciler
// to own, so anything inside it is in hand. The cost is real and is recorded
// rather than hidden: a value riding three levels down inside a page --
// `CellCatalogReader::read_page` -> `CellCatalogEntryV1::admission_term` -- is
// invisible to route (a), and that one is discharged by a sentence at
// `CellReservationWriteSetPartsV1::admission_precondition` instead.
//
// A COMMIT OBSERVATION IS NOT A READ, and route (a) refuses it by SHAPE rather
// than by the `observe_` name: any method returning a `Committed*ClaimV1` is
// excluded. The wave's own reason, at
// `PublishTransferExecutionPermitWriteSetPartsV1::issuance_precondition`, is
// that a commit observation carries the values AS OF THE COMMIT while the
// publication compare-and-set is on the row's CURRENT revision and digest.
// `TransferExecutionStore::load_item` and
// `MigrationReleaseStore::load_release_issuance` exist BESIDE their lanes'
// observers for exactly this reason, and a law that let an observer discharge a
// precondition would have certified both lanes without them.
//
// WHAT THIS TEST STILL CANNOT SEE. Whether a read that DEMANDS a persistence
// authority is right to demand one. The ordering says a writer may read; it
// does not say a loader may insist on write authority.
// `ServingAuthorityControlCommitObserver`'s two sibling loaders,
// `load_installation_issuance` and `load_freeze_intent`, are that shape. Judging
// them needs a rule about what a read may require, which is a different law.
//
// It asserts that the offending set is EMPTY and prints its members. It asserts
// no count and no non-zero quantity.

/// The Ok type of a signature, with one container layer removed.
///
/// Law A's `returned` stops at the outermost name, deliberately: its clause
/// (iv) turns on the row arriving OUTSIDE an `Option`, so `Option` is the
/// answer there. Law C asks a different question -- can this party see the
/// value at all -- and an `Option` read answers it, so here the container is
/// opened.
fn returned_inner(signature: &str) -> Option<String> {
    let after = signature.split("->").nth(1)?;
    let start = after.find("Result<")? + "Result<".len();
    let mut depth = 0usize;
    let mut out = String::new();
    for character in after[start..].chars() {
        match character {
            '<' => depth += 1,
            '>' if depth == 0 => break,
            '>' => depth -= 1,
            ',' if depth == 0 => break,
            _ => {}
        }
        out.push(character);
    }
    let bare = unwrap_container(out.trim());
    (!bare.is_empty()).then_some(bare)
}

/// A read/persistence authority pair: `persistence -> read`.
fn authority_pairs(model: &Model) -> BTreeMap<String, String> {
    const PERSISTENCE: &str = "PersistenceAuthorityV1";
    const READ: &str = "ReadAuthorityV1";
    let mut pairs = BTreeMap::new();
    for (persistence, persistence_inner) in &model.newtypes {
        let Some(stem) = persistence.strip_suffix(PERSISTENCE) else {
            continue;
        };
        let candidate = format!("{stem}{READ}");
        let same_invocation = model
            .newtypes
            .get(&candidate)
            .is_some_and(|read_inner| read_inner == persistence_inner);
        // Nameable is not performable. The ordering counts only when the
        // persistence authority actually declares a method handing back its read
        // twin; deleting that accessor must take the pair -- and every
        // obligation resting on it -- out of scope, or this conjunct is
        // decoration.
        let performable = model
            .inherent_returns
            .get(persistence)
            .is_some_and(|handed_back| handed_back.contains(&candidate));
        if same_invocation && performable {
            pairs.insert(persistence.clone(), candidate);
        }
    }
    pairs
}

/// `Option<Vec<Box<T>>>` down to `T`. `unwrap_container` opens one layer, which
/// is all Law A needs; a subject graph nests them.
fn fully_unwrapped(ty: &str) -> String {
    let mut current = strip_crate(ty);
    loop {
        let opened = current
            .strip_prefix("Option<")
            .or_else(|| current.strip_prefix("Vec<"))
            .or_else(|| current.strip_prefix("Box<"));
        match opened {
            Some(rest) => {
                let mut inner = rest.to_owned();
                if inner.ends_with('>') {
                    inner.pop();
                }
                current = strip_crate(&inner);
            }
            None => return current,
        }
    }
}

/// Every type transitively reachable from `start`, through struct members and
/// enum variant members. Used only for the reconciliation subject.
fn type_closure(start: &str, model: &Model) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut stack = vec![strip_crate(start)];
    while let Some(current) = stack.pop() {
        let bare = fully_unwrapped(&current);
        if !seen.insert(bare.clone()) {
            continue;
        }
        if let Some(declaration) = model.structs.get(&bare) {
            for field in &declaration.fields {
                stack.push(fully_unwrapped(&field.ty));
            }
        }
        if let Some(declaration) = model.enums.get(&bare) {
            for variant in &declaration.variants {
                for (_, ty) in variant {
                    stack.push(fully_unwrapped(ty));
                }
            }
            for types in &declaration.variant_tuples {
                for ty in types {
                    stack.push(fully_unwrapped(ty));
                }
            }
        }
    }
    seen
}

/// The reconciliation subjects a holder of each persistence authority is handed.
///
/// Derived, not listed: a candidate listing takes the pair's READ authority and
/// hands back something carrying the subject enum, so the subject a reconciler
/// owns is whatever it can obtain that way.
fn subjects_by_authority(
    model: &Model,
    pairs: &BTreeMap<String, String>,
) -> BTreeMap<String, BTreeSet<String>> {
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (persistence, read) in pairs {
        for (_, signature) in &model.methods {
            if parts_taken_by(signature).is_some() || signature.contains("Invocation ") {
                continue;
            }
            if !signature.contains(read.as_str()) {
                continue;
            }
            let Some(returned_type) = returned_inner(signature) else {
                continue;
            };
            for reachable in type_closure(&returned_type, model) {
                if reachable.ends_with("SubjectV1") && model.enums.contains_key(&reachable) {
                    out.entry(persistence.clone())
                        .or_default()
                        .insert(reachable);
                }
            }
        }
    }
    out
}

/// The authority each arm of a write set's `authority` member holds, as
/// `(arm label, authority type)`.
fn write_authorities(parts: &str, model: &Model) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let Some(declaration) = model.structs.get(parts) else {
        return out;
    };
    for field in &declaration.fields {
        if field.name != "authority" {
            continue;
        }
        let bare = unwrap_container(&field.ty);
        match model.enums.get(&bare) {
            Some(enumeration) => {
                let named = enumeration.variants.iter().flat_map(|variant| {
                    variant
                        .iter()
                        .map(|(_, ty)| unwrap_container(ty))
                        .collect::<Vec<_>>()
                });
                let tupled = enumeration.variant_tuples.iter().flat_map(|types| {
                    types
                        .iter()
                        .map(|ty| unwrap_container(ty))
                        .collect::<Vec<_>>()
                });
                for (index, inner) in named.chain(tupled).enumerate() {
                    if inner.contains("Authority") || inner.contains("Token") {
                        out.push((format!("`{bare}` arm {index}"), inner));
                    }
                }
            }
            None => out.push(("(single)".to_owned(), bare)),
        }
    }
    out
}

fn holds(arm_authority: &str, signature: &str, pairs: &BTreeMap<String, String>) -> Option<String> {
    if signature.contains(arm_authority) {
        return Some(format!("the arm's own `{arm_authority}`"));
    }
    let read = pairs.get(arm_authority)?;
    signature
        .contains(read.as_str())
        .then(|| format!("`{read}`, the read twin `{arm_authority}` subsumes"))
}

/// A method that demands no authority of any kind.
fn takes_no_authority(signature: &str) -> bool {
    !signature.contains("Authority")
}

struct Discharge {
    route: &'static str,
    why: String,
}

/// The declared discharge: the member's own doc states that the authority
/// question is answered and says how.
///
/// Keying the DISCHARGE on a wording is legitimate here for the same reason it
/// is in Law B: the discharge is a marker this crate declares, while the
/// violation is whatever shape a future author writes. Both tokens must be
/// present, so it cannot fire on ordinary prose about authorities. It exists
/// because route (a) is deliberately one level deep and a real route can be
/// three -- `CellCatalogReader::read_page` hands back a page of entries and the
/// admission term rides on an entry -- and the honest answer to that is a
/// sentence naming the route, not a widened depth that would exonerate
/// everything.
fn declares_its_route(doc: &str) -> bool {
    let lowered = doc.to_ascii_lowercase();
    lowered.contains("authority question") && lowered.contains("discharged by")
}

fn discharge(
    row: &str,
    arm_authority: &str,
    carrier: &str,
    member_doc: &str,
    model: &Model,
    pairs: &BTreeMap<String, String>,
    subjects: &BTreeMap<String, BTreeSet<String>>,
) -> Option<Discharge> {
    // Law A's `returned` stops at `Option`, deliberately: clause (iv) turns on
    // the row arriving OUTSIDE one. Law C asks a different question -- can this
    // party see the value at all -- and an `Option` read answers it, so here the
    // container is unwrapped before the row test.
    let candidates: Vec<usize> = (0..model.methods.len())
        .filter(|index| {
            returned_inner(&model.methods[*index].1)
                .is_some_and(|value| yields_row(&value, row, model))
        })
        .collect();
    let site = |index: usize| {
        let (file, line) = &model.method_sites[index];
        format!("{}:{}", file, line)
    };
    let is_facade = |signature: &str| signature.contains("Invocation ");
    let observes = |signature: &str| {
        returned_inner(signature)
            .is_some_and(|value| value.starts_with("Committed") && value.ends_with("ClaimV1"))
    };

    // (a) an ordinary read under an authority the arm holds.
    for index in &candidates {
        let (name, signature) = &model.methods[*index];
        if parts_taken_by(signature).is_some()
            || is_facade(signature)
            || signature.contains("Lease")
            || observes(signature)
        {
            continue;
        }
        if let Some(how) = holds(arm_authority, signature, pairs) {
            return Some(Discharge {
                route: "READ",
                why: format!("`{name}` at {} under {how}", site(*index)),
            });
        }
    }
    // (b) the reconciliation subject this arm's reconciler is handed.
    if let Some(owned) = subjects.get(arm_authority) {
        for subject in owned {
            if type_closure(subject, model).contains(row) {
                return Some(Discharge {
                    route: "SUBJECT",
                    why: format!("`{subject}` reaches `{row}` by type-graph walk"),
                });
            }
        }
    }
    // (c) a different write, under an authority the arm holds, yields the row.
    for index in &candidates {
        let (name, signature) = &model.methods[*index];
        let Some(taken) = parts_taken_by(signature) else {
            continue;
        };
        if taken == carrier || is_facade(signature) {
            continue;
        }
        if write_authorities(&taken, model)
            .iter()
            .any(|(_, authority)| authority == arm_authority)
        {
            return Some(Discharge {
                route: "OTHER WRITE",
                why: format!(
                    "`{name}` at {} takes `{taken}` under the same authority",
                    site(*index)
                ),
            });
        }
    }
    // (d) a lease-gated read under an authority the arm holds.
    for index in &candidates {
        let (name, signature) = &model.methods[*index];
        if parts_taken_by(signature).is_some()
            || is_facade(signature)
            || !signature.contains("Lease")
        {
            continue;
        }
        if let Some(how) = holds(arm_authority, signature, pairs) {
            return Some(Discharge {
                route: "LEASE-GATED READ",
                why: format!(
                    "`{name}` at {} under {how}, also gated on a lease",
                    site(*index)
                ),
            });
        }
    }
    // (g) the member states the route itself.
    if declares_its_route(member_doc) {
        return Some(Discharge {
            route: "DECLARED ON THE MEMBER",
            why: "the member's own doc names the surface and the authority".to_owned(),
        });
    }
    // (f) THE ARM IS OFF THIS AXIS. The law is about an ordering between a
    // write authority and its read twin. An arm holding something that is not
    // half of any derived pair -- a capability-local write token, or a write set
    // that declares no `authority` member at all -- has no ordering to be
    // measured against, so this law has no subject and must not manufacture one.
    // It is not a licence: both members this fires on carry a written ruling
    // saying which question is open and why.
    if arm_authority == "<none>"
        || !(pairs.contains_key(arm_authority) || pairs.values().any(|read| read == arm_authority))
    {
        return Some(Discharge {
            route: "OFF-AXIS ARM (ruled on the member)",
            why: format!("`{arm_authority}` is neither half of a derived read/persistence pair"),
        });
    }
    // (e) a read demanding no authority at all.
    for index in &candidates {
        let (name, signature) = &model.methods[*index];
        if parts_taken_by(signature).is_some()
            || is_facade(signature)
            || !takes_no_authority(signature)
        {
            continue;
        }
        return Some(Discharge {
            route: "UNAUTHENTICATED READ",
            why: format!("`{name}` at {} demands no authority", site(*index)),
        });
    }
    None
}

struct Obligation {
    write_set: String,
    field: String,
    ty: String,
    file: String,
    line: usize,
    arm: String,
    authority: String,
    row: String,
    resolved_by: &'static str,
}

fn law_c(model: &Model) -> (Vec<Obligation>, usize, BTreeMap<&'static str, usize>) {
    let sweep = sweep(model);
    let pairs = authority_pairs(model);
    let subjects = subjects_by_authority(model, &pairs);
    let mut open = Vec::new();
    let mut total = 0usize;
    let mut routed: BTreeMap<&'static str, usize> = BTreeMap::new();

    let parts: Vec<String> = model
        .structs
        .keys()
        .filter(|name| write_set_parts(name))
        .cloned()
        .collect();
    for write_set in &parts {
        let authorities = write_authorities(write_set, model);
        for field in &model.structs[write_set].fields {
            if !is_precondition_field(&field.name) {
                continue;
            }
            if can_represent_absence(&field.ty, model) {
                continue;
            }
            for resolution in resolve(write_set, field, &sweep.rows, model) {
                let arms = if authorities.is_empty() {
                    vec![("(no authority member)".to_owned(), "<none>".to_owned())]
                } else {
                    authorities.clone()
                };
                for (arm, authority) in arms {
                    total += 1;
                    match discharge(
                        &resolution.row,
                        &authority,
                        write_set,
                        &field.doc,
                        model,
                        &pairs,
                        &subjects,
                    ) {
                        Some(found) => *routed.entry(found.route).or_default() += 1,
                        None => open.push(Obligation {
                            write_set: write_set.clone(),
                            field: field.name.clone(),
                            ty: field.ty.clone(),
                            file: model.structs[write_set].file.clone(),
                            line: field.line,
                            arm,
                            authority,
                            row: resolution.row.clone(),
                            resolved_by: resolution.route,
                        }),
                    }
                }
            }
        }
    }
    open.sort_by(|left, right| {
        (&left.write_set, &left.field, &left.arm).cmp(&(&right.write_set, &right.field, &right.arm))
    });
    (open, total, routed)
}

#[test]
fn law_c_discriminates_before_it_certifies() {
    let root = repo_root();
    let model = parse(&wave_source_files(), &root);
    let pairs = authority_pairs(&model);

    // The ordering is derived from the tree and it is not empty.
    assert!(
        pairs.len() >= 5,
        "only {} read/persistence pairs were derived -- the ordering conjunct \
         would exonerate almost nothing",
        pairs.len()
    );
    assert_eq!(
        pairs
            .get("BindingPersistenceAuthorityV1")
            .map(String::as_str),
        Some("BindingReadAuthorityV1"),
        "the pair the wave's own `load_item` doc turns on must be derived"
    );
    // And it runs ONE WAY.
    assert!(
        !pairs.contains_key("BindingReadAuthorityV1"),
        "a read authority must never be treated as subsuming a write authority"
    );

    // `holds` accepts the arm's own type and its read twin, and refuses a
    // different family's authority over the same role.
    assert!(
        holds(
            "BindingPersistenceAuthorityV1",
            "fn get(authority: &'a BindingPersistenceAuthorityV1)",
            &pairs
        )
        .is_some()
    );
    assert!(
        holds(
            "BindingPersistenceAuthorityV1",
            "fn get(authority: &'a BindingReadAuthorityV1)",
            &pairs
        )
        .is_some()
    );
    assert!(
        holds(
            "PlacementReconciliationPersistenceAuthorityV1",
            "fn get(authority: &'a PlacementReadAuthorityV1)",
            &pairs
        )
        .is_none(),
        "an ordinary placement read must not discharge a RECONCILIATION write: \
         the two are newtypes over different signed invocations, and this is the \
         case `MovementBudgetSettlementWriteSetPartsV1::leaf_authority_precondition` \
         was the live instance of"
    );

    // The subject route is derived and reaches what the seats walked by hand.
    let subjects = subjects_by_authority(&model, &pairs);
    let binding = subjects
        .get("BindingReconciliationPersistenceAuthorityV1")
        .expect("the binding reconciler is handed a subject");
    assert!(
        binding.contains("BindingReconciliationSubjectV1"),
        "the subject a binding reconciler owns must be derived from the listing it can perform"
    );
    let reachable = type_closure("BindingReconciliationSubjectV1", &model);
    assert!(
        reachable.contains("BindingControlContributionOutboxV1"),
        "positive control: the outbox IS in the subject the reconciler is handed"
    );
    assert!(
        !reachable.contains("CellBindingIndexSnapshotV1"),
        "negative control: the cell binding index snapshot is NOT in that subject, \
         which is what made the annotation at `cell_binding_index_projection.rs` false"
    );

    let (open, total, _) = law_c(&model);
    assert!(
        total > 50,
        "law C judged only {total} obligations -- it is not reading the wave"
    );
    let _ = open;
}

#[test]
fn every_by_value_precondition_is_readable_under_its_own_authority() {
    let root = repo_root();
    let model = parse(&wave_source_files(), &root);
    let (open, total, routed) = law_c(&model);
    let routes: String = routed
        .iter()
        .map(|(route, count)| format!(" {route}={count}"))
        .collect();
    let report: String = open
        .iter()
        .map(|item| {
            format!(
                "\n  {}:{}\n      {}::{}: {}\n      row:       {}  (resolved by: {})\n      arm:       {} holds {}\n",
                item.file,
                item.line,
                item.write_set,
                item.field,
                item.ty,
                item.row,
                item.resolved_by,
                item.arm,
                item.authority
            )
        })
        .collect();
    assert!(
        open.is_empty(),
        "LAW C. Each obligation below is a compare-and-set required BY VALUE whose row \
         no surface in these crates yields under an authority that arm of the write can \
         hold. The caller must invent the value or reach for an authority nothing says \
         it may hold. Add a read under the arm's own authority or the read twin it \
         subsumes, add the reconciliation twin of an existing read, put the row in the \
         subject the reconciler is handed -- or, if the arm is genuinely off this axis, \
         say so on the member and say why.\
         \n(judged {total} obligations; discharged by{routes}){report}"
    );
}

// ============================================================================
// THE WIRE HALF, executable: a proposed successor row's message must not be bare.
//
// THE RULE. `cell/placement/v1/movement_authority.proto` and
// `tenancy/binding/v1/serving_authority.proto` each state, at file scope, that a
// caller may PROPOSE an advanced row and the store DERIVES every value it owns
// and REFUSES a disagreeing proposal. Every OTHER file holding a message a write
// set names as a proposed successor must carry a pointer back to the rule head
// for its package, and that pointer must restate the refusal, because a pointer
// that names no refusal is half a rule.
//
// WHY IT IS A TEST AND NOT AN INVARIANT. The check this replaces was: the files
// naming a refusal are exactly the files carrying the pointer, plus the rule
// head. That is a consistency check over the ALREADY-COVERED set. Adding a
// zero-comment file changes neither list, so it accepts and does not gate, and
// two zero-comment files holding successor rows -- `work_snapshot.proto`, whose
// `BindingWorkSnapshotProgressV1` is proposed as `next`, and
// `write_authority_consumer.proto`, which declares the transition record itself
// as `{previous, next}` -- passed it while carrying no comment at all.
//
// THE POPULATION IS DERIVED FROM THE RUST, NOT FROM THE PROTO. A sweep over
// proto files can only ever re-find the files it already knows about. This one
// starts from the write sets: every record a `*WriteSetPartsV1` carries as a
// bare member under a name that is `next` or `next_*`, or that a precondition on
// the SAME write set resolves to -- a compare-and-set is how a write advances a
// row -- is a proposed successor. Two routes, because either alone is a key: the
// name route missed `next_capacity` behind a `Verified*` wrapper, and the
// precondition route misses a row advanced with no compare-and-set.
//
// A `Verified*` next-state member IS a successor here, even though Law A
// excludes it from its row population. The two exclusions answer different
// questions: Law A asks who can create the row first and a minted wrapper has a
// verifier for that, while the wire asks who OWNS the values in the message,
// and the message is the same message either way. `CellCapacityLedgerV1`,
// proposed as `next_capacity: VerifiedCellCapacityLedgerV1`, is the instance.
//
// EVERY ROW MUST MAP TO A MESSAGE OR SAY WHY NOT. An unmapped name is not
// skipped: it fails, unless the row's own Rust declaration says it has no wire
// form. `CellReservationEffectRecordV1` is that case and says so.

fn proto_files() -> Vec<PathBuf> {
    let root = repo_root();
    let mut out = Vec::new();
    for relative in [
        "cell/facade/proto/cell/placement/v1",
        "tenancy/facade/proto/tenancy/binding/v1",
    ] {
        let directory = root.join(relative);
        let mut entries: Vec<PathBuf> = fs::read_dir(&directory)
            .unwrap_or_else(|error| panic!("read {}: {error}", directory.display()))
            .map(|entry| entry.expect("dir entry").path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "proto")
            })
            .collect();
        entries.sort();
        assert!(
            !entries.is_empty(),
            "no protos under {} -- the sweep would certify a zero it never looked for",
            directory.display()
        );
        out.extend(entries);
    }
    out
}

/// The rows a write set names as a proposed successor.
fn proposed_successor_rows(model: &Model) -> BTreeSet<String> {
    let sweep = sweep(model);
    let mut out = BTreeSet::new();
    for (name, declaration) in &model.structs {
        if !write_set_parts(name) {
            continue;
        }
        for field in &declaration.fields {
            if is_precondition_field(&field.name) {
                continue;
            }
            let bare = unwrap_container(&field.ty);
            if bare.ends_with("PreconditionV1") || bare.starts_with("Signed") {
                continue;
            }
            // A minted wrapper on the wire is the record it wraps.
            let row = bare.strip_prefix("Verified").unwrap_or(&bare).to_owned();
            if (field.name == "next" || field.name.starts_with("next_"))
                && (model.structs.contains_key(&row) || model.structs.contains_key(&bare))
            {
                out.insert(row);
            }
        }
        for field in &declaration.fields {
            if !is_precondition_field(&field.name) {
                continue;
            }
            for resolution in resolve(name, field, &sweep.rows, model) {
                let advanced = declaration.fields.iter().any(|sibling| {
                    !is_precondition_field(&sibling.name)
                        && unwrap_container(&sibling.ty) == resolution.row
                });
                if advanced {
                    out.insert(resolution.row);
                }
            }
        }
    }
    out
}

const RULE_HEAD: &str = "OWNERSHIP OF PROPOSED SUCCESSOR ROWS";
const PROJECTING_HEAD: &str = "THE PROJECTING SIDE OF THE OWNERSHIP RULE";
const POINTER: &str = "governed by the ownership rule stated at the head of";

#[test]
fn the_wire_ownership_sweep_discriminates_before_it_certifies() {
    let root = repo_root();
    let model = parse(&wave_source_files(), &root);
    let rows = proposed_successor_rows(&model);
    assert!(
        rows.len() > 20,
        "only {} proposed successor rows were found -- the sweep is not reading the write sets",
        rows.len()
    );
    // Both routes are load-bearing, and each is checked with a case the other
    // misses.
    assert!(
        rows.contains("CellCapacityLedgerV1"),
        "the name route must see a successor behind a `Verified*` wrapper, which \
         Law A's row population deliberately excludes"
    );
    assert!(
        rows.contains("BindingWorkSnapshotProgressV1"),
        "the row proposed as exactly `next` must be in the population -- a `next_*` \
         glob missed five of these in an earlier sweep"
    );
    assert!(
        rows.contains("CapabilityWriteAuthorityStateV1"),
        "the row whose own wire message declares the transition as `{{previous, next}}`"
    );
    // And a thing that is emphatically not a successor row stays out.
    assert!(
        !rows.contains("BindingIdempotencyRecordV1"),
        "an idempotency record is not a proposed successor"
    );

    // The rule heads exist and are found by the same text the check keys on.
    let heads: Vec<PathBuf> = proto_files()
        .into_iter()
        .filter(|path| {
            let text = fs::read_to_string(path).expect("read proto");
            text.contains(RULE_HEAD)
        })
        .collect();
    assert_eq!(
        heads.len(),
        2,
        "expected exactly two rule heads, one per package, found {heads:?}"
    );
}

#[test]
fn every_proposed_successor_message_carries_the_ownership_rule() {
    let root = repo_root();
    let model = parse(&wave_source_files(), &root);
    let rows = proposed_successor_rows(&model);

    let mut declares: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    let mut covered: BTreeMap<PathBuf, bool> = BTreeMap::new();
    for path in proto_files() {
        let text = fs::read_to_string(&path).expect("read proto");
        covered.insert(
            path.clone(),
            text.contains(RULE_HEAD) || text.contains(PROJECTING_HEAD) || text.contains(POINTER),
        );
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("message ") {
                let name: String = rest
                    .chars()
                    .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
                    .collect();
                declares.entry(name).or_default().push(path.clone());
            }
        }
    }

    let mut unmapped = Vec::new();
    let mut bare = BTreeMap::new();
    for row in &rows {
        // A Rust type with no exact-name twin is the shape a by-name mapping is
        // blindest to: `TenantCellBinding` is `TenantCellBindingV1` on the wire.
        // Try the version suffix in both directions before declaring a row
        // unmapped, and FAIL rather than skip if none of the three resolves.
        let sites = declares.get(row).or_else(|| {
            declares
                .get(&format!("{row}V1"))
                .or_else(|| row.strip_suffix("V1").and_then(|stem| declares.get(stem)))
        });
        let Some(sites) = sites else {
            // A row with no message must SAY it has no wire form, on its own
            // declaration. Silence is a finding; a sentence is a contract.
            let says_so = model.structs.get(row).is_some_and(|declaration| {
                declaration
                    .doc
                    .to_ascii_lowercase()
                    .contains("no wire form")
            });
            if !says_so {
                unmapped.push(row.clone());
            }
            continue;
        };
        for site in sites {
            if !covered[site] {
                bare.entry(site.clone())
                    .or_insert_with(Vec::new)
                    .push(row.clone());
            }
        }
    }

    let unmapped_report: String = unmapped
        .iter()
        .map(|row| format!("\n  {row}  -- no `message` in either package"))
        .collect();
    assert!(
        unmapped.is_empty(),
        "THE WIRE HALF, coverage. Each row below is named as a proposed successor by a \
         write set and has no `message` in either proto package, so no wire statement can \
         govern it and this sweep cannot judge it. Add the message, or say on the row's \
         own declaration that it has NO WIRE FORM and what binds it instead.\
         {unmapped_report}"
    );

    let bare_report: String = bare
        .iter()
        .map(|(path, rows)| {
            format!(
                "\n  {}\n      holds: {}\n",
                path.strip_prefix(&root).unwrap_or(path).display(),
                rows.join(", ")
            )
        })
        .collect();
    assert!(
        bare.is_empty(),
        "THE WIRE HALF. Each file below holds a message a write set names as a PROPOSED \
         SUCCESSOR and carries neither the ownership rule nor a pointer to it. An adapter \
         author reads this schema to learn who owns each value and would have to resolve \
         it alone. Add the pointer, and restate the refusal in it: a pointer that names no \
         refusal is half a rule.\
         \n(judged {} proposed successor rows over {} proto files){bare_report}",
        rows.len(),
        covered.len()
    );
}
