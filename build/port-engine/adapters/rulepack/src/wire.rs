//! CLOSED serde wire shapes.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use crate::CONFLICT_REFUSE;
use crate::rule::{DeferredKind, DispositionRule, SelectingFixture, TraitReceiver};

fn default_conflict() -> String {
    CONFLICT_REFUSE.to_owned()
}

/// CLOSED wire shape. An unknown key is a refusal, not a shrug: `type_map_override` for
/// `type_map_overrides` would otherwise parse clean, override nothing, and leave the pack author
/// looking at a green load and the wrong emitted types. `_comment` is declared so prose can live
/// beside the data it explains without punching a hole in the closure.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RulepackDocument {
    #[serde(default, rename = "_comment")]
    pub(crate) _comment: serde_json::Value,
    pub(crate) pair: PairFields,
    #[serde(default)]
    pub(crate) type_map: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) type_constructors: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) copy_types: BTreeSet<String>,
    #[serde(default)]
    pub(crate) cast_types: BTreeSet<String>,
    #[serde(default)]
    pub(crate) zero_values: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) trait_object_forms: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) failure_convention: Option<FailureDoc>,
    #[serde(default)]
    pub(crate) function_map: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) type_map_overrides: BTreeMap<String, BTreeMap<String, String>>,
    #[serde(default)]
    pub(crate) deferred_kinds: Vec<DeferredKind>,
    #[serde(default)]
    pub(crate) trait_receiver: Option<TraitReceiver>,
    #[serde(default)]
    pub(crate) pointer_dispositions: Vec<DispositionRule>,
    pub(crate) rules: Vec<RuleDocument>,
    pub(crate) applies: BTreeMap<String, Vec<String>>,
}

/// The wire shape of the failure convention.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FailureDoc {
    pub(crate) source_type: String,
    pub(crate) target_type: String,
    pub(crate) absent: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PairFields {
    pub(crate) source: String,
    pub(crate) target: String,
}

/// Wire shape for one rule. Selection gating requires `id` / `version` / `selecting_fixtures`;
/// transform apply also requires non-empty `precondition` + `construction`. Closed, for the same
/// reason as [`RulepackDocument`].
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RuleDocument {
    #[serde(default, rename = "_comment")]
    pub(crate) _comment: serde_json::Value,
    pub(crate) id: String,
    pub(crate) version: String,
    #[serde(default)]
    pub(crate) precondition: String,
    #[serde(default)]
    pub(crate) captures: Vec<String>,
    #[serde(default)]
    pub(crate) construction: String,
    #[serde(default)]
    pub(crate) precedence: i64,
    // Defaults to the only implemented policy rather than to the empty string. The engine refuses
    // a conflict unconditionally — the kernel has no other code path — so an omitted policy and a
    // stated `refuse` describe the same behaviour, while a stated ANYTHING ELSE describes
    // behaviour that does not exist and is refused below.
    #[serde(default = "default_conflict")]
    pub(crate) conflict: String,
    // Declared, and refused while unimplemented — see `RulepackError::UnimplementedSemantics`.
    #[serde(default)]
    pub(crate) required_diagnostics: Vec<String>,
    #[serde(default)]
    pub(crate) proof_obligations: Vec<String>,
    #[serde(default)]
    pub(crate) selecting_fixtures: Vec<SelectingFixture>,
}
