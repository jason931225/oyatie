//! The namespace configuration: what a relation *means* on an object type.
//!
//! `UsersetRewrite` describes a shape but binds to nothing. Without this table
//! a rewrite is inert data — there is no way to ask what `document#viewer`
//! expands to. This is the binding, and it is deny-by-omission: a relation
//! with no entry grants nothing rather than falling back to direct tuples.

use std::collections::{BTreeMap, BTreeSet, btree_map::Entry};

use policy_cedar_domain::rebac::{RebacRelation, UsersetRewrite};

use crate::error::ExpansionError;
use crate::stratify::assert_stratified;
use crate::{NamespaceCompileError, compile::check_references};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct RelationKey {
    pub(crate) object_type: String,
    pub(crate) relation: String,
}

impl RelationKey {
    pub(crate) fn new(object_type: impl Into<String>, relation: &RebacRelation) -> Self {
        Self {
            object_type: object_type.into(),
            relation: relation.as_str().to_owned(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NamespaceConfig {
    relations: BTreeMap<RelationKey, UsersetRewrite>,
}

impl NamespaceConfig {
    /// Compile complete fragments without implicit replacement. Unlike
    /// [`Self::define`], even identical duplicate definitions are refused.
    /// Forward references are resolved after all definitions are collected.
    ///
    /// Same-type computed relations and tupleset source relations must exist.
    /// Tuple target types are not declared by the rewrite vocabulary, so their
    /// computed relations remain runtime checks, not a compilation guarantee.
    ///
    /// # Errors
    /// Duplicate definitions, unknown local references, empty composites, or
    /// existing model stratification failures. Duplicates take precedence and
    /// are selected in key order; other checks use key order then authored
    /// left-to-right rewrite order, before stratification.
    pub fn compile(
        definitions: impl IntoIterator<Item = (String, RebacRelation, UsersetRewrite)>,
    ) -> Result<ValidatedNamespace, NamespaceCompileError> {
        let mut relations = BTreeMap::new();
        let mut duplicates = BTreeSet::new();
        for (object_type, relation, rewrite) in definitions {
            match relations.entry(RelationKey::new(object_type, &relation)) {
                Entry::Vacant(entry) => {
                    entry.insert(rewrite);
                }
                Entry::Occupied(entry) => {
                    duplicates.insert(entry.key().clone());
                }
            }
        }
        if let Some(key) = duplicates.into_iter().next() {
            return Err(NamespaceCompileError::DuplicateRelation {
                object_type: key.object_type,
                relation: key.relation,
            });
        }
        check_references(&relations)?;
        Self { relations }
            .validated()
            .map_err(NamespaceCompileError::Model)
    }

    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Define `object_type#relation`. A later definition replaces an earlier
    /// one, so a caller assembling a config from fragments gets last-wins
    /// rather than a silently merged tree.
    #[must_use]
    pub fn define(
        mut self,
        object_type: impl Into<String>,
        relation: &RebacRelation,
        rewrite: UsersetRewrite,
    ) -> Self {
        self.relations
            .insert(RelationKey::new(object_type, relation), rewrite);
        self
    }

    /// The rewrite for `object_type#relation`, or [`ExpansionError::UndefinedRelation`].
    ///
    /// # Errors
    /// An unconfigured relation is a denial, not a direct-tuple fallback:
    /// falling back would make a typo in the config grant exactly the direct
    /// access the config meant to constrain.
    pub fn rewrite(
        &self,
        object_type: &str,
        relation: &RebacRelation,
    ) -> Result<&UsersetRewrite, ExpansionError> {
        self.relations
            .get(&RelationKey::new(object_type, relation))
            .ok_or_else(|| ExpansionError::UndefinedRelation {
                object_type: object_type.to_owned(),
                relation: relation.as_str().to_owned(),
            })
    }

    /// Check the model and hand back the only form an `Expander` accepts.
    ///
    /// # Errors
    /// [`ExpansionError::NonStratified`] when a relation reaches itself
    /// through a `Difference` subtraction. Refusing here rather than at check
    /// time is deliberate: the failure belongs to whoever wrote the model, and
    /// a type that cannot be built wrong cannot be evaluated wrong.
    pub fn validated(self) -> Result<ValidatedNamespace, ExpansionError> {
        assert_stratified(&self.relations)?;
        Ok(ValidatedNamespace(self))
    }

    #[must_use]
    pub fn is_defined(&self, object_type: &str, relation: &RebacRelation) -> bool {
        self.relations
            .contains_key(&RelationKey::new(object_type, relation))
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.relations.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.relations.is_empty()
    }
}

/// A [`NamespaceConfig`] that has passed stratification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedNamespace(NamespaceConfig);

impl ValidatedNamespace {
    /// The rewrite for `object_type#relation`.
    ///
    /// # Errors
    /// [`ExpansionError::UndefinedRelation`] when none is defined.
    pub fn rewrite(
        &self,
        object_type: &str,
        relation: &RebacRelation,
    ) -> Result<&UsersetRewrite, ExpansionError> {
        self.0.rewrite(object_type, relation)
    }
}
