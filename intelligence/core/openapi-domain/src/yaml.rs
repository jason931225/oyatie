pub(crate) fn scalar_value_at_indent(
    lines: &[LogicalLine],
    range: std::ops::Range<usize>,
    indent: usize,
    key: &str,
) -> Option<String> {
    lines[range]
        .iter()
        .find(|line| {
            line.indent == indent && yaml_key(&line.text).is_some_and(|found| found == key)
        })
        .and_then(|line| yaml_value(&line.text))
        .map(clean_yaml_scalar)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LogicalLine {
    pub(crate) indent: usize,
    pub(crate) text: String,
}

pub(crate) fn logical_lines(contents: &str) -> Vec<LogicalLine> {
    contents
        .lines()
        .filter_map(|raw| {
            let without_comment = strip_yaml_comment(raw);
            let trimmed = without_comment.trim_end();
            if trimmed.trim().is_empty() {
                return None;
            }
            let indent = trimmed
                .chars()
                .take_while(|character| *character == ' ')
                .count();
            Some(LogicalLine {
                indent,
                text: trimmed[indent..].trim().to_string(),
            })
        })
        .collect()
}

fn strip_yaml_comment(line: &str) -> String {
    let mut in_single = false;
    let mut in_double = false;
    let mut previous = '\0';
    for (index, character) in line.char_indices() {
        match character {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single && previous != '\\' => in_double = !in_double,
            '#' if !in_single && !in_double => return line[..index].to_string(),
            _ => {}
        }
        previous = character;
    }
    line.to_string()
}

pub(crate) fn top_level_value(lines: &[LogicalLine], key: &str) -> Option<String> {
    lines
        .iter()
        .find(|line| line.indent == 0 && yaml_key(&line.text).is_some_and(|found| found == key))
        .and_then(|line| yaml_value(&line.text))
        .map(clean_yaml_scalar)
}

pub(crate) fn top_level_block(lines: &[LogicalLine], key: &str) -> Option<std::ops::Range<usize>> {
    let start = lines.iter().position(|line| {
        line.indent == 0 && yaml_key(&line.text).is_some_and(|found| found == key)
    })?;
    let end = lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find(|(_, line)| line.indent == 0)
        .map(|(index, _)| index)
        .unwrap_or(lines.len());
    Some(start + 1..end)
}

pub(crate) fn block_scalar_value(
    lines: &[LogicalLine],
    range: std::ops::Range<usize>,
    indent: usize,
    key: &str,
) -> Option<String> {
    lines[range]
        .iter()
        .find(|line| {
            line.indent == indent && yaml_key(&line.text).is_some_and(|found| found == key)
        })
        .and_then(|line| yaml_value(&line.text))
        .map(clean_yaml_scalar)
}

pub(crate) fn scalar_value_at_any_indent(
    lines: &[LogicalLine],
    range: std::ops::Range<usize>,
    key: &str,
) -> Option<String> {
    lines[range]
        .iter()
        .find(|line| yaml_key(&line.text).is_some_and(|found| found == key))
        .and_then(|line| yaml_value(&line.text))
        .map(clean_yaml_scalar)
}

pub(crate) fn list_item_yaml_value(line: &str, key: &str) -> Option<String> {
    let rest = line.strip_prefix("- ")?;
    let (found_key, value) = rest.split_once(':')?;
    if clean_yaml_key(found_key.trim()) == key && !value.trim().is_empty() {
        Some(clean_yaml_scalar(value.trim()))
    } else {
        None
    }
}

pub(crate) fn list_item_scalar(line: &str) -> Option<String> {
    line.strip_prefix("- ")
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.contains(':'))
        .map(clean_yaml_scalar)
}

pub(crate) fn find_next_at_or_above_indent(
    lines: &[LogicalLine],
    start: usize,
    end: usize,
    indent: usize,
) -> usize {
    lines[start..end]
        .iter()
        .position(|line| line.indent <= indent)
        .map(|offset| start + offset)
        .unwrap_or(end)
}

pub(crate) fn yaml_key(line: &str) -> Option<&str> {
    line.split_once(':')
        .map(|(key, _)| clean_yaml_key(key.trim()))
        .filter(|key| !key.is_empty())
}

pub(crate) fn yaml_value(line: &str) -> Option<&str> {
    line.split_once(':')
        .map(|(_, value)| value.trim())
        .filter(|value| !value.is_empty())
}

fn clean_yaml_key(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|inner| inner.strip_suffix('\''))
        })
        .unwrap_or(value)
}

pub(crate) fn clean_yaml_scalar(value: &str) -> String {
    clean_yaml_key(value.trim()).to_string()
}

pub(crate) fn component_schema_ref(value: &str) -> Option<String> {
    value
        .strip_prefix("#/components/schemas/")
        .map(str::to_string)
        .filter(|schema_name| !schema_name.trim().is_empty())
}
