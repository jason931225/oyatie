use crate::runtime_test_scan::skip_whitespace;
use crate::rust_lexer::{
    consume_block_comment, consume_line_comment, quoted_string_literal_value,
    raw_string_literal_value, rust_code_without_comments_and_literals, starts_with_bytes,
};
pub(crate) fn rust_code_contains_identifier(source: &str, identifier: &str) -> bool {
    if !valid_rust_identifier(identifier) {
        return false;
    }
    let code = rust_code_without_comments_and_literals(source);
    find_rust_identifier(&code, identifier, 0).is_some()
}

pub(crate) fn rust_code_contains_public_function(source: &str, function_name: &str) -> bool {
    if !valid_rust_identifier(function_name) {
        return false;
    }
    let code = rust_code_without_comments_and_literals(source);
    let mut search_start = 0usize;
    while let Some((public_index, mut cursor)) = find_top_level_bare_pub_item(&code, search_start) {
        cursor = skip_rust_function_qualifiers(&code, cursor);
        if rust_identifier_at(&code, cursor, "fn") {
            cursor = skip_whitespace(&code, cursor + "fn".len());
            if rust_identifier_at(&code, cursor, function_name) {
                let after_name = skip_whitespace(&code, cursor + function_name.len());
                if code[after_name..].starts_with('(') {
                    return true;
                }
            }
        }
        search_start = public_index + "pub".len();
    }
    false
}

pub(crate) fn rust_code_contains_public_string_const(source: &str, expected: &str) -> bool {
    let code = rust_code_without_comments_and_literals(source);
    let mut search_start = 0usize;
    while let Some((public_index, mut cursor)) = find_top_level_bare_pub_item(&code, search_start) {
        if rust_identifier_at(&code, cursor, "const") {
            cursor = skip_whitespace(&code, cursor + "const".len());
            if let Some(name_end) = rust_identifier_end_at(&code, cursor) {
                let declaration_end = code[name_end..]
                    .find(';')
                    .map_or(code.len(), |relative| name_end + relative);
                if let Some(relative_equals) = code[name_end..declaration_end].find('=') {
                    let equals_index = name_end + relative_equals;
                    let value_index = skip_whitespace(source, equals_index + 1);
                    if string_literal_equals_at(source, value_index, expected) {
                        return true;
                    }
                }
            }
        }
        search_start = public_index + "pub".len();
    }
    false
}

pub(crate) fn find_public_rust_enum(code: &str, enum_name: &str) -> Option<usize> {
    if !valid_rust_identifier(enum_name) {
        return None;
    }
    let mut search_start = 0usize;
    while let Some((public_index, mut cursor)) = find_top_level_bare_pub_item(code, search_start) {
        if rust_identifier_at(code, cursor, "enum") {
            let enum_index = cursor;
            cursor = skip_whitespace(code, cursor + "enum".len());
            if rust_identifier_at(code, cursor, enum_name) {
                let after_name = skip_whitespace(code, cursor + enum_name.len());
                if code[after_name..].starts_with('{') {
                    return Some(enum_index);
                }
            }
        }
        search_start = public_index + "pub".len();
    }
    None
}

pub(crate) fn find_public_rust_struct(code: &str, struct_name: &str) -> Option<usize> {
    if !valid_rust_identifier(struct_name) {
        return None;
    }
    let mut search_start = 0usize;
    while let Some((public_index, mut cursor)) = find_top_level_bare_pub_item(code, search_start) {
        if rust_identifier_at(code, cursor, "struct") {
            let struct_index = cursor;
            cursor = skip_whitespace(code, cursor + "struct".len());
            if rust_identifier_at(code, cursor, struct_name) {
                let after_name = skip_whitespace(code, cursor + struct_name.len());
                if code[after_name..].starts_with('{') {
                    return Some(struct_index);
                }
            }
        }
        search_start = public_index + "pub".len();
    }
    None
}

fn find_top_level_bare_pub_item(code: &str, start: usize) -> Option<(usize, usize)> {
    let mut search_start = start;
    while let Some(public_index) = find_rust_identifier(code, "pub", search_start) {
        let item_start = skip_whitespace(code, public_index + "pub".len());
        if brace_depth_before(code, public_index) == 0 && !code[item_start..].starts_with('(') {
            return Some((public_index, item_start));
        }
        search_start = public_index + "pub".len();
    }
    None
}

fn skip_rust_function_qualifiers(code: &str, mut cursor: usize) -> usize {
    loop {
        let next = ["async", "unsafe", "extern", "const"]
            .iter()
            .find(|qualifier| rust_identifier_at(code, cursor, qualifier));
        let Some(qualifier) = next else {
            return cursor;
        };
        cursor = skip_whitespace(code, cursor + qualifier.len());
    }
}

fn rust_identifier_at(code: &str, index: usize, identifier: &str) -> bool {
    code.get(index..)
        .is_some_and(|remaining| remaining.starts_with(identifier))
        && (index == 0
            || code[..index]
                .chars()
                .next_back()
                .is_none_or(|character| !is_rust_identifier_character(character)))
        && (index + identifier.len() == code.len()
            || code[index + identifier.len()..]
                .chars()
                .next()
                .is_none_or(|character| !is_rust_identifier_character(character)))
}

fn rust_identifier_end_at(code: &str, index: usize) -> Option<usize> {
    let mut chars = code[index..].char_indices();
    let (_, first) = chars.next()?;
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return None;
    }
    let mut end = index + first.len_utf8();
    for (relative_index, character) in chars {
        if is_rust_identifier_character(character) {
            end = index + relative_index + character.len_utf8();
        } else {
            break;
        }
    }
    Some(end)
}

fn brace_depth_before(code: &str, index: usize) -> usize {
    let mut depth = 0usize;
    for character in code[..index].chars() {
        match character {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    depth
}

pub(crate) fn rust_string_literal_equals(source: &str, expected: &str) -> bool {
    let bytes = source.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        if starts_with_bytes(bytes, index, b"//") {
            index = consume_line_comment(bytes, index + 2);
            continue;
        }
        if starts_with_bytes(bytes, index, b"/*") {
            index = consume_block_comment(bytes, index + 2);
            continue;
        }
        if let Some((next_index, value)) = raw_string_literal_value(source, index) {
            if value == expected {
                return true;
            }
            index = next_index;
            continue;
        }
        if bytes[index] == b'"' {
            let (next_index, value) = quoted_string_literal_value(bytes, index + 1);
            if value == expected {
                return true;
            }
            index = next_index;
            continue;
        }
        index += 1;
    }
    false
}

fn string_literal_equals_at(source: &str, index: usize, expected: &str) -> bool {
    if let Some((_, value)) = raw_string_literal_value(source, index) {
        return value == expected;
    }
    if source.as_bytes().get(index).copied() == Some(b'"') {
        let (_, value) = quoted_string_literal_value(source.as_bytes(), index + 1);
        return value == expected;
    }
    false
}

fn valid_rust_identifier(identifier: &str) -> bool {
    let mut chars = identifier.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

pub(crate) fn find_rust_identifier(source: &str, identifier: &str, start: usize) -> Option<usize> {
    let mut search_start = start;
    while let Some(relative_index) = source[search_start..].find(identifier) {
        let index = search_start + relative_index;
        let before_is_boundary = index == 0
            || source[..index]
                .chars()
                .next_back()
                .is_none_or(|character| !is_rust_identifier_character(character));
        let after_index = index + identifier.len();
        let after_is_boundary = after_index == source.len()
            || source[after_index..]
                .chars()
                .next()
                .is_none_or(|character| !is_rust_identifier_character(character));
        if before_is_boundary && after_is_boundary {
            return Some(index);
        }
        search_start = after_index;
    }
    None
}

pub(crate) fn find_rust_keyword_identifier(
    code: &str,
    keyword: &str,
    identifier: &str,
    start: usize,
) -> Option<usize> {
    let mut search_start = start;
    while let Some(keyword_index) = find_rust_identifier(code, keyword, search_start) {
        let mut cursor = keyword_index + keyword.len();
        let mut saw_whitespace = false;
        while let Some(character) = code[cursor..].chars().next() {
            if !character.is_whitespace() {
                break;
            }
            saw_whitespace = true;
            cursor += character.len_utf8();
        }
        if saw_whitespace
            && code[cursor..].starts_with(identifier)
            && code[cursor + identifier.len()..]
                .chars()
                .next()
                .is_none_or(|character| !is_rust_identifier_character(character))
        {
            return Some(keyword_index);
        }
        search_start = keyword_index + keyword.len();
    }
    None
}

pub(crate) fn is_rust_identifier_character(character: char) -> bool {
    character == '_' || character.is_ascii_alphanumeric()
}
