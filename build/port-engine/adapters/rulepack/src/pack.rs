//! The loaded pack: validation on the way in, and the seam impls on the way out.

use std::collections::{BTreeMap, BTreeSet};

use port_engine_api::{
    Digest, FailureConvention, LanguagePair, PackSemantics, PointerDisposition, RuleId, RulePack,
    UnitId,
};
use port_engine_hash::digest_bytes;

use crate::error::RulepackError;
use crate::policy::{validate_dispositions, validate_policy};
use crate::rule::{DeferredKind, LoadedRule, TraitReceiver};
use crate::wire::RulepackDocument;
use crate::{CONFLICT_REFUSE, RULEPACK_GO_RUST_V1_JSON, RULEPACK_V0_JSON};

/// Loaded neutral rule pack implementing [`RulePack`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadedRulePack {
    pub(crate) pair: LanguagePair,
    pub(crate) digest: Digest,
    pub(crate) rules: Vec<RuleId>,
    pub(crate) loaded_rules: Vec<LoadedRule>,
    pub(crate) applies: BTreeMap<UnitId, Vec<RuleId>>,
    pub(crate) type_map: BTreeMap<String, String>,
    pub(crate) type_constructors: BTreeMap<String, String>,
    pub(crate) copy_types: BTreeSet<String>,
    pub(crate) cast_types: BTreeSet<String>,
    pub(crate) zero_values: BTreeMap<String, String>,
    pub(crate) trait_object_forms: BTreeMap<String, String>,
    pub(crate) failure_convention: Option<FailureConvention>,
    pub(crate) function_map: BTreeMap<String, String>,
    pub(crate) type_map_overrides: BTreeMap<String, BTreeMap<String, String>>,
    pub(crate) deferred_kinds: Vec<DeferredKind>,
    pub(crate) deferred_kind_set: BTreeSet<String>,
    pub(crate) trait_receiver: Option<TraitReceiver>,
    pub(crate) dispositions: Vec<PointerDisposition>,
}

impl LoadedRulePack {
    /// Load and validate the embedded v0 rulepack mirror (fixture-gated).
    ///
    /// # Errors
    /// [`RulepackError`] on parse, schema, fixture, selection, undeclared-apply, or pair refusal.
    pub fn load_embedded() -> Result<Self, RulepackError> {
        Self::load_from_str(RULEPACK_V0_JSON)
    }

    /// Load from an in-memory JSON string (test hook / future specs materializer input).
    ///
    /// # Errors
    /// [`RulepackError`] on parse, schema, fixture, selection, undeclared-apply, or pair refusal.
    pub fn load_from_str(json: &str) -> Result<Self, RulepackError> {
        let doc: RulepackDocument =
            serde_json::from_str(json).map_err(|err| RulepackError::Parse {
                detail: err.to_string(),
            })?;
        if doc.pair.source.is_empty() || doc.pair.target.is_empty() {
            return Err(RulepackError::Schema { field: "pair" });
        }
        let pair = LanguagePair {
            source: doc.pair.source,
            target: doc.pair.target,
        };
        // Fail closed on ambiguous pair before we ever plan.
        pair.slug().map_err(RulepackError::Pair)?;

        if doc.rules.is_empty() {
            return Err(RulepackError::Schema { field: "rules" });
        }
        let mut seen = BTreeSet::new();
        let mut rules = Vec::with_capacity(doc.rules.len());
        let mut loaded_rules = Vec::with_capacity(doc.rules.len());
        let mut previous_precedence: Option<i64> = None;
        for rule in doc.rules {
            if rule.id.is_empty() {
                return Err(RulepackError::Schema {
                    field: "rules[].id",
                });
            }
            if rule.version.is_empty() {
                return Err(RulepackError::Schema {
                    field: "rules[].version",
                });
            }
            if rule.precondition.is_empty() {
                return Err(RulepackError::Schema {
                    field: "rules[].precondition",
                });
            }
            if rule.construction.is_empty() {
                return Err(RulepackError::Schema {
                    field: "rules[].construction",
                });
            }
            if rule.selecting_fixtures.is_empty() {
                return Err(RulepackError::MissingSelectingFixture { rule: rule.id });
            }
            for fixture in &rule.selecting_fixtures {
                if fixture.id.is_empty() {
                    return Err(RulepackError::Schema {
                        field: "rules[].selecting_fixtures[].id",
                    });
                }
                if fixture.unit.is_empty() {
                    return Err(RulepackError::Schema {
                        field: "rules[].selecting_fixtures[].unit",
                    });
                }
            }
            if !seen.insert(rule.id.clone()) {
                return Err(RulepackError::Schema {
                    field: "rules(duplicate)",
                });
            }
            // Every field the wire shape carries must either drive behaviour or be refused.
            // These two carry no implementation, so a pack declaring them is told so rather than
            // loading green and receiving nothing.
            if !rule.required_diagnostics.is_empty() {
                return Err(RulepackError::UnimplementedSemantics {
                    rule: rule.id,
                    field: "required_diagnostics",
                });
            }
            if !rule.proof_obligations.is_empty() {
                return Err(RulepackError::UnimplementedSemantics {
                    rule: rule.id,
                    field: "proof_obligations",
                });
            }
            if rule.conflict != CONFLICT_REFUSE {
                return Err(RulepackError::UnknownConflictPolicy {
                    rule: rule.id,
                    policy: rule.conflict,
                });
            }
            // Declaration order IS the transform order — `port_engine_kernel::plan` refuses a
            // unit whose rules arrive out of declared position. `precedence` therefore has to
            // agree with it or the pack states an order that nothing obeys, and a reviewer
            // reading the precedences would be reading a fiction.
            if let Some(previous) = previous_precedence.filter(|p| rule.precedence <= *p) {
                return Err(RulepackError::PrecedenceDisagreesWithOrder {
                    rule: rule.id,
                    precedence: rule.precedence,
                    previous,
                });
            }
            previous_precedence = Some(rule.precedence);

            for capture in &rule.captures {
                if capture.is_empty() {
                    return Err(RulepackError::Schema {
                        field: "rules[].captures[]",
                    });
                }
            }

            let id = RuleId(rule.id);
            loaded_rules.push(LoadedRule {
                id: id.clone(),
                version: rule.version,
                precondition: rule.precondition,
                construction: rule.construction,
                captures: rule.captures,
                precedence: rule.precedence,
                conflict: rule.conflict,
                selecting_fixtures: rule.selecting_fixtures,
            });
            rules.push(id);
        }
        let declared: BTreeSet<&str> = rules.iter().map(|r| r.0.as_str()).collect();

        let mut applies = BTreeMap::new();
        for (unit, rule_ids) in doc.applies {
            if unit.is_empty() {
                return Err(RulepackError::Schema {
                    field: "applies.unit",
                });
            }
            let mut mapped = Vec::with_capacity(rule_ids.len());
            for rule in rule_ids {
                if !declared.contains(rule.as_str()) {
                    return Err(RulepackError::UndeclaredApply {
                        unit: unit.clone(),
                        rule,
                    });
                }
                mapped.push(RuleId(rule));
            }
            applies.insert(UnitId(unit), mapped);
        }

        for rule in &loaded_rules {
            if !rule
                .selecting_fixtures
                .iter()
                .any(|fixture| fixture.selects)
            {
                return Err(RulepackError::NoPositiveFixture {
                    rule: rule.id.0.clone(),
                    fixture_count: rule.selecting_fixtures.len(),
                });
            }
            for fixture in &rule.selecting_fixtures {
                let unit = UnitId(fixture.unit.clone());
                let actual = applies
                    .get(&unit)
                    .is_some_and(|selected| selected.contains(&rule.id));
                if actual != fixture.selects {
                    return Err(RulepackError::FixtureExpectationMismatch {
                        rule: rule.id.0.clone(),
                        fixture: fixture.id.clone(),
                        unit: fixture.unit.clone(),
                        expected: fixture.selects,
                        actual,
                    });
                }
            }
        }

        let deferred_kind_set = validate_policy(
            &doc.deferred_kinds,
            doc.trait_receiver.as_ref(),
            &loaded_rules,
        )?;

        // Digest the embedded bytes exactly — whitespace is part of the identity until a
        // canonicalizer lands with the forever specs/port-rules materializer.
        let digest = digest_bytes(json.as_bytes());
        Ok(Self {
            pair,
            digest,
            rules,
            loaded_rules,
            applies,
            type_map: doc.type_map,
            type_constructors: doc.type_constructors,
            copy_types: doc.copy_types,
            cast_types: doc.cast_types,
            zero_values: doc.zero_values,
            trait_object_forms: doc.trait_object_forms,
            function_map: doc.function_map,
            failure_convention: doc.failure_convention.map(|failure| FailureConvention {
                source_type: failure.source_type,
                target_type: failure.target_type,
                absent: failure.absent,
            }),
            type_map_overrides: doc.type_map_overrides,
            deferred_kinds: doc.deferred_kinds,
            deferred_kind_set,
            dispositions: validate_dispositions(&doc.pointer_dispositions)?,
            trait_receiver: doc.trait_receiver,
        })
    }

    /// Load and validate the embedded go→rust v1 pack.
    ///
    /// # Errors
    /// [`RulepackError`] on any of the same refusals as [`Self::load_embedded`].
    pub fn load_embedded_go_rust() -> Result<Self, RulepackError> {
        Self::load_from_str(RULEPACK_GO_RUST_V1_JSON)
    }

    /// The kinds this pack knowingly leaves untranslated, with their recorded reasons.
    #[must_use]
    pub fn deferred(&self) -> &[DeferredKind] {
        &self.deferred_kinds
    }

    /// Borrow the language pair.
    #[must_use]
    pub fn language_pair(&self) -> &LanguagePair {
        &self.pair
    }

    /// Loaded rule records in pack order (each has a validated positive fixture).
    #[must_use]
    pub fn loaded_rules(&self) -> &[LoadedRule] {
        &self.loaded_rules
    }

    /// Total positive selecting fixtures across every loaded rule.
    #[must_use]
    pub fn selecting_fixture_count(&self) -> usize {
        self.loaded_rules
            .iter()
            .map(|r| {
                r.selecting_fixtures
                    .iter()
                    .filter(|fixture| fixture.selects)
                    .count()
            })
            .sum()
    }

    /// Look up a loaded rule by id.
    #[must_use]
    pub fn rule(&self, id: &RuleId) -> Option<&LoadedRule> {
        self.loaded_rules.iter().find(|r| &r.id == id)
    }
}
