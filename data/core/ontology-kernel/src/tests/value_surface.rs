//! Pins: the tier→typing coverage of V1 is frozen verbatim (any
//! widening must be an explicit reviewed diff here), and the legacy bridge
//! constructors are pinned byte-identical to the pre-model contract.

use crate::*;
use data_boundary_kernel::{DataClass, PrivacyDataClass};

fn internal() -> PrivacyDataClass {
    PrivacyDataClass::try_from(DataClass::InternalOnly).unwrap()
}

fn register(properties: Vec<EntityTypePropertyDefinition>) -> Result<(), OntologyEngineError> {
    OntologyEngine::default()
        .register_entity_type(
            EntityTypeDefinition::new(
                "ten_test",
                EntityTypeId::new("ety_frozen").unwrap(),
                "Frozen",
                properties,
                1,
            )
            .unwrap(),
        )
        .map(|_| ())
}

#[test]
fn tier_typing_coverage_is_frozen() {
    let declaration_for = |tier: PropertyTier| -> Option<ValueTypeDeclaration> {
        match tier {
            PropertyTier::Scalar => Some(ValueTypeDeclaration::Scalar(ScalarType::String)),
            PropertyTier::Vector => Some(ValueTypeDeclaration::Array {
                element: Box::new(ValueTypeDeclaration::Scalar(ScalarType::Integer)),
            }),
            PropertyTier::Struct => Some(ValueTypeDeclaration::Struct(StructSchema {
                fields: vec![StructFieldDeclaration {
                    name: "field".into(),
                    value_type: ValueTypeDeclaration::Scalar(ScalarType::Boolean),
                    required: true,
                }],
            })),
            PropertyTier::Timeseries | PropertyTier::Geo | PropertyTier::Ciphertext => None,
        }
    };

    for tier in PropertyTier::all_tiers() {
        match declaration_for(tier) {
            // Covered tier: the declaration's projection equals the tier and
            // registration admits it.
            Some(declaration) => {
                assert_eq!(declaration.tier(), tier, "projection for {tier:?}");
                let typed =
                    EntityTypePropertyDefinition::typed("p", declaration, internal(), false)
                        .unwrap();
                assert_eq!(typed.tier, tier);
                register(vec![typed]).unwrap();
            }
            // Exotic tier: ANY declaration on it is rejected at registration.
            None => {
                let mut property =
                    EntityTypePropertyDefinition::new("p", tier, internal(), false).unwrap();
                property.value_type = Some(ValueTypeDeclaration::Scalar(ScalarType::String));
                assert_eq!(
                    register(vec![property]),
                    Err(OntologyEngineError::ValueTypeTierMismatch { name: "p".into() }),
                    "exotic tier {tier:?} must stay untyped in V1",
                );
            }
        }
    }
}

#[test]
fn bridge_constructors_pinned_byte_identical() {
    let by_new = ObjectProperty::new(
        "note".into(),
        "free text".into(),
        PropertyTier::Scalar,
        internal(),
    );
    let by_privacy = ObjectProperty::new_with_privacy_data_class(
        "note".into(),
        "free text".into(),
        PropertyTier::Scalar,
        internal(),
    );
    let by_legacy = ObjectProperty::try_from_legacy_data_class(
        "note".into(),
        "free text".into(),
        PropertyTier::Scalar,
        DataClass::InternalOnly,
    )
    .unwrap();

    for property in [&by_new, &by_privacy, &by_legacy] {
        assert_eq!(
            property.value.value,
            PropertyValue::String("free text".into())
        );
        assert_eq!(property.value.value.as_str(), Some("free text"));
        assert_eq!(property.tier, PropertyTier::Scalar);
    }
    assert_eq!(by_new, by_privacy);
    assert_eq!(by_new, by_legacy);
}
