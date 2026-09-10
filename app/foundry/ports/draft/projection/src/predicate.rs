//! Property predicates: the indexed read surface. `Equals` is exact
//! typed equality; `Range` is inclusive and KIND-scoped — comparison
//! only means something within one value kind (a `StorageClass` groups
//! several kinds, so class scoping was too coarse: an Integer range
//! over a Boolean is incoherent, not empty). A mixed-kind or unrankable
//! (array/struct) range is refused at construction, and a
//! kind-mismatched stored value is refused at match time — schema drift
//! is loud, never a silent false. An absent property is no match.

use data_ontology_kernel::{ObjectEntity, PropertyValue};

use crate::store::ProjectionStoreError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PredicateError {
    BlankProperty,
    MixedValueKinds,
    /// Arrays and structs have no meaningful order; a range over them
    /// would pin the derived structural order into every adapter.
    UnrankedValueKind,
    /// `from` sorts after `to`; an inverted range is ambiguity, not an
    /// empty result.
    InvertedRange,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PropertyPredicate {
    Equals {
        property: String,     // data_class: INTERNAL_ONLY
        value: PropertyValue, // data_class: PII_IDENTIFYING
    },
    Range {
        property: String,    // data_class: INTERNAL_ONLY
        from: PropertyValue, // data_class: PII_IDENTIFYING
        to: PropertyValue,   // data_class: PII_IDENTIFYING
    },
}

impl PropertyPredicate {
    pub fn equals(
        property: impl Into<String>,
        value: PropertyValue,
    ) -> Result<Self, PredicateError> {
        let property = non_blank(property.into())?;
        Ok(Self::Equals { property, value })
    }

    pub fn range(
        property: impl Into<String>,
        from: PropertyValue,
        to: PropertyValue,
    ) -> Result<Self, PredicateError> {
        let property = non_blank(property.into())?;
        if from.type_label() != to.type_label() {
            return Err(PredicateError::MixedValueKinds);
        }
        if matches!(from, PropertyValue::Array(_) | PropertyValue::Struct(_)) {
            return Err(PredicateError::UnrankedValueKind);
        }
        if from > to {
            return Err(PredicateError::InvertedRange);
        }
        Ok(Self::Range { property, from, to })
    }

    /// The (property, kind label) a `Range` compares under; `None` for
    /// `Equals`. Stores and adapters use it to refuse kind drift
    /// window-independently — a cursor or page limit never hides it.
    pub fn range_kind(&self) -> Option<(&str, &'static str)> {
        match self {
            Self::Equals { .. } => None,
            Self::Range { property, from, .. } => Some((property.as_str(), from.type_label())),
        }
    }

    /// Whether `entity` matches. `Ok(false)` is a real answer; a
    /// kind-mismatched stored value under a `Range` is an error. Public
    /// because adapters use THIS as the final word on every candidate —
    /// their indexes accelerate, they never decide.
    pub fn matches(&self, entity: &ObjectEntity) -> Result<bool, ProjectionStoreError> {
        match self {
            Self::Equals { property, value } => Ok(entity
                .properties
                .get(property)
                .is_some_and(|stored| &stored.value.value == value)),
            Self::Range { property, from, to } => {
                let Some(stored) = entity.properties.get(property) else {
                    return Ok(false);
                };
                let stored = &stored.value.value;
                if stored.type_label() != from.type_label() {
                    return Err(ProjectionStoreError::KindMismatch {
                        property: property.clone(),
                    });
                }
                Ok(stored >= from && stored <= to)
            }
        }
    }
}

fn non_blank(property: String) -> Result<String, PredicateError> {
    if property.trim().is_empty() || property.trim() != property {
        return Err(PredicateError::BlankProperty);
    }
    Ok(property)
}
