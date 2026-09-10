//! Shared fakes for the transform tests: a pack whose semantics are declared inline, and a
//! model whose declarations are handed in directly.

use std::collections::{BTreeMap, BTreeSet};

use port_engine_api::{
    Declaration, Digest, FailureConvention, LanguagePair, PackSemantics, PlanStep,
    PointerDisposition, RuleId, SourceModel, TargetIr, TransformPlan, TypeRef, UnitId,
};
use port_engine_rust_ir::RustIr;
use port_engine_transform::*;

#[derive(Default)]
pub struct Pack {
    /// rule → (construction, precondition, captures)
    pub rules: BTreeMap<&'static str, (&'static str, &'static str, Vec<String>)>,
    pub types: BTreeMap<String, String>,
    pub constructors: BTreeMap<String, String>,
    pub overrides: BTreeMap<String, BTreeMap<String, String>>,
    pub deferred: BTreeSet<String>,
    pub copies: BTreeSet<String>,
    pub casts: BTreeSet<String>,
    pub zeroes: BTreeMap<String, String>,
    pub trait_objects: BTreeMap<String, String>,
    pub failure: Option<FailureConvention>,
    pub functions: BTreeMap<String, String>,
    /// The declared trait-receiver decision. `None` means the pack made none, which is a refusal.
    pub receiver: Option<(String, String)>,
    pub dispositions: Vec<PointerDisposition>,
}

impl Pack {
    pub fn with_rule(
        mut self,
        id: &'static str,
        construction: &'static str,
        captures: &[&str],
    ) -> Self {
        self.rules.insert(
            id,
            (
                construction,
                PRECONDITION_UNIT_PRESENT,
                captures.iter().map(|c| (*c).to_owned()).collect(),
            ),
        );
        self
    }

    pub fn with_types(mut self, pairs: &[(&str, &str)]) -> Self {
        for (from, to) in pairs {
            self.types.insert((*from).to_owned(), (*to).to_owned());
        }
        self
    }

    /// Declare a composite constructor template, as a real pack does.
    pub fn with_constructor(mut self, kind: &str, template: &str) -> Self {
        self.constructors
            .insert(kind.to_owned(), template.to_owned());
        self
    }

    pub fn with_override(mut self, construction: &str, from: &str, to: &str) -> Self {
        self.overrides
            .entry(construction.to_owned())
            .or_default()
            .insert(from.to_owned(), to.to_owned());
        self
    }

    /// Declare an ownership rule, as a real pack must.
    pub fn with_disposition(
        mut self,
        id: &str,
        mutated: Option<bool>,
        escapes: Option<bool>,
        target: &str,
        receiver: Option<&str>,
    ) -> Self {
        self.dispositions.push(PointerDisposition {
            id: id.to_owned(),
            when_mutated: mutated,
            when_escapes: escapes,
            when_effect_unknown: Some(false),
            target: target.to_owned(),
            receiver: receiver.map(ToOwned::to_owned),
            reason: "fixture decision".to_owned(),
        });
        self
    }

    /// Declare the trait-receiver decision, as a real pack must.
    pub fn with_trait_receiver(mut self, mode: &str) -> Self {
        self.receiver = Some((mode.to_owned(), "fixture decision".to_owned()));
        self
    }

    /// Declare which source types copy in the target, as a real pack must.
    pub fn with_copy_types(mut self, names: &[&str]) -> Self {
        for name in names {
            self.copies.insert((*name).to_owned());
        }
        self
    }

    /// Declare a source function's target expression, as a real pack must.
    pub fn with_function(mut self, source: &str, template: &str) -> Self {
        self.functions
            .insert(source.to_owned(), template.to_owned());
        self
    }

    /// Declare the source's failure convention, as a real pack must.
    pub fn with_failure(mut self, source: &str, target: &str) -> Self {
        self.failure = Some(FailureConvention {
            source_type: source.to_owned(),
            target_type: target.to_owned(),
            absent: "nil".to_owned(),
        });
        self
    }

    /// Declare the target form a trait takes in one position, as a real pack must.
    pub fn with_trait_object(mut self, position: &str, form: &str) -> Self {
        self.trait_objects
            .insert(position.to_owned(), form.to_owned());
        self
    }

    /// Declare a source type's target zero value, as a real pack must.
    pub fn with_zero_value(mut self, source: &str, target: &str) -> Self {
        self.zeroes.insert(source.to_owned(), target.to_owned());
        self
    }

    pub fn with_deferred(mut self, kinds: &[&str]) -> Self {
        for kind in kinds {
            self.deferred.insert((*kind).to_owned());
        }
        self
    }
}

impl PackSemantics for Pack {
    fn construction(&self, rule: &RuleId) -> Option<&str> {
        self.rules.get(rule.0.as_str()).map(|(c, _, _)| *c)
    }
    fn precondition(&self, rule: &RuleId) -> Option<&str> {
        self.rules.get(rule.0.as_str()).map(|(_, p, _)| *p)
    }
    fn captures(&self, rule: &RuleId) -> Option<&[String]> {
        self.rules
            .get(rule.0.as_str())
            .map(|(_, _, c)| c.as_slice())
    }
    fn type_map(&self) -> &BTreeMap<String, String> {
        &self.types
    }
    fn type_constructors(&self) -> &BTreeMap<String, String> {
        &self.constructors
    }
    fn type_map_overrides(&self, construction: &str) -> Option<&BTreeMap<String, String>> {
        self.overrides.get(construction)
    }
    fn cast_types(&self) -> &BTreeSet<String> {
        &self.casts
    }
    fn copy_types(&self) -> &BTreeSet<String> {
        &self.copies
    }
    fn function_map(&self) -> &BTreeMap<String, String> {
        &self.functions
    }
    fn failure_convention(&self) -> Option<&FailureConvention> {
        self.failure.as_ref()
    }
    fn trait_object_forms(&self) -> &BTreeMap<String, String> {
        &self.trait_objects
    }
    fn zero_values(&self) -> &BTreeMap<String, String> {
        &self.zeroes
    }
    fn deferred_kinds(&self) -> &BTreeSet<String> {
        &self.deferred
    }
    fn pointer_dispositions(&self) -> &[PointerDisposition] {
        &self.dispositions
    }
    fn trait_receiver(&self) -> Option<(&str, &str)> {
        self.receiver
            .as_ref()
            .map(|(mode, reason)| (mode.as_str(), reason.as_str()))
    }
}

pub struct Model {
    pub units: Vec<UnitId>,
    pub declarations: BTreeMap<String, Vec<Declaration>>,
}

impl SourceModel for Model {
    fn language(&self) -> &str {
        "go"
    }
    fn snapshot_digest(&self) -> Digest {
        Digest("snap".into())
    }
    fn units(&self) -> Vec<UnitId> {
        self.units.clone()
    }
    fn declarations(&self, unit: &UnitId) -> Option<Vec<Declaration>> {
        self.declarations.get(&unit.0).cloned()
    }
}

/// A declaration whose type, when it has one, is a PRIMITIVE of that name. Tests that need a
/// named or composite type build the `TypeRef` themselves.
pub fn decl(kind: &str, name: &str, type_ref: &str) -> Declaration {
    Declaration {
        kind: kind.into(),
        name: name.into(),
        type_ref: if type_ref.is_empty() {
            TypeRef::default()
        } else {
            TypeRef::basic(type_ref)
        },
        flags: ["exported".to_owned()].into_iter().collect(),
        attrs: BTreeMap::new(),
        children: Vec::new(),
    }
}

pub fn child(kind: &str, name: &str, type_ref: &str) -> Declaration {
    let mut node = decl(kind, name, type_ref);
    node.flags.clear();
    node
}

pub fn model_with(declarations: Vec<Declaration>) -> Model {
    Model {
        units: vec![UnitId("u".into())],
        declarations: BTreeMap::from([("u".to_owned(), declarations)]),
    }
}

pub fn plan_with(rules: &[&str]) -> TransformPlan {
    TransformPlan {
        pair: LanguagePair {
            source: "go".into(),
            target: "rust".into(),
        },
        steps: rules
            .iter()
            .map(|rule| PlanStep {
                unit: UnitId("u".into()),
                rule: RuleId((*rule).to_owned()),
            })
            .collect(),
    }
}

pub fn rendered(ir: &RustIr) -> String {
    // Round-trip through the renderer the pipeline actually uses, so a construction emitting
    // text that syn cannot parse fails here rather than three stages later.
    let renderer = port_engine_rust_ir::RustRenderer::new();
    let emitted = renderer
        .render_rust_ir(ir)
        .expect("emitted Rust must render");
    emitted
        .values()
        .map(|bytes| String::from_utf8(bytes.clone()).expect("utf-8"))
        .collect::<Vec<_>>()
        .join("\n")
}
