//! Ontology kernel: data-classed entities, property-tier semantics, and pillar isolation.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod pillar;
pub use pillar::{OntologyPillar, UnknownPillarLabel};

mod action_parameters;
mod definitions;
mod display;
mod engine;
mod error;
mod object_graph;
mod property;
mod revisioning;
mod value;
mod value_type;

#[cfg(test)]
mod tests;

pub use action_parameters::ActionParameterDefinition;
pub use definitions::{
    ActionInvocationReceipt, ActionInvocationRequest, ActionPolicyDecision, ActionTypeDefinition,
    ActionTypeId, AutonomyTier, EntityTypeDefinition, EntityTypeId, EntityTypePropertyDefinition,
    LinkCardinality, LinkTypeDefinition, LinkTypeId,
};
pub use display::DisplayMetadata;
pub use engine::{LinkInstanceOutcome, OntologyEngine};
pub use error::OntologyEngineError;
pub use object_graph::{
    ObjectEntity, ObjectEntityUpsertOutcome, ObjectGraph, ObjectGraphError,
    ObjectPropertyUpsertOutcome,
};
pub use property::{ObjectProperty, PropertyTier};
pub use value::{CalendarDate, FiniteDouble, PropertyValue, StorageClass, ValueTypeError};
pub use value_type::{
    MAX_VALUE_TYPE_DEPTH, ScalarType, StructFieldDeclaration, StructSchema, ValueTypeDeclaration,
    ValueTypeViolation,
};
