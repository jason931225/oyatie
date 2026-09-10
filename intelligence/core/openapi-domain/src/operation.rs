use crate::error::OpenApiSourceError;
use crate::path_validation::{
    collides_with_fixed_operation_method, is_fixed_operation_method, valid_numeric_response_status,
    valid_response_key,
};
use crate::yaml::{
    LogicalLine, clean_yaml_scalar, component_schema_ref, find_next_at_or_above_indent,
    logical_lines, top_level_block, yaml_key, yaml_value,
};
use std::collections::BTreeMap;
use std::collections::BTreeSet;

const ADDITIONAL_OPERATIONS_FIELD: &str = "additionalOperations";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OpenApiOperation {
    pub(crate) operation_id: String,
    pub(crate) contract_path: String,
    pub(crate) response_statuses: BTreeSet<String>,
    pub(crate) non_explicit_response_keys: BTreeSet<String>,
    pub(crate) response_schema_refs: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct OpenApiResponseKeys {
    explicit_statuses: BTreeSet<String>,
    non_explicit_keys: BTreeSet<String>,
    schema_refs: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PathItemOperationBlock {
    pub(crate) method: String,
    pub(crate) range: std::ops::Range<usize>,
}

pub(crate) fn collect_document_operations(
    path: &str,
    contents: &str,
) -> Result<Vec<OpenApiOperation>, OpenApiSourceError> {
    let lines = logical_lines(contents);
    let paths_range = top_level_block(&lines, "paths").ok_or_else(|| {
        OpenApiSourceError::MissingTopLevelField {
            path: path.into(),
            field: "paths",
        }
    })?;
    let mut operations = Vec::new();
    let mut index = paths_range.start;
    while index < paths_range.end {
        let line = &lines[index];
        if line.indent != 2 || !line.text.starts_with('/') {
            index += 1;
            continue;
        }
        let Some(api_path) = yaml_key(&line.text).map(str::to_string) else {
            index += 1;
            continue;
        };
        let next_path_index = find_next_at_or_above_indent(&lines, index + 1, paths_range.end, 2);
        collect_path_item_operations(
            path,
            &api_path,
            &lines,
            index + 1..next_path_index,
            &mut operations,
        )?;
        index = next_path_index;
    }
    Ok(operations)
}

fn collect_path_item_operations(
    document_path: &str,
    api_path: &str,
    lines: &[LogicalLine],
    range: std::ops::Range<usize>,
    operations: &mut Vec<OpenApiOperation>,
) -> Result<(), OpenApiSourceError> {
    for block in path_item_operation_blocks(document_path, api_path, lines, range)? {
        let operation_id = operation_id_in_range(lines, block.range.clone()).ok_or_else(|| {
            OpenApiSourceError::MissingOperationId {
                path: document_path.into(),
                api_path: api_path.into(),
                method: block.method.clone(),
            }
        })?;
        let response_keys = response_keys_in_range(lines, block.range);
        operations.push(OpenApiOperation {
            operation_id,
            contract_path: document_path.into(),
            response_statuses: response_keys.explicit_statuses,
            non_explicit_response_keys: response_keys.non_explicit_keys,
            response_schema_refs: response_keys.schema_refs,
        });
    }
    Ok(())
}

pub(crate) fn path_item_operation_blocks(
    document_path: &str,
    api_path: &str,
    lines: &[LogicalLine],
    range: std::ops::Range<usize>,
) -> Result<Vec<PathItemOperationBlock>, OpenApiSourceError> {
    let mut blocks = Vec::new();
    let mut index = range.start;
    while index < range.end {
        let line = &lines[index];
        let Some(key) = yaml_key(&line.text) else {
            index += 1;
            continue;
        };
        if line.indent != 4 {
            index += 1;
            continue;
        }
        if is_fixed_operation_method(key) {
            let next_operation_index = find_next_at_or_above_indent(lines, index + 1, range.end, 4);
            blocks.push(PathItemOperationBlock {
                method: key.into(),
                range: index + 1..next_operation_index,
            });
            index = next_operation_index;
            continue;
        }
        if key == ADDITIONAL_OPERATIONS_FIELD {
            let additional_end = find_next_at_or_above_indent(lines, index + 1, range.end, 4);
            collect_additional_operation_blocks(
                document_path,
                api_path,
                lines,
                index + 1..additional_end,
                &mut blocks,
            )?;
            index = additional_end;
            continue;
        }
        index += 1;
    }
    Ok(blocks)
}

fn collect_additional_operation_blocks(
    document_path: &str,
    api_path: &str,
    lines: &[LogicalLine],
    range: std::ops::Range<usize>,
    blocks: &mut Vec<PathItemOperationBlock>,
) -> Result<(), OpenApiSourceError> {
    let mut index = range.start;
    while index < range.end {
        let line = &lines[index];
        if line.indent != 6 {
            index += 1;
            continue;
        }
        let Some(method) = yaml_key(&line.text) else {
            index += 1;
            continue;
        };
        if collides_with_fixed_operation_method(method) {
            return Err(
                OpenApiSourceError::AdditionalOperationFixedMethodCollision {
                    path: document_path.into(),
                    api_path: api_path.into(),
                    method: method.into(),
                },
            );
        }
        let next_operation_index = find_next_at_or_above_indent(lines, index + 1, range.end, 6);
        blocks.push(PathItemOperationBlock {
            method: method.into(),
            range: index + 1..next_operation_index,
        });
        index = next_operation_index;
    }
    Ok(())
}

fn operation_id_in_range(lines: &[LogicalLine], range: std::ops::Range<usize>) -> Option<String> {
    lines[range]
        .iter()
        .find(|line| {
            line.indent > 4 && yaml_key(&line.text).is_some_and(|found| found == "operationId")
        })
        .and_then(|line| yaml_value(&line.text))
        .map(clean_yaml_scalar)
        .filter(|value| !value.trim().is_empty())
}

fn response_keys_in_range(
    lines: &[LogicalLine],
    range: std::ops::Range<usize>,
) -> OpenApiResponseKeys {
    let Some(responses_index) = lines[range.clone()].iter().position(|line| {
        line.indent > 4 && yaml_key(&line.text).is_some_and(|found| found == "responses")
    }) else {
        return OpenApiResponseKeys::default();
    };
    let responses_index = range.start + responses_index;
    let responses_indent = lines[responses_index].indent;
    let responses_end =
        find_next_at_or_above_indent(lines, responses_index + 1, range.end, responses_indent);
    let mut response_keys = OpenApiResponseKeys::default();
    let response_indent = responses_indent + 2;
    let mut index = responses_index + 1;
    while index < responses_end {
        let line = &lines[index];
        let Some(key) = yaml_key(&line.text) else {
            index += 1;
            continue;
        };
        if line.indent != response_indent || !valid_response_key(key) {
            index += 1;
            continue;
        }
        let response_end =
            find_next_at_or_above_indent(lines, index + 1, responses_end, line.indent);
        if valid_numeric_response_status(key) {
            response_keys.explicit_statuses.insert(key.to_string());
            if let Some(schema_ref) = response_schema_ref_for(lines, index + 1..response_end) {
                response_keys
                    .schema_refs
                    .insert(key.to_string(), schema_ref);
            }
        } else {
            response_keys.non_explicit_keys.insert(key.to_string());
        }
        index = response_end;
    }
    response_keys
}

fn response_schema_ref_for(
    lines: &[LogicalLine],
    response_range: std::ops::Range<usize>,
) -> Option<String> {
    let content_index = lines[response_range.clone()]
        .iter()
        .position(|line| yaml_key(&line.text).is_some_and(|found| found == "content"))?
        + response_range.start;
    let content_indent = lines[content_index].indent;
    let content_end =
        find_next_at_or_above_indent(lines, content_index + 1, response_range.end, content_indent);
    let json_index = lines[content_index + 1..content_end]
        .iter()
        .position(|line| yaml_key(&line.text).is_some_and(|found| found == "application/json"))?
        + content_index
        + 1;
    let json_indent = lines[json_index].indent;
    let json_end = find_next_at_or_above_indent(lines, json_index + 1, content_end, json_indent);
    let schema_index = lines[json_index + 1..json_end]
        .iter()
        .position(|line| yaml_key(&line.text).is_some_and(|found| found == "schema"))?
        + json_index
        + 1;
    let schema_indent = lines[schema_index].indent;
    let schema_end = find_next_at_or_above_indent(lines, schema_index + 1, json_end, schema_indent);
    lines[schema_index + 1..schema_end]
        .iter()
        .find(|line| yaml_key(&line.text).is_some_and(|found| found == "$ref"))
        .and_then(|line| yaml_value(&line.text))
        .map(clean_yaml_scalar)
        .and_then(|value| component_schema_ref(&value))
}
