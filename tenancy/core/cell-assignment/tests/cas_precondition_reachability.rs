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
//!         so an earlier, different write created it.
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
//!   * a single intra-doc link to an in-crate type, on the field or on the
//!     field's type declaration. This last route is a contract statement --
//!     "this compare-and-set is on that row" -- and is the only one an author
//!     writes by hand.
//!
//! THE BLIND SPOT, stated because every key has one. This test sees only the
//! face of the law where NO VALUE EXISTS YET. It cannot see the other face: a
//! value that exists but is UNREADABLE under the authority the write itself
//! takes. `RecordTransferExecutionOutcomeWriteSetPartsV1::item_precondition` is
//! that shape -- the item row exists, and after this wave inserted a third
//! writer of it the only read returning it is gated on a reconciliation lease
//! the ordinary path must not hold. Deciding that requires ordering the
//! authority types, which this test does not do. It is handled by hand.
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
}

#[derive(Default)]
struct Model {
    structs: BTreeMap<String, Struct>,
    enums: BTreeMap<String, Enum>,
    /// `(method name, whole signature)` for every trait method in the wave.
    methods: Vec<(String, String)>,
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
                let mut cursor = index + 1;
                while cursor < lines.len() && lines[cursor] != "}" {
                    let body = lines[cursor];
                    let is_variant = body.len() > 4
                        && body.starts_with("    ")
                        && !body.starts_with("     ")
                        && body[4..].starts_with(|character: char| character.is_ascii_uppercase());
                    if is_variant {
                        variants.push(Vec::new());
                    } else if let Some(current) = variants.last_mut()
                        && let Some(rest) = body.strip_prefix("        ")
                        && let Some((field_name, field_type)) = member(&format!("    {rest}"))
                    {
                        current.push((field_name, field_type));
                    }
                    cursor += 1;
                }
                model.enums.insert(name, Enum { doc, variants });
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
                index = cursor + 1;
                pending.clear();
                continue;
            }

            pending.clear();
            index += 1;
        }
    }
    model
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
) -> Option<Resolution> {
    let ty = unwrap_container(&field.ty);

    if rows.contains(&ty) {
        return Some(Resolution {
            row: ty,
            route: "the field's type is the row",
        });
    }
    for suffix in VERSION_SUFFIXES {
        if let Some(stem) = ty.strip_suffix(suffix) {
            let candidate = format!("{stem}V1");
            if rows.contains(&candidate) {
                return Some(Resolution {
                    row: candidate,
                    route: "scalar newtype naming the row plus a version role",
                });
            }
        }
    }
    for suffix in ["PreconditionV1", "Precondition"] {
        let Some(stem) = ty.strip_suffix(suffix) else {
            continue;
        };
        let exact = format!("{stem}V1");
        if rows.contains(&exact) {
            return Some(Resolution {
                row: exact,
                route: "precondition type named for the row",
            });
        }
        let prefixed: Vec<&String> = rows.iter().filter(|row| row.starts_with(stem)).collect();
        if let [single] = prefixed.as_slice() {
            return Some(Resolution {
                row: (*single).clone(),
                route: "precondition type is the unique prefix of a row's name",
            });
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
                return Some(Resolution {
                    row: inner.into_iter().next().expect("one"),
                    route: "precondition struct whose members resolve to one row",
                });
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
            return Some(Resolution {
                row: hits.into_iter().next().expect("one"),
                route: "field name stems to the one row this write set carries",
            });
        }
    }
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
        let links = doc_links(&doc);
        let named: Vec<&String> = links
            .iter()
            .filter(|name| model.structs.contains_key(*name) || model.enums.contains_key(*name))
            .collect();
        if let [single] = named.as_slice() {
            return Some(Resolution {
                row: (*single).clone(),
                route: "an intra-doc link declaring the row this compare-and-set is on",
            });
        }
    }
    None
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
            if !model.structs.contains_key(&bare) || bare.ends_with("PreconditionV1") {
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
            match resolve(write_set, field, &rows, model) {
                Some(resolution) => attributed.entry(resolution.row).or_default().push((
                    (*write_set).clone(),
                    field.clone(),
                    resolution.route,
                )),
                None => unresolved.push(Unresolved {
                    write_set: (*write_set).clone(),
                    field: field.name.clone(),
                    ty: field.ty.clone(),
                    file: model.structs[*write_set].file.clone(),
                    line: field.line,
                }),
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

        if absence || unconditional || foreign {
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
