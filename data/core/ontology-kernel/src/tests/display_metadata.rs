//! Display metadata: attachable to all four definition kinds and to
//! properties, blank-field refusal at the engine boundary, and free
//! evolution — the first field the frozen-field law does not cover.

use crate::*;
use data_boundary_kernel::{DataClass, PrivacyDataClass};

fn internal() -> PrivacyDataClass {
    PrivacyDataClass::try_from(DataClass::InternalOnly).unwrap()
}

fn display(description: &str) -> DisplayMetadata {
    DisplayMetadata {
        description: Some(description.to_string()),
        ..DisplayMetadata::default()
    }
}

fn prop() -> EntityTypePropertyDefinition {
    EntityTypePropertyDefinition::new("name", PropertyTier::Scalar, internal(), true).unwrap()
}

fn entity(revision: u32) -> EntityTypeDefinition {
    EntityTypeDefinition::new(
        "ten_test",
        EntityTypeId::new("ety_account").unwrap(),
        "Account",
        vec![prop()],
        revision,
    )
    .unwrap()
}

fn engine_with_entity() -> OntologyEngine {
    let mut engine = OntologyEngine::default();
    engine.register_entity_type(entity(1)).unwrap();
    engine
}

fn link() -> LinkTypeDefinition {
    LinkTypeDefinition::new(
        "ten_test",
        LinkTypeId::new("lty_owns").unwrap(),
        EntityTypeId::new("ety_account").unwrap(),
        EntityTypeId::new("ety_account").unwrap(),
        LinkCardinality::ManyToMany,
        false,
    )
    .unwrap()
}

fn action() -> ActionTypeDefinition {
    ActionTypeDefinition::new(
        "ten_test",
        ActionTypeId::new("aty_close").unwrap(),
        EntityTypeId::new("ety_account").unwrap(),
        "console",
        AutonomyTier::T1Assist,
        "account.closed",
    )
    .unwrap()
}

#[test]
fn display_attaches_to_every_kind() {
    let mut engine = OntologyEngine::default();
    let mut with_prop_display = entity(1).with_display(display("an account"));
    with_prop_display.properties[0].display = Some(DisplayMetadata {
        display_name: Some("Name".to_string()),
        ..DisplayMetadata::default()
    });
    engine.register_entity_type(with_prop_display).unwrap();
    engine
        .register_link_type(link().with_display(display("ownership")))
        .unwrap();
    engine
        .register_action_type(action().with_display(display("close the account")))
        .unwrap();

    let stored = engine
        .entity_type("ten_test", &EntityTypeId::new("ety_account").unwrap())
        .unwrap();
    assert_eq!(
        stored.display.as_ref().unwrap().description.as_deref(),
        Some("an account")
    );
    assert_eq!(
        stored.properties[0]
            .display
            .as_ref()
            .unwrap()
            .display_name
            .as_deref(),
        Some("Name")
    );
}

#[test]
fn blank_display_fields_refused() {
    let mut engine = engine_with_entity();
    let blank = DisplayMetadata {
        icon: Some("  ".to_string()),
        ..DisplayMetadata::default()
    };
    assert_eq!(
        engine.register_link_type(link().with_display(blank.clone())),
        Err(OntologyEngineError::BlankDisplayField {
            field: "icon".to_string()
        })
    );
    assert_eq!(
        engine.register_action_type(action().with_display(blank.clone())),
        Err(OntologyEngineError::BlankDisplayField {
            field: "icon".to_string()
        })
    );
    let mut fresh = OntologyEngine::default();
    assert_eq!(
        fresh.register_entity_type(entity(1).with_display(blank)),
        Err(OntologyEngineError::BlankDisplayField {
            field: "icon".to_string()
        })
    );
}

#[test]
fn display_evolves_freely() {
    let mut engine = engine_with_entity();
    engine.register_link_type(link()).unwrap();
    engine.register_action_type(action()).unwrap();

    engine
        .evolve_entity_type(entity(2).with_display(display("now described")))
        .expect("entity display change must be additive");
    engine
        .evolve_link_type(
            link()
                .with_revision(2)
                .with_display(display("now described")),
        )
        .expect("link display change must evolve — the loosen moment");
    engine
        .evolve_action_type(
            action()
                .with_revision(2)
                .with_display(display("now described")),
        )
        .expect("action display change must evolve");

    // And a blank display is refused on the evolve paths too.
    let blank = DisplayMetadata {
        color: Some("".to_string()),
        ..DisplayMetadata::default()
    };
    assert_eq!(
        engine.evolve_link_type(link().with_revision(3).with_display(blank)),
        Err(OntologyEngineError::BlankDisplayField {
            field: "color".to_string()
        })
    );
}
