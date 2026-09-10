use crate::runtime_test_scan::skip_whitespace;
use crate::rust_lexer::quoted_string_literal_value;
use crate::rust_source::is_rust_identifier_character;
pub(crate) fn serde_rename_attribute(attribute: &str) -> Option<String> {
    serde_attribute_string_value(attribute, "rename")
}

pub(crate) fn serde_alias_attribute(attribute: &str) -> Option<String> {
    serde_attribute_string_value(attribute, "alias")
}

pub(crate) fn serde_rename_all_attribute(attribute: &str) -> Option<String> {
    serde_attribute_string_value(attribute, "rename_all")
}

pub(crate) fn serde_skip_attribute(attribute: &str) -> bool {
    serde_word_attribute(attribute, "skip")
}

pub(crate) fn serde_flatten_attribute(attribute: &str) -> bool {
    serde_word_attribute(attribute, "flatten")
}

pub(crate) fn serde_skip_serializing_if_attribute(attribute: &str) -> bool {
    serde_attribute_string_value(attribute, "skip_serializing_if").is_some()
}

pub(crate) fn serde_default_attribute(attribute: &str) -> bool {
    serde_word_attribute(attribute, "default")
        || serde_attribute_string_value(attribute, "default").is_some()
}

fn serde_word_attribute(attribute: &str, word: &str) -> bool {
    serde_attribute_body(attribute).is_some_and(|body| {
        serde_attribute_clauses(body)
            .into_iter()
            .any(|clause| clause.trim() == word)
    })
}

fn serde_attribute_string_value(attribute: &str, key: &str) -> Option<String> {
    let body = serde_attribute_body(attribute)?;
    serde_attribute_clauses(body)
        .into_iter()
        .find_map(|clause| serde_clause_string_value(clause.trim(), key))
}

fn serde_attribute_body(attribute: &str) -> Option<&str> {
    attribute
        .trim()
        .strip_prefix("#[serde(")?
        .strip_suffix(")]")
}

fn serde_attribute_clauses(body: &str) -> Vec<&str> {
    let bytes = body.as_bytes();
    let mut clauses = Vec::new();
    let mut clause_start = 0usize;
    let mut index = 0usize;
    let mut nesting_depth = 0usize;
    while index < bytes.len() {
        match bytes[index] {
            b'"' => {
                let (next_index, _) = quoted_string_literal_value(bytes, index + 1);
                index = next_index;
            }
            b'(' | b'[' | b'{' => {
                nesting_depth += 1;
                index += 1;
            }
            b')' | b']' | b'}' => {
                nesting_depth = nesting_depth.saturating_sub(1);
                index += 1;
            }
            b',' if nesting_depth == 0 => {
                clauses.push(&body[clause_start..index]);
                index += 1;
                clause_start = index;
            }
            _ => index += 1,
        }
    }
    clauses.push(&body[clause_start..]);
    clauses
}

fn serde_clause_string_value(clause: &str, key: &str) -> Option<String> {
    let rest = clause.strip_prefix(key)?;
    if rest
        .chars()
        .next()
        .is_some_and(is_rust_identifier_character)
    {
        return None;
    }
    let equals_index = skip_whitespace(clause, key.len());
    if !clause[equals_index..].starts_with('=') {
        return None;
    }
    let value_index = skip_whitespace(clause, equals_index + 1);
    if clause.as_bytes().get(value_index).copied() != Some(b'"') {
        return None;
    }
    let (_, value) = quoted_string_literal_value(clause.as_bytes(), value_index + 1);
    (!value.is_empty()).then_some(value)
}

pub(crate) fn unsupported_serde_field_attribute(attribute: &str) -> Option<String> {
    let body = serde_attribute_body(attribute)?;
    serde_attribute_clauses(body)
        .into_iter()
        .map(str::trim)
        .filter(|clause| !clause.is_empty())
        .find(|clause| !supported_serde_field_clause(clause))
        .map(serde_clause_name)
}

fn supported_serde_field_clause(clause: &str) -> bool {
    matches!(clause, "skip" | "flatten" | "default")
        || serde_clause_string_value(clause, "rename").is_some()
        || serde_clause_string_value(clause, "alias").is_some()
        || serde_clause_string_value(clause, "skip_serializing_if").is_some()
        || serde_clause_string_value(clause, "default").is_some()
}

pub(crate) fn unsupported_serde_struct_attribute(attribute: &str) -> Option<String> {
    let body = serde_attribute_body(attribute)?;
    serde_attribute_clauses(body)
        .into_iter()
        .map(str::trim)
        .filter(|clause| !clause.is_empty())
        .find(|clause| !supported_serde_struct_clause(clause))
        .map(serde_clause_name)
}

fn supported_serde_struct_clause(clause: &str) -> bool {
    serde_clause_string_value(clause, "rename_all").is_some()
}

fn serde_clause_name(clause: &str) -> String {
    let mut end = 0usize;
    for (index, character) in clause.char_indices() {
        if index == 0 && !is_rust_identifier_character(character) {
            return clause.to_string();
        }
        if !is_rust_identifier_character(character) {
            break;
        }
        end = index + character.len_utf8();
    }
    if end == 0 {
        clause.to_string()
    } else {
        clause[..end].to_string()
    }
}

pub(crate) fn supported_serde_rename_all_rule(style: &str) -> bool {
    matches!(style, "camelCase" | "snake_case")
}

pub(crate) fn apply_supported_serde_rename_all(field_name: &str, style: &str) -> String {
    match style {
        "camelCase" => snake_to_lower_camel(field_name),
        "snake_case" => field_name.to_string(),
        _ => unreachable!("unsupported serde rename_all rules fail before field mapping"),
    }
}

fn snake_to_lower_camel(field_name: &str) -> String {
    let mut renamed = String::new();
    let mut uppercase_next = false;
    for character in field_name.chars() {
        if character == '_' {
            uppercase_next = true;
            continue;
        }
        if uppercase_next {
            renamed.extend(character.to_uppercase());
            uppercase_next = false;
        } else {
            renamed.push(character);
        }
    }
    renamed
}
