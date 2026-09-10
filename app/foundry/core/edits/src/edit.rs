//! The edit vocabulary: what an Action may do to its object.
//!
//! One envelope = one object stays law: every edit targets the envelope's
//! `object_ref`. [`OntologyEdit::CreateLink`] is owned by the FROM
//! endpoint — the envelope's object — and carries only the target; the
//! inbound side is derived index, never a peer edit.

use crate::property::WireProperty;

const ENTITY_TYPE_ID_PREFIX: &str = "ety_";
const LINK_TYPE_ID_PREFIX: &str = "lty_";
const TARGET_ENTITY_ID_PREFIX: &str = "ent_";

/// Why an edit or edit set was refused at construction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EditError {
    /// Not `ENTITY_TYPE_ID_PREFIX`-prefixed, trimmed, and non-blank.
    InvalidEntityTypeId,
    /// Not `LINK_TYPE_ID_PREFIX`-prefixed, trimmed, and non-blank.
    InvalidLinkTypeId,
    /// Not `TARGET_ENTITY_ID_PREFIX`-prefixed, trimmed, and non-blank.
    InvalidTargetEntityId,
    /// An edit set must carry at least one edit.
    EmptyEditSet,
}

/// The u8 wire tag of every edit kind, live and reserved alike —
/// byte-frozen from birth. The reserved kinds have NO [`OntologyEdit`]
/// variant: nothing may enter the log that the fold cannot apply, so a
/// reserved kind is representable only as a tag. They un-reserve when the
/// kernel grows removal operations, as an additive law change.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EditTag {
    CreateObject,
    UpsertProperties,
    /// RESERVED.
    UnsetProperties,
    /// RESERVED.
    DeleteObject,
    CreateLink,
    /// RESERVED.
    DeleteLink,
}

impl EditTag {
    pub const fn tag(self) -> u8 {
        match self {
            Self::CreateObject => 0,
            Self::UpsertProperties => 1,
            Self::UnsetProperties => 2,
            Self::DeleteObject => 3,
            Self::CreateLink => 4,
            Self::DeleteLink => 5,
        }
    }

    pub const fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            0 => Some(Self::CreateObject),
            1 => Some(Self::UpsertProperties),
            2 => Some(Self::UnsetProperties),
            3 => Some(Self::DeleteObject),
            4 => Some(Self::CreateLink),
            5 => Some(Self::DeleteLink),
            _ => None,
        }
    }

    /// A reserved tag is byte-frozen but writer-refused: no edit carries it
    /// until the kernel removal lane lands.
    pub const fn is_reserved(self) -> bool {
        matches!(
            self,
            Self::UnsetProperties | Self::DeleteObject | Self::DeleteLink
        )
    }
}

/// One edit to the envelope's object; see [`EditTag`] for the reserved
/// kinds.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OntologyEdit {
    CreateObject {
        entity_type: String, // data_class: INTERNAL_ONLY
        properties: Vec<WireProperty>,
    },
    UpsertProperties {
        set: Vec<WireProperty>,
    },
    CreateLink {
        link_type: String,    // data_class: INTERNAL_ONLY
        to_entity_id: String, // data_class: INTERNAL_ONLY
    },
}

impl OntologyEdit {
    pub fn create_object(
        entity_type: impl Into<String>,
        properties: Vec<WireProperty>,
    ) -> Result<Self, EditError> {
        let entity_type = entity_type.into();
        validate_prefixed(
            &entity_type,
            ENTITY_TYPE_ID_PREFIX,
            EditError::InvalidEntityTypeId,
        )?;
        Ok(Self::CreateObject {
            entity_type,
            properties,
        })
    }

    pub fn upsert_properties(set: Vec<WireProperty>) -> Result<Self, EditError> {
        Ok(Self::UpsertProperties { set })
    }

    pub fn create_link(
        link_type: impl Into<String>,
        to_entity_id: impl Into<String>,
    ) -> Result<Self, EditError> {
        let link_type = link_type.into();
        validate_prefixed(
            &link_type,
            LINK_TYPE_ID_PREFIX,
            EditError::InvalidLinkTypeId,
        )?;
        let to_entity_id = to_entity_id.into();
        validate_prefixed(
            &to_entity_id,
            TARGET_ENTITY_ID_PREFIX,
            EditError::InvalidTargetEntityId,
        )?;
        Ok(Self::CreateLink {
            link_type,
            to_entity_id,
        })
    }

    pub const fn tag(&self) -> EditTag {
        match self {
            Self::CreateObject { .. } => EditTag::CreateObject,
            Self::UpsertProperties { .. } => EditTag::UpsertProperties,
            Self::CreateLink { .. } => EditTag::CreateLink,
        }
    }
}

/// The ordered, non-empty edits of one Action.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditSet {
    edits: Vec<OntologyEdit>,
}

impl EditSet {
    pub fn new(edits: Vec<OntologyEdit>) -> Result<Self, EditError> {
        if edits.is_empty() {
            return Err(EditError::EmptyEditSet);
        }
        Ok(Self { edits })
    }

    pub fn edits(&self) -> &[OntologyEdit] {
        &self.edits
    }
}

fn validate_prefixed(value: &str, prefix: &str, refusal: EditError) -> Result<(), EditError> {
    if value.trim() != value || !value.starts_with(prefix) || value.len() == prefix.len() {
        return Err(refusal);
    }
    Ok(())
}
