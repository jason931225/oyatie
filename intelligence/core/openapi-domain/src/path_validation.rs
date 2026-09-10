use crate::error::OpenApiSourceError;
use crate::ingress::validate_mutating_operation_ingress;
use crate::operation::path_item_operation_blocks;
use crate::parameter::{
    OperationParameter, merged_operation_parameters, operation_parameters, path_item_parameters,
    validate_path_template_parameters,
};
use crate::yaml::{
    LogicalLine, clean_yaml_scalar, find_next_at_or_above_indent, yaml_key, yaml_value,
};
const FIXED_OPERATION_METHODS: [&str; 9] = [
    "get", "put", "post", "delete", "options", "head", "patch", "trace", "query",
];

pub(crate) fn validate_paths(
    document_path: &str,
    lines: &[LogicalLine],
    range: std::ops::Range<usize>,
) -> Result<usize, OpenApiSourceError> {
    let mut operations_checked = 0usize;
    let mut path_item_seen = false;
    let mut index = range.start;
    while index < range.end {
        let line = &lines[index];
        if line.indent != 2 || !line.text.starts_with('/') {
            index += 1;
            continue;
        }
        let Some(api_path) = yaml_key(&line.text).map(str::to_string) else {
            index += 1;
            continue;
        };
        path_item_seen = true;
        let next_path_index = find_next_at_or_above_indent(lines, index + 1, range.end, 2);
        let operation_count =
            validate_path_item(document_path, &api_path, lines, index + 1..next_path_index)?;
        operations_checked += operation_count;
        index = next_path_index;
    }

    if !path_item_seen {
        return Err(OpenApiSourceError::MissingPathItem {
            path: document_path.into(),
        });
    }
    Ok(operations_checked)
}

fn validate_path_item(
    document_path: &str,
    api_path: &str,
    lines: &[LogicalLine],
    range: std::ops::Range<usize>,
) -> Result<usize, OpenApiSourceError> {
    let operation_blocks =
        path_item_operation_blocks(document_path, api_path, lines, range.clone())?;
    let inherited_parameters = path_item_parameters(lines, range.clone());
    for block in &operation_blocks {
        validate_operation(
            document_path,
            api_path,
            &block.method,
            lines,
            block.range.clone(),
            &inherited_parameters,
        )?;
    }

    if operation_blocks.is_empty() {
        return Err(OpenApiSourceError::MissingOperation {
            path: document_path.into(),
            api_path: api_path.into(),
        });
    }
    Ok(operation_blocks.len())
}

fn validate_operation(
    document_path: &str,
    api_path: &str,
    method: &str,
    lines: &[LogicalLine],
    range: std::ops::Range<usize>,
    inherited_parameters: &[OperationParameter],
) -> Result<(), OpenApiSourceError> {
    let has_operation_id = lines[range.clone()].iter().any(|line| {
        line.indent > 4
            && yaml_key(&line.text).is_some_and(|found| found == "operationId")
            && yaml_value(&line.text)
                .map(clean_yaml_scalar)
                .is_some_and(|value| !value.trim().is_empty())
    });
    if !has_operation_id {
        return Err(OpenApiSourceError::MissingOperationId {
            path: document_path.into(),
            api_path: api_path.into(),
            method: method.into(),
        });
    }

    let Some(responses_index) = lines[range.clone()].iter().position(|line| {
        line.indent > 4 && yaml_key(&line.text).is_some_and(|found| found == "responses")
    }) else {
        return Err(OpenApiSourceError::MissingResponses {
            path: document_path.into(),
            api_path: api_path.into(),
            method: method.into(),
        });
    };
    let responses_index = range.start + responses_index;
    let responses_end = find_next_at_or_above_indent(
        lines,
        responses_index + 1,
        range.end,
        lines[responses_index].indent,
    );
    let has_response = lines[responses_index + 1..responses_end]
        .iter()
        .any(|line| {
            line.indent > lines[responses_index].indent
                && yaml_key(&line.text).is_some_and(valid_response_key)
        });
    if !has_response {
        return Err(OpenApiSourceError::MissingResponses {
            path: document_path.into(),
            api_path: api_path.into(),
            method: method.into(),
        });
    }
    let parameters = merged_operation_parameters(
        inherited_parameters,
        operation_parameters(lines, range.clone()),
    );
    validate_path_template_parameters(document_path, api_path, method, &parameters)?;
    validate_mutating_operation_ingress(
        document_path,
        api_path,
        method,
        lines,
        range,
        &parameters,
    )?;
    Ok(())
}

pub(crate) fn operation_requires_mutating_ingress(method: &str) -> bool {
    // OpenAPI 3.2 `additionalOperations` custom methods are treated as
    // tenant-mutating until they become a fixed safe method in this validator.
    // This keeps custom verbs such as COPY on the same bearer/idempotency
    // contract as POST/PUT/PATCH/DELETE instead of accepting unaudited ingress.
    matches!(method, "post" | "put" | "patch" | "delete") || !is_fixed_operation_method(method)
}

pub(crate) fn is_fixed_operation_method(method: &str) -> bool {
    FIXED_OPERATION_METHODS.contains(&method)
}

pub(crate) fn collides_with_fixed_operation_method(method: &str) -> bool {
    FIXED_OPERATION_METHODS
        .iter()
        .any(|fixed| fixed.eq_ignore_ascii_case(method))
}

pub(crate) fn valid_response_key(key: &str) -> bool {
    key == "default" || valid_numeric_response_status(key) || valid_response_range_key(key)
}

pub(crate) fn valid_numeric_response_status(key: &str) -> bool {
    key.len() == 3 && key.bytes().all(|byte| byte.is_ascii_digit())
}

fn valid_response_range_key(key: &str) -> bool {
    matches!(key.as_bytes(), [b'1'..=b'5', b'X', b'X'])
}
