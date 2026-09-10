use crate::rust_source::is_rust_identifier_character;
pub(crate) fn rust_code_without_comments_and_literals(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut output = bytes.to_vec();
    let mut index = 0usize;
    while index < bytes.len() {
        if starts_with_bytes(bytes, index, b"//") {
            let next_index = consume_line_comment(bytes, index + 2);
            mask_non_code_bytes(&mut output, index, next_index);
            index = next_index;
            continue;
        }
        if starts_with_bytes(bytes, index, b"/*") {
            let next_index = consume_block_comment(bytes, index + 2);
            mask_non_code_bytes(&mut output, index, next_index);
            index = next_index;
            continue;
        }
        if let Some((next_index, _)) = raw_string_literal_value(source, index) {
            mask_non_code_bytes(&mut output, index, next_index);
            index = next_index;
            continue;
        }
        if bytes[index] == b'"' {
            let (next_index, _) = quoted_string_literal_value(bytes, index + 1);
            mask_non_code_bytes(&mut output, index, next_index);
            index = next_index;
            continue;
        }
        if starts_simple_char_literal(bytes, index) {
            let next_index = consume_quoted_literal(bytes, index + 1, b'\'');
            mask_non_code_bytes(&mut output, index, next_index);
            index = next_index;
            continue;
        }
        index += 1;
    }
    // ADR-0083 Tier 1: `mask_non_code_bytes` overwrites individual bytes
    // inside comment/literal byte ranges, which can split multi-byte UTF-8
    // sequences. Use `from_utf8_lossy` to keep the helper infallible
    // without an `.expect()` and without panicking on inputs that contain
    // non-ASCII characters inside literals or comments.
    String::from_utf8_lossy(&output).into_owned()
}

fn mask_non_code_bytes(output: &mut [u8], start: usize, end: usize) {
    for byte in &mut output[start..end] {
        if *byte != b'\n' && *byte != b'\r' {
            *byte = b' ';
        }
    }
}

pub(crate) fn starts_with_bytes(bytes: &[u8], index: usize, expected: &[u8]) -> bool {
    bytes
        .get(index..)
        .is_some_and(|remaining| remaining.starts_with(expected))
}

pub(crate) fn consume_line_comment(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() && bytes[index] != b'\n' {
        index += 1;
    }
    index
}

pub(crate) fn consume_block_comment(bytes: &[u8], mut index: usize) -> usize {
    let mut depth = 1usize;
    while index < bytes.len() {
        if starts_with_bytes(bytes, index, b"/*") {
            depth += 1;
            index += 2;
            continue;
        }
        if starts_with_bytes(bytes, index, b"*/") {
            depth -= 1;
            index += 2;
            if depth == 0 {
                return index;
            }
            continue;
        }
        index += 1;
    }
    bytes.len()
}

fn consume_quoted_literal(bytes: &[u8], mut index: usize, delimiter: u8) -> usize {
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index = (index + 2).min(bytes.len()),
            byte if byte == delimiter => return index + 1,
            _ => index += 1,
        }
    }
    bytes.len()
}

fn starts_simple_char_literal(bytes: &[u8], index: usize) -> bool {
    if bytes.get(index).copied() != Some(b'\'') {
        return false;
    }
    bytes
        .get(index + 1)
        .copied()
        .is_some_and(|next| next == b'\\' || bytes.get(index + 2).copied() == Some(b'\''))
}

pub(crate) fn quoted_string_literal_value(bytes: &[u8], mut index: usize) -> (usize, String) {
    let mut value = String::new();
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => {
                if let Some(escaped) = bytes.get(index + 1).copied() {
                    value.push(match escaped {
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'\\' => '\\',
                        b'"' => '"',
                        other => other as char,
                    });
                }
                index = (index + 2).min(bytes.len());
            }
            b'"' => return (index + 1, value),
            byte => {
                value.push(byte as char);
                index += 1;
            }
        }
    }
    (bytes.len(), value)
}

pub(crate) fn raw_string_literal_value(source: &str, index: usize) -> Option<(usize, &str)> {
    let bytes = source.as_bytes();
    if bytes.get(index).copied()? != b'r' {
        return None;
    }
    if index > 0
        && source[..index]
            .chars()
            .next_back()
            .is_some_and(is_rust_identifier_character)
    {
        return None;
    }
    let mut delimiter_index = index + 1;
    while bytes.get(delimiter_index).copied() == Some(b'#') {
        delimiter_index += 1;
    }
    if bytes.get(delimiter_index).copied() != Some(b'"') {
        return None;
    }
    let hashes = delimiter_index - index - 1;
    let content_start = delimiter_index + 1;
    let mut search_index = content_start;
    while search_index < bytes.len() {
        if bytes[search_index] == b'"'
            && bytes
                .get(search_index + 1..search_index + 1 + hashes)
                .is_some_and(|suffix| suffix.iter().all(|byte| *byte == b'#'))
        {
            let end_index = search_index + 1 + hashes;
            return Some((end_index, &source[content_start..search_index]));
        }
        search_index += 1;
    }
    Some((bytes.len(), &source[content_start..]))
}
