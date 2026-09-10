use crate::path_validation::valid_numeric_response_status;
use crate::runtime_binding_source::RuntimeStatusParseError;
use crate::rust_lexer::rust_code_without_comments_and_literals;
use crate::rust_source::{find_public_rust_enum, find_rust_keyword_identifier};
use std::collections::BTreeSet;

pub(crate) fn rust_enum_variants(
    source: &str,
    status_type: &str,
) -> Result<BTreeSet<String>, RuntimeStatusParseError> {
    let code = rust_code_without_comments_and_literals(source);
    let enum_index = find_public_rust_enum(&code, status_type)
        .ok_or(RuntimeStatusParseError::MissingStatusType)?;
    let brace_index = enum_index
        + code[enum_index..].find('{').ok_or_else(|| {
            RuntimeStatusParseError::Invalid(format!(
                "{status_type} enum declaration must have a body"
            ))
        })?;
    let block_end = balanced_brace_end(&code, brace_index).ok_or_else(|| {
        RuntimeStatusParseError::Invalid(format!(
            "{status_type} enum declaration has unbalanced braces"
        ))
    })?;
    let block = &source[brace_index + 1..block_end];
    let mut variants = BTreeSet::new();
    for segment in block.split(',') {
        let Some(variant) = rust_fieldless_variant_name(status_type, segment)? else {
            continue;
        };
        if !variants.insert(variant.clone()) {
            return Err(RuntimeStatusParseError::Invalid(format!(
                "{status_type} declares variant {variant} more than once"
            )));
        }
    }
    Ok(variants)
}

fn rust_fieldless_variant_name(
    status_type: &str,
    segment: &str,
) -> Result<Option<String>, RuntimeStatusParseError> {
    let cleaned = segment
        .lines()
        .map(str::trim)
        .filter(|line| {
            !line.is_empty()
                && !line.starts_with("#[")
                && !line.starts_with("//")
                && !line.starts_with("/*")
        })
        .collect::<Vec<_>>()
        .join(" ");
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let mut chars = trimmed.char_indices().peekable();
    let Some((_, first)) = chars.peek().copied() else {
        return Ok(None);
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return Err(RuntimeStatusParseError::Invalid(format!(
            "{status_type} status enum contains an invalid variant declaration: {trimmed}"
        )));
    }
    let mut name_end = 0usize;
    for (index, character) in chars {
        if character == '_' || character.is_ascii_alphanumeric() {
            name_end = index + character.len_utf8();
        } else {
            break;
        }
    }
    let name = &trimmed[..name_end];
    if name.is_empty() {
        return Ok(None);
    }
    let remainder = trimmed[name_end..].trim();
    if !remainder.is_empty() {
        return Err(RuntimeStatusParseError::Invalid(format!(
            "{status_type} status enum variant {name} must be fieldless"
        )));
    }
    Ok(Some(name.to_string()))
}

pub(crate) fn rust_status_code_match_blocks<'a>(
    source: &'a str,
    status_type: &str,
) -> Vec<&'a str> {
    let mut blocks = Vec::new();
    for impl_block in rust_impl_blocks(source, status_type) {
        let impl_code = rust_code_without_comments_and_literals(impl_block);
        let mut search_start = 0usize;
        while let Some(fn_index) =
            find_rust_keyword_identifier(&impl_code, "fn", "code", search_start)
        {
            let Some(relative_fn_brace_index) = impl_code[fn_index..].find('{') else {
                break;
            };
            let fn_brace_index = fn_index + relative_fn_brace_index;
            let Some(fn_end) = balanced_brace_end(&impl_code, fn_brace_index) else {
                break;
            };
            let fn_block = &impl_block[fn_brace_index..=fn_end];
            if let Some(match_block) = rust_match_self_block(fn_block) {
                blocks.push(match_block);
            }
            search_start = fn_end + 1;
        }
    }
    blocks
}

fn rust_match_self_block(source: &str) -> Option<&str> {
    let code = rust_code_without_comments_and_literals(source);
    let match_index = find_rust_keyword_identifier(&code, "match", "self", 0)?;
    let brace_index = match_index + code[match_index..].find('{')?;
    let block_end = balanced_brace_end(&code, brace_index)?;
    Some(&source[brace_index + 1..block_end])
}

fn rust_impl_blocks<'a>(source: &'a str, status_type: &str) -> Vec<&'a str> {
    let code = rust_code_without_comments_and_literals(source);
    let mut blocks = Vec::new();
    let mut search_start = 0usize;
    while let Some(impl_index) =
        find_rust_keyword_identifier(&code, "impl", status_type, search_start)
    {
        let Some(relative_brace_index) = code[impl_index..].find('{') else {
            break;
        };
        let brace_index = impl_index + relative_brace_index;
        let Some(block_end) = balanced_brace_end(&code, brace_index) else {
            break;
        };
        blocks.push(&source[brace_index..=block_end]);
        search_start = block_end + 1;
    }
    blocks
}

pub(crate) fn balanced_brace_end(source: &str, open_brace_index: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (relative_index, character) in source[open_brace_index..].char_indices() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(open_brace_index + relative_index);
                }
            }
            _ => {}
        }
    }
    None
}

pub(crate) fn status_match_mappings(
    block: &str,
) -> Result<Vec<(String, String)>, RuntimeStatusParseError> {
    let mut mappings = Vec::new();
    for arm in block.split(',') {
        let Some((left, right)) = arm.split_once("=>") else {
            continue;
        };
        let Some(status) = explicit_status_literal(right) else {
            return Err(RuntimeStatusParseError::Invalid(
                "status code mappings must return explicit three-digit numeric literals".into(),
            ));
        };
        let mut mapped_any_variant = false;
        for variant_expression in left.split('|') {
            let Some(variant) = self_variant_name(variant_expression) else {
                return Err(RuntimeStatusParseError::Invalid(
                    "status code mappings must use explicit Self::Variant arms".into(),
                ));
            };
            mappings.push((variant, status.clone()));
            mapped_any_variant = true;
        }
        if !mapped_any_variant {
            return Err(RuntimeStatusParseError::Invalid(
                "status code mappings must include at least one Self::Variant arm".into(),
            ));
        }
    }
    Ok(mappings)
}

fn explicit_status_literal(expression: &str) -> Option<String> {
    let trimmed = expression.trim();
    let status = trimmed
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect::<String>();
    if !valid_numeric_response_status(&status) {
        return None;
    }
    if !trimmed[status.len()..].trim().is_empty() {
        return None;
    }
    Some(status)
}

fn self_variant_name(expression: &str) -> Option<String> {
    let trimmed = expression.trim();
    let suffix = trimmed.strip_prefix("Self::")?;
    let mut name_end = 0usize;
    for (index, character) in suffix.char_indices() {
        if character == '_' || character.is_ascii_alphanumeric() {
            name_end = index + character.len_utf8();
        } else {
            break;
        }
    }
    let name = &suffix[..name_end];
    if name.is_empty() {
        return None;
    }
    if !suffix[name_end..].trim().is_empty() {
        return None;
    }
    Some(name.to_string())
}
