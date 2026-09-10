use crate::rust_lexer::rust_code_without_comments_and_literals;
use crate::rust_source::{find_rust_identifier, is_rust_identifier_character};
pub(crate) fn runtime_test_covers_status(test: &str, status_type: &str, status: &str) -> bool {
    test.split(';')
        .any(|statement| runtime_test_asserts_status_code(statement, status_type, status))
}

fn runtime_test_asserts_status_code(statement: &str, status_type: &str, status: &str) -> bool {
    let code = rust_code_without_comments_and_literals(statement);
    statement_invokes_macro(&code, "assert_eq")
        && statement_uses_status_type_variant(&code, status_type)
        && statement_calls_code_method(&code)
        && statement_contains_response_status(&code, status)
}

fn statement_invokes_macro(code: &str, macro_name: &str) -> bool {
    let mut search_start = 0usize;
    while let Some(index) = find_rust_identifier(code, macro_name, search_start) {
        let cursor = skip_whitespace(code, index + macro_name.len());
        if code[cursor..].starts_with('!') {
            return true;
        }
        search_start = index + macro_name.len();
    }
    false
}

fn statement_uses_status_type_variant(code: &str, status_type: &str) -> bool {
    let mut search_start = 0usize;
    while let Some(index) = find_rust_identifier(code, status_type, search_start) {
        let cursor = skip_whitespace(code, index + status_type.len());
        if code[cursor..].starts_with("::") {
            return true;
        }
        search_start = index + status_type.len();
    }
    false
}

fn statement_calls_code_method(code: &str) -> bool {
    code.chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>()
        .contains(".code()")
}

pub(crate) fn skip_whitespace(source: &str, mut index: usize) -> usize {
    while let Some(character) = source[index..].chars().next() {
        if !character.is_whitespace() {
            break;
        }
        index += character.len_utf8();
    }
    index
}

fn statement_contains_response_status(statement: &str, status: &str) -> bool {
    let mut search_start = 0usize;
    while let Some(relative_index) = statement[search_start..].find(status) {
        let index = search_start + relative_index;
        let before_is_boundary = index == 0
            || statement[..index]
                .chars()
                .next_back()
                .is_none_or(|character| {
                    !is_rust_identifier_character(character) && character != '.'
                });
        let after_index = index + status.len();
        let after_is_boundary = after_index == statement.len()
            || statement[after_index..]
                .chars()
                .next()
                .is_none_or(|character| {
                    !is_rust_identifier_character(character) && character != '.'
                });
        if before_is_boundary && after_is_boundary {
            return true;
        }
        search_start = after_index;
    }
    false
}
