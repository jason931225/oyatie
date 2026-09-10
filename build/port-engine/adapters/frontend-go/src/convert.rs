//! Wire declarations → neutral [`Declaration`] nodes, with the vocabulary checks.

use std::collections::{BTreeMap, BTreeSet};

use port_engine_api::{Declaration, TypeRef};

use crate::error::SnapshotError;
use crate::vocabulary::{
    KNOWN_ATTR_KEYS, KNOWN_DECLARATION_KINDS, KNOWN_FLAGS, KNOWN_MEMBER_KINDS, KNOWN_TYPE_KINDS,
    NAMESPACE_KINDS,
};
use crate::wire::DeclarationEntry;
use crate::wire::TypeEntry;

pub(crate) fn convert_declarations(
    unit_id: &str,
    entries: &[DeclarationEntry],
) -> Result<Vec<Declaration>, SnapshotError> {
    convert_level(unit_id, entries, KNOWN_DECLARATION_KINDS, Namespace::Yes)
}

/// Whether a level is one of [`crate::vocabulary::NAMESPACE_KINDS`].
#[derive(Clone, Copy, Eq, PartialEq)]
enum Namespace {
    Yes,
    No,
}

/// Convert one type node, checking every kind against the closed vocabulary.
///
/// An absent type is the DEFAULT node rather than a missing one: a declaration with no type is a
/// real shape (a function, a struct), and making the caller distinguish `None` from "empty" would
/// push a null check into every construction for no gain.
fn convert_type(unit_id: &str, entry: Option<&TypeEntry>) -> Result<TypeRef, SnapshotError> {
    let Some(entry) = entry else {
        return Ok(TypeRef::default());
    };
    if !KNOWN_TYPE_KINDS.contains(&entry.kind.as_str()) {
        return Err(SnapshotError::UnknownTypeKind {
            unit_id: unit_id.to_owned(),
            actual: entry.kind.clone(),
        });
    }
    if entry.name.contains('\0') || entry.package.contains('\0') {
        return Err(SnapshotError::Schema {
            field: "packages.declarations.type",
        });
    }
    let mut args = Vec::with_capacity(entry.args.len());
    for arg in &entry.args {
        args.push(convert_type(unit_id, Some(arg))?);
    }
    Ok(TypeRef {
        kind: entry.kind.clone(),
        name: entry.name.clone(),
        package: entry.package.clone(),
        args,
    })
}

fn convert_level(
    unit_id: &str,
    entries: &[DeclarationEntry],
    allowed_kinds: &[&str],
    namespace: Namespace,
) -> Result<Vec<Declaration>, SnapshotError> {
    let mut named = BTreeSet::new();
    let mut out = Vec::with_capacity(entries.len());

    for entry in entries {
        if !allowed_kinds.contains(&entry.kind.as_str()) {
            return Err(SnapshotError::UnknownDeclarationKind {
                unit_id: unit_id.to_owned(),
                actual: entry.kind.clone(),
            });
        }
        if entry.name.contains('\0') {
            return Err(SnapshotError::Schema {
                field: "packages.declarations",
            });
        }
        if namespace == Namespace::Yes
            && !entry.name.is_empty()
            && entry.name != "_"
            && !named.insert(entry.name.clone())
        {
            return Err(SnapshotError::DuplicateDeclaration {
                unit_id: unit_id.to_owned(),
                name: entry.name.clone(),
            });
        }

        let mut flags = BTreeSet::new();
        for flag in &entry.flags {
            if !KNOWN_FLAGS.contains(&flag.as_str()) {
                return Err(SnapshotError::UnknownFlag {
                    unit_id: unit_id.to_owned(),
                    actual: flag.clone(),
                });
            }
            flags.insert(flag.clone());
        }

        let mut attrs = BTreeMap::new();
        for (key, value) in &entry.attrs {
            if !KNOWN_ATTR_KEYS.contains(&key.as_str()) {
                return Err(SnapshotError::UnknownAttr {
                    unit_id: unit_id.to_owned(),
                    actual: key.clone(),
                });
            }
            attrs.insert(key.clone(), value.clone());
        }

        out.push(Declaration {
            kind: entry.kind.clone(),
            name: entry.name.clone(),
            type_ref: convert_type(unit_id, entry.type_ref.as_ref())?,
            flags,
            attrs,
            children: convert_level(
                unit_id,
                &entry.children,
                KNOWN_MEMBER_KINDS,
                if NAMESPACE_KINDS.contains(&entry.kind.as_str()) {
                    Namespace::Yes
                } else {
                    Namespace::No
                },
            )?,
        });
    }

    Ok(out)
}
