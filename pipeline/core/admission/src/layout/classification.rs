//! Retention of declared data classes across a change.

/// Classes that only restate the default, and so may be deleted freely.
const DEFAULT_CLASSES: [&str; 2] = ["INTERNAL_ONLY", "PUBLIC"];

/// Refuse deleting a non-default `data_class:` annotation from a changed file.
///
/// Classes are compared as a multiset per file, not by diff position: rustfmt
/// re-pads a trailing comment column when a neighbouring line changes width,
/// so an annotation moves without anything being lost.
pub fn deleted_classification_violations(path: &str, before: &[u8], after: &[u8]) -> Vec<String> {
    if !path.ends_with(".rs") {
        return Vec::new();
    }
    let (before, after) = (
        String::from_utf8_lossy(before),
        String::from_utf8_lossy(after),
    );
    let mut surviving: Vec<&str> = declared_classes(&after)
        .into_iter()
        .map(|it| it.1)
        .collect();
    declared_classes(&before)
        .into_iter()
        .filter(
            |(_, class)| match surviving.iter().position(|kept| kept == class) {
                Some(index) => {
                    surviving.swap_remove(index);
                    false
                }
                None => true,
            },
        )
        .map(|(line, class)| {
            format!(
                "{path}:{line}: `// data_class: {class}` was deleted; only INTERNAL_ONLY \
                 and PUBLIC restate the default and may go"
            )
        })
        .collect()
}

/// The class a comment on this line declares. The token is SCREAMING_SNAKE,
/// which separates a declared class from a field named `data_class` whose
/// type is PascalCase.
fn declared_class(line: &str) -> Option<&str> {
    let (head, rest) = line.split_once("data_class:")?;
    if !head.contains("//") {
        return None;
    }
    let token = rest.trim_start();
    let end = token
        .find(|byte: char| !byte.is_ascii_uppercase() && !byte.is_ascii_digit() && byte != '_')
        .unwrap_or(token.len());
    let token = &token[..end];
    token
        .starts_with(|byte: char| byte.is_ascii_uppercase())
        .then_some(token)
}

/// Every non-default class a file declares, as `(line, class)`.
fn declared_classes(text: &str) -> Vec<(usize, &str)> {
    let lines: Vec<&str> = text.lines().collect();
    (0..lines.len())
        .filter_map(|index| declared_class(lines[index]).map(|class| (index, class)))
        .filter(|(_, class)| !DEFAULT_CLASSES.contains(class))
        .filter(|(index, class)| !restates_the_declaration_below(&lines, *index, class))
        .map(|(index, class)| (index + 1, class))
        .collect()
}

/// Whether a comment-only line repeats the class of the declaration it
/// introduces, which is one judgment written twice.
fn restates_the_declaration_below(lines: &[&str], index: usize, class: &str) -> bool {
    lines[index].trim_start().starts_with("//")
        && lines[index + 1..]
            .iter()
            .find(|line| {
                let text = line.trim();
                !text.is_empty() && !text.starts_with("//") && !text.starts_with("#[")
            })
            .and_then(|line| declared_class(line))
            == Some(class)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATH: &str = "app/hr/core/employment-domain/src/lib.rs";

    /// Every non-default class the repository actually declares. A gate whose
    /// default list swallowed one of these would admit its deletion.
    const NON_DEFAULT: [&str; 6] = [
        "FINANCIAL",
        "TENANT_SCOPED",
        "PII_IDENTIFYING",
        "SENSITIVE_PIPA_ART23",
        "PROPERTY_VALUE_PRIVACY_CLASS",
        "SECRET",
    ];

    fn field(class: Option<&str>) -> String {
        match class {
            Some(class) => format!("    pub value: String, // data_class: {class}\n"),
            None => "    pub value: String,\n".to_owned(),
        }
    }

    fn deleting(class: &str) -> Vec<String> {
        deleted_classification_violations(
            PATH,
            field(Some(class)).as_bytes(),
            field(None).as_bytes(),
        )
    }

    #[test]
    fn a_deleted_privacy_class_is_refused_naming_file_and_line() {
        let refusals = deleting("PII_IDENTIFYING");
        assert_eq!(refusals.len(), 1, "{refusals:?}");
        assert!(
            refusals[0].starts_with(&format!("{PATH}:1:"))
                && refusals[0].contains("PII_IDENTIFYING"),
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
        for class in ["INTERNAL_ONLY", "PUBLIC"] {
            assert!(deleting(class).is_empty(), "{class} restates the default");
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
        assert!(
            deleted_classification_violations(PATH, before.as_bytes(), after.as_bytes()).is_empty()
        );
    }

    #[test]
    fn a_reformatted_annotation_is_not_a_deletion() {
        let after = "    pub value: String,   // data_class: FINANCIAL\n    pub n: u8,\n";
        assert!(
            deleted_classification_violations(
                PATH,
                field(Some("FINANCIAL")).as_bytes(),
                after.as_bytes(),
            )
            .is_empty(),
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
            deleted_classification_violations(PATH, before.as_bytes(), after.as_bytes()).is_empty(),
            "the class still stands on its own declaration"
        );
    }

    #[test]
    fn neighbours_sharing_a_class_each_answer_for_themselves() {
        let before = "    pub accrual_units: f64, // data_class: FINANCIAL\n\
                          pub deduction_units: f64, // data_class: FINANCIAL\n";
        let after = "    pub accrual_units: f64,\n\
                         pub deduction_units: f64, // data_class: FINANCIAL\n";
        assert_eq!(
            deleted_classification_violations(PATH, before.as_bytes(), after.as_bytes()).len(),
            1,
            "a field is not covered by the next field's class"
        );
    }

    #[test]
    fn a_field_named_data_class_is_not_an_annotation() {
        let before = "    pub data_class: Classified<DataClass>,\n";
        assert!(
            deleted_classification_violations(PATH, before.as_bytes(), b"").is_empty(),
            "a PascalCase type is not a declared class"
        );
    }

    #[test]
    fn only_rust_sources_are_judged() {
        assert!(
            deleted_classification_violations(
                "network/core/x/notes.txt",
                field(Some("SECRET")).as_bytes(),
                b"",
            )
            .is_empty()
        );
    }
}
