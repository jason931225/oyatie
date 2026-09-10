use super::*;

const PATH: &str = "app/hr/core/employment-domain/src/lib.rs";

/// The non-default classes this parser observes across `dev` at 4316afb03,
/// each with at least one live annotation. A class born after that commit is
/// not covered here; a class listed here that the default list swallowed
/// would become freely deletable with this suite still green.
const NON_DEFAULT: [&str; 21] = [
    "FINANCIAL",
    "TENANT_SCOPED",
    "PII_IDENTIFYING",
    "SECRET",
    "AUDIT",
    "PROPERTY_VALUE_PRIVACY_CLASS",
    "PII_QUASI_IDENTIFIER",
    "FINANCIAL_REGULATED_CREDIT",
    "SENSITIVE_PIPA_ART23",
    "SECRET_REFERENCE",
    "BEHAVIORAL_TENANT_PRODUCT",
    "TENANT_PUBLIC",
    "TENANT_AUDIT",
    "AUTHENTICATION",
    "AUDIT_INTERNAL",
    "TENANT_PRIVATE",
    "SECRET_REF",
    "SEARCH_QUERY",
    "CREDENTIAL",
    "TENANT_PAYLOAD",
    "CARRIED_BY_CLASSIFIED_FIELD",
];

fn field(class: Option<&str>) -> String {
    match class {
        Some(class) => format!("    pub value: String, // data_class: {class}\n"),
        None => "    pub value: String,\n".to_owned(),
    }
}

fn deleting(class: &str) -> Vec<String> {
    deleted_classification_violations(PATH, field(Some(class)).as_bytes(), field(None).as_bytes())
}

fn refusals(before: &str, after: &str) -> Vec<String> {
    deleted_classification_violations(PATH, before.as_bytes(), after.as_bytes())
}

#[test]
fn a_deleted_privacy_class_is_refused_naming_file_and_line() {
    let refusals = deleting("PII_IDENTIFYING");
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert!(
        refusals[0].starts_with(&format!("{PATH}:1:")) && refusals[0].contains("PII_IDENTIFYING"),
        "{}",
        refusals[0]
    );
}

#[test]
fn a_deleted_legal_class_is_refused() {
    assert_eq!(deleting("SENSITIVE_PIPA_ART23").len(), 1);
}

#[test]
fn every_non_default_class_is_refused_when_deleted() {
    for class in NON_DEFAULT {
        assert_eq!(deleting(class).len(), 1, "{class} may not be deleted");
    }
}

#[test]
fn a_deleted_default_class_is_admitted() {
    for class in DEFAULT_CLASSES {
        assert!(deleting(class).is_empty(), "{class} restates the default");
    }
}

#[test]
fn the_default_list_is_the_two_classes_that_carry_no_judgment() {
    assert_eq!(&DEFAULT_CLASSES[..], ["INTERNAL_ONLY", "PUBLIC"]);
    let refusal = &deleting("FINANCIAL")[0];
    for class in DEFAULT_CLASSES {
        assert!(refusal.contains(class), "{refusal} omits {class}");
    }
}

#[test]
fn a_default_carrying_trailing_prose_is_still_a_default() {
    assert!(
        deleting("INTERNAL_ONLY - private: see the unforgeability note above").is_empty(),
        "prose after the class does not promote it to a judgment"
    );
}

#[test]
fn a_change_that_touches_no_annotation_is_admitted() {
    let before = format!("{}    pub other: u32,\n", field(Some("FINANCIAL")));
    let after = format!("{}    pub other: u64,\n", field(Some("FINANCIAL")));
    assert!(refusals(&before, &after).is_empty());
}

#[test]
fn a_reformatted_annotation_is_not_a_deletion() {
    let after = "    pub value: String,   // data_class: FINANCIAL\n    pub n: u8,\n";
    assert!(
        refusals(&field(Some("FINANCIAL")), after).is_empty(),
        "rustfmt re-padding a comment column loses nothing"
    );
}

#[test]
fn a_restatement_of_the_declaration_below_may_go() {
    let before = "/// data_class: TENANT_SCOPED\n\
                  #[derive(Clone)]\n\
                  pub struct AgentToken(pub String); // data_class: TENANT_SCOPED\n";
    let after = "#[derive(Clone)]\n\
                 pub struct AgentToken(pub String); // data_class: TENANT_SCOPED\n";
    assert!(
        refusals(before, after).is_empty(),
        "the class still stands on its own declaration"
    );
}

#[test]
fn a_restatement_of_every_class_on_a_multi_class_declaration_may_go() {
    let before = "/// data_class: AUDIT_INTERNAL, TENANT_AUDIT\n\
                  pub struct Event(String); // data_class: AUDIT_INTERNAL, TENANT_AUDIT\n";
    let after = "pub struct Event(String); // data_class: AUDIT_INTERNAL, TENANT_AUDIT\n";
    assert!(
        refusals(before, after).is_empty(),
        "a restatement covers each class it names, not only the first"
    );
}

#[test]
fn prose_mentioning_a_class_does_not_answer_for_its_declaration() {
    let before = "/// The raw bytes MUST NOT reach a log (data_class: CREDENTIAL).\n\
                  pub struct ProviderCredential(Bytes); // data_class: CREDENTIAL\n";
    let after = "/// The raw bytes MUST NOT reach a log (data_class: CREDENTIAL).\n\
                 pub struct ProviderCredential(Bytes);\n";
    let refusals = refusals(before, after);
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert!(
        refusals[0].starts_with(&format!("{PATH}:2:")),
        "{}",
        refusals[0]
    );
}

#[test]
fn every_class_on_a_comma_separated_annotation_is_retained() {
    let before = "/// data_class: TENANT_PRIVATE, FINANCIAL, PII_IDENTIFYING (scope-dependent)\n\
                  pub struct Balance(Money);\n";
    let after = "/// data_class: TENANT_PRIVATE\n\
                 pub struct Balance(Money);\n";
    let refusals = refusals(before, after);
    assert_eq!(refusals.len(), 2, "{refusals:?}");
    assert!(
        refusals.iter().any(|it| it.contains("FINANCIAL"))
            && refusals.iter().any(|it| it.contains("PII_IDENTIFYING")),
        "{refusals:?}"
    );
}

#[test]
fn a_class_after_a_plus_is_retained_even_when_the_first_is_a_default() {
    let before = "    pub payees: Vec<Payee>, // data_class: INTERNAL_ONLY + FINANCIAL\n";
    let after = "    pub payees: Vec<Payee>,\n";
    let refusals = refusals(before, after);
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert!(refusals[0].contains("FINANCIAL"), "{}", refusals[0]);
}

#[test]
fn neighbours_sharing_a_class_each_answer_for_themselves() {
    let before = "    pub accrual_units: f64, // data_class: FINANCIAL\n\
                      pub deduction_units: f64, // data_class: FINANCIAL\n";
    let after = "    pub accrual_units: f64,\n\
                     pub deduction_units: f64, // data_class: FINANCIAL\n";
    assert_eq!(
        refusals(before, after).len(),
        1,
        "a field is not covered by the next field's class"
    );
}

#[test]
fn a_field_named_data_class_is_not_an_annotation() {
    assert!(
        refusals(
            "    pub data_class: Classified<DataClass>,\n",
            "    pub n: u8,\n"
        )
        .is_empty(),
        "the marker must sit in a comment to declare anything"
    );
}

#[test]
fn a_file_absent_from_the_head_commit_is_left_to_the_path_gates() {
    assert!(
        deleted_classification_violations(PATH, field(Some("SECRET")).as_bytes(), b"").is_empty(),
        "deleting a file is not scrubbing its annotations"
    );
}

#[test]
fn only_rust_sources_are_judged() {
    assert!(
        deleted_classification_violations(
            "network/core/x/notes.txt",
            field(Some("SECRET")).as_bytes(),
            b"    pub value: String,\n",
        )
        .is_empty()
    );
}
