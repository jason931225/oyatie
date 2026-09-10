use serde::{Deserialize, Serialize};

use super::RebacTupleValidationError;
use super::tuple::RebacRelation;

/// Zanzibar/OpenFGA-style userset rewrite tree.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum UsersetRewrite {
    This,
    ComputedUserset {
        relation: RebacRelation, // data_class: INTERNAL_ONLY
    },
    TupleToUserset {
        tupleset_relation: RebacRelation, // data_class: INTERNAL_ONLY
        computed_userset_relation: RebacRelation, // data_class: INTERNAL_ONLY
    },
    Union {
        children: Vec<UsersetRewrite>, // data_class: INTERNAL_ONLY
    },
    Intersection {
        children: Vec<UsersetRewrite>, // data_class: INTERNAL_ONLY
    },
    Difference {
        base: Box<UsersetRewrite>,     // data_class: INTERNAL_ONLY
        subtract: Box<UsersetRewrite>, // data_class: INTERNAL_ONLY
    },
}

impl UsersetRewrite {
    #[must_use]
    pub fn this() -> Self {
        Self::This
    }

    #[must_use]
    pub fn computed_userset(relation: RebacRelation) -> Self {
        Self::ComputedUserset { relation }
    }

    #[must_use]
    pub fn tuple_to_userset(
        tupleset_relation: RebacRelation,
        computed_userset_relation: RebacRelation,
    ) -> Self {
        Self::TupleToUserset {
            tupleset_relation,
            computed_userset_relation,
        }
    }

    pub fn union(children: Vec<Self>) -> Result<Self, RebacTupleValidationError> {
        if children.is_empty() {
            return Err(RebacTupleValidationError::EmptyRewrite { kind: "union" });
        }
        let rewrite = Self::Union { children };
        rewrite.validate()?;
        Ok(rewrite)
    }

    pub fn intersection(children: Vec<Self>) -> Result<Self, RebacTupleValidationError> {
        if children.is_empty() {
            return Err(RebacTupleValidationError::EmptyRewrite {
                kind: "intersection",
            });
        }
        let rewrite = Self::Intersection { children };
        rewrite.validate()?;
        Ok(rewrite)
    }

    #[must_use]
    pub fn difference(base: Self, subtract: Self) -> Self {
        Self::Difference {
            base: Box::new(base),
            subtract: Box::new(subtract),
        }
    }

    pub fn validate(&self) -> Result<(), RebacTupleValidationError> {
        match self {
            Self::This | Self::ComputedUserset { .. } | Self::TupleToUserset { .. } => Ok(()),
            Self::Union { children } => validate_children("union", children),
            Self::Intersection { children } => validate_children("intersection", children),
            Self::Difference { base, subtract } => {
                base.validate()?;
                subtract.validate()
            }
        }
    }
}

fn validate_children(
    kind: &'static str,
    children: &[UsersetRewrite],
) -> Result<(), RebacTupleValidationError> {
    if children.is_empty() {
        return Err(RebacTupleValidationError::EmptyRewrite { kind });
    }
    for child in children {
        child.validate()?;
    }
    Ok(())
}
