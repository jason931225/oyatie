//! The edit vocabulary and wire-value pins: tag numbering frozen verbatim,
//! reserved kinds unconstructible, fail-closed construction throughout.

use std::collections::BTreeMap;

use foundry_edits::{
    EditError, EditSet, EditTag, OntologyEdit, WireDataClass, WireDate, WireDouble, WireProperty,
    WirePropertyError, WireTier, WireValue, WireValueError,
};

fn prop(name: &str) -> WireProperty {
    WireProperty::new(
        name,
        WireTier::Scalar,
        WireDataClass::InternalOnly,
        WireValue::String("v".into()),
    )
    .unwrap()
}

#[test]
fn edit_tag_numbering_is_frozen() {
    let expected = [
        (EditTag::CreateObject, 0, false),
        (EditTag::UpsertProperties, 1, false),
        (EditTag::UnsetProperties, 2, true),
        (EditTag::DeleteObject, 3, true),
        (EditTag::CreateLink, 4, false),
        (EditTag::DeleteLink, 5, true),
    ];
    for (tag, byte, reserved) in expected {
        assert_eq!(tag.tag(), byte);
        assert_eq!(EditTag::from_tag(byte), Some(tag));
        assert_eq!(tag.is_reserved(), reserved);
    }
    assert_eq!(EditTag::from_tag(6), None);
    assert_eq!(EditTag::from_tag(u8::MAX), None);
}

#[test]
fn live_edits_report_their_tags() {
    let create = OntologyEdit::create_object("ety_reading", vec![prop("name")]).unwrap();
    assert_eq!(create.tag(), EditTag::CreateObject);
    let upsert = OntologyEdit::upsert_properties(vec![prop("name")]).unwrap();
    assert_eq!(upsert.tag(), EditTag::UpsertProperties);
    let link = OntologyEdit::create_link("lty_measures", "ent_target_1").unwrap();
    assert_eq!(link.tag(), EditTag::CreateLink);
}

#[test]
fn create_object_requires_ety_prefixed_type() {
    for bad in ["", "  ", "reading", "ent_reading", " ety_reading"] {
        assert_eq!(
            OntologyEdit::create_object(bad, vec![prop("name")]),
            Err(EditError::InvalidEntityTypeId),
            "entity type {bad:?} must be refused",
        );
    }
}

#[test]
fn create_link_requires_prefixed_ids() {
    for bad in ["", "  ", "measures", "ety_measures", " lty_measures"] {
        assert_eq!(
            OntologyEdit::create_link(bad, "ent_target_1"),
            Err(EditError::InvalidLinkTypeId),
            "link type {bad:?} must be refused",
        );
    }
    for bad in ["", "  ", "target", "ety_target", " ent_target_1"] {
        assert_eq!(
            OntologyEdit::create_link("lty_measures", bad),
            Err(EditError::InvalidTargetEntityId),
            "target {bad:?} must be refused",
        );
    }
}

#[test]
fn edit_sets_are_non_empty() {
    assert_eq!(EditSet::new(vec![]), Err(EditError::EmptyEditSet));
    let set = EditSet::new(vec![
        OntologyEdit::upsert_properties(vec![prop("name")]).unwrap(),
    ])
    .unwrap();
    assert_eq!(set.edits().len(), 1);
}

#[test]
fn wire_property_names_fail_closed() {
    for (bad, expected) in [
        ("", WirePropertyError::EmptyPropertyName),
        ("   ", WirePropertyError::EmptyPropertyName),
        (" name", WirePropertyError::NotTrimmedPropertyName),
        ("name ", WirePropertyError::NotTrimmedPropertyName),
    ] {
        assert_eq!(
            WireProperty::new(
                bad,
                WireTier::Scalar,
                WireDataClass::InternalOnly,
                WireValue::String("v".into()),
            ),
            Err(expected),
            "property name {bad:?} must be refused",
        );
    }
}

#[test]
fn wire_doubles_are_finite_and_order_preserving() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(WireDouble::new(bad), Err(WireValueError::NonFiniteDouble));
    }
    assert_eq!(WireDouble::new(-0.0), WireDouble::new(0.0));
    let ordered = [-1000.5, -1.0, -0.0, 0.5, 1.0, 1000.25];
    let keys: Vec<WireDouble> = ordered
        .iter()
        .map(|v| WireDouble::new(*v).unwrap())
        .collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted, "derived order must equal numeric order");
    for (raw, key) in ordered.iter().zip(&keys) {
        let expected = if *raw == 0.0 { 0.0 } else { *raw };
        assert_eq!(key.get(), expected, "round-trip for {raw}");
    }
}

#[test]
fn wire_dates_are_validated() {
    assert!(WireDate::new(2024, 2, 29).is_ok());
    assert_eq!(WireDate::new(2023, 2, 29), Err(WireValueError::InvalidDate));
    assert_eq!(WireDate::new(1900, 2, 29), Err(WireValueError::InvalidDate));
    assert!(WireDate::new(2000, 2, 29).is_ok());
    assert_eq!(WireDate::new(2024, 0, 1), Err(WireValueError::InvalidDate));
    assert_eq!(WireDate::new(2024, 13, 1), Err(WireValueError::InvalidDate));
    assert_eq!(WireDate::new(2024, 4, 31), Err(WireValueError::InvalidDate));
    assert_eq!(WireDate::new(2024, 12, 0), Err(WireValueError::InvalidDate));
}

#[test]
fn wire_tag_numbering_is_frozen() {
    let tiers = [
        (WireTier::Scalar, 0),
        (WireTier::Vector, 1),
        (WireTier::Timeseries, 2),
        (WireTier::Geo, 3),
        (WireTier::Ciphertext, 4),
        (WireTier::Struct, 5),
    ];
    for (tier, byte) in tiers {
        assert_eq!(tier.tag(), byte);
        assert_eq!(WireTier::from_tag(byte), Some(tier));
    }
    assert_eq!(WireTier::from_tag(6), None);

    let classes = [
        (WireDataClass::Public, 0, "PUBLIC"),
        (WireDataClass::InternalOnly, 1, "INTERNAL_ONLY"),
        (WireDataClass::PiiIdentifying, 2, "PII_IDENTIFYING"),
        (WireDataClass::PiiQuasiIdentifier, 3, "PII_QUASI_IDENTIFIER"),
        (WireDataClass::Phi, 4, "PHI"),
        (WireDataClass::Pci, 5, "PCI"),
        (WireDataClass::Financial, 6, "FINANCIAL"),
        (
            WireDataClass::FinancialRegulatedCredit,
            7,
            "FINANCIAL_REGULATED_CREDIT",
        ),
        (
            WireDataClass::BehavioralTenantProduct,
            8,
            "BEHAVIORAL_TENANT_PRODUCT",
        ),
        (WireDataClass::BehavioralAds, 9, "BEHAVIORAL_ADS"),
        (WireDataClass::DeclaredPreference, 10, "DECLARED_PREFERENCE"),
        (WireDataClass::SearchQuery, 11, "SEARCH_QUERY"),
        (
            WireDataClass::SensitivePipaArt23,
            12,
            "SENSITIVE_PIPA_ART23",
        ),
    ];
    for (class, byte, label) in classes {
        assert_eq!(class.tag(), byte);
        assert_eq!(WireDataClass::from_tag(byte), Some(class));
        assert_eq!(class.label(), label);
    }
    assert_eq!(WireDataClass::from_tag(13), None);
}

#[test]
fn typed_values_nest() {
    let mut entries = BTreeMap::new();
    entries.insert(
        "count".to_string(),
        WireValue::Array(vec![WireValue::Integer(1), WireValue::Integer(2)]),
    );
    let value = WireValue::Struct(entries);
    let clone = value.clone();
    assert_eq!(value, clone);
}
