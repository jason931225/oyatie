use crate::error::OpenApiSourceError;
use crate::yaml::{
    LogicalLine, clean_yaml_scalar, find_next_at_or_above_indent, list_item_yaml_value,
    scalar_value_at_any_indent, yaml_key, yaml_value,
};
use std::collections::BTreeSet;

pub(crate) fn validate_path_template_parameters(
    document_path: &str,
    api_path: &str,
    method: &str,
    parameters: &[OperationParameter],
) -> Result<(), OpenApiSourceError> {
    let template_parameters = path_template_parameters(api_path);
    for parameter_name in &template_parameters {
        let Some(parameter) = parameters.iter().find(|parameter| {
            &parameter.name == parameter_name && parameter.location.as_deref() == Some("path")
        }) else {
            return Err(OpenApiSourceError::MissingPathTemplateParameter {
                path: document_path.into(),
                api_path: api_path.into(),
                method: method.into(),
                parameter: parameter_name.clone(),
            });
        };
        if parameter.required.as_deref() != Some("true") {
            return Err(OpenApiSourceError::InvalidPathTemplateParameter {
                path: document_path.into(),
                api_path: api_path.into(),
                method: method.into(),
                parameter: parameter.name.clone(),
                reason: "path template parameter must be required: true".into(),
            });
        }
        if parameter.schema_type.as_deref() != Some("string") {
            return Err(OpenApiSourceError::InvalidPathTemplateParameter {
                path: document_path.into(),
                api_path: api_path.into(),
                method: method.into(),
                parameter: parameter.name.clone(),
                reason: "path template parameter schema type must be string".into(),
            });
        }
    }

    for parameter in parameters
        .iter()
        .filter(|parameter| parameter.location.as_deref() == Some("path"))
    {
        if !template_parameters.contains(&parameter.name) {
            return Err(OpenApiSourceError::InvalidPathTemplateParameter {
                path: document_path.into(),
                api_path: api_path.into(),
                method: method.into(),
                parameter: parameter.name.clone(),
                reason: "path parameter is not present in the path template".into(),
            });
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OperationParameter {
    pub(crate) name: String,
    pub(crate) location: Option<String>,
    pub(crate) required: Option<String>,
    pub(crate) schema_type: Option<String>,
    pub(crate) min_length: Option<String>,
}

pub(crate) fn operation_parameters(
    lines: &[LogicalLine],
    range: std::ops::Range<usize>,
) -> Vec<OperationParameter> {
    let Some(parameters_index) = lines[range.clone()].iter().position(|line| {
        line.indent > 4 && yaml_key(&line.text).is_some_and(|key| key == "parameters")
    }) else {
        return Vec::new();
    };
    let parameters_index = range.start + parameters_index;
    parameters_at(lines, parameters_index, range.end)
}

pub(crate) fn path_item_parameters(
    lines: &[LogicalLine],
    range: std::ops::Range<usize>,
) -> Vec<OperationParameter> {
    let Some(parameters_index) = lines[range.clone()].iter().position(|line| {
        line.indent == 4 && yaml_key(&line.text).is_some_and(|key| key == "parameters")
    }) else {
        return Vec::new();
    };
    let parameters_index = range.start + parameters_index;
    parameters_at(lines, parameters_index, range.end)
}

fn parameters_at(
    lines: &[LogicalLine],
    parameters_index: usize,
    range_end: usize,
) -> Vec<OperationParameter> {
    let parameters_end = find_next_at_or_above_indent(
        lines,
        parameters_index + 1,
        range_end,
        lines[parameters_index].indent,
    );
    let mut parameters = Vec::new();
    let mut index = parameters_index + 1;
    while index < parameters_end {
        let line = &lines[index];
        if !line.text.starts_with("- ") {
            index += 1;
            continue;
        }
        let parameter_end =
            find_next_at_or_above_indent(lines, index + 1, parameters_end, line.indent);
        if let Some(name) = parameter_name(lines, index, parameter_end) {
            parameters.push(OperationParameter {
                name,
                location: scalar_value_at_any_indent(lines, index..parameter_end, "in"),
                required: scalar_value_at_any_indent(lines, index..parameter_end, "required"),
                schema_type: scalar_value_at_any_indent(lines, index..parameter_end, "type"),
                min_length: scalar_value_at_any_indent(lines, index..parameter_end, "minLength"),
            });
        }
        index = parameter_end;
    }
    parameters
}

pub(crate) fn merged_operation_parameters(
    inherited_parameters: &[OperationParameter],
    operation_parameters: Vec<OperationParameter>,
) -> Vec<OperationParameter> {
    let mut merged = inherited_parameters.to_vec();
    for parameter in operation_parameters {
        if let Some(location) = &parameter.location {
            merged.retain(|inherited| {
                !(inherited.name == parameter.name
                    && inherited.location.as_deref() == Some(location.as_str()))
            });
        }
        merged.push(parameter);
    }
    merged
}

pub(crate) fn parameter_name(
    lines: &[LogicalLine],
    item_index: usize,
    item_end: usize,
) -> Option<String> {
    if let Some(value) = list_item_yaml_value(&lines[item_index].text, "name") {
        return Some(value);
    }
    lines[item_index + 1..item_end].iter().find_map(|line| {
        if yaml_key(&line.text).is_some_and(|key| key == "name") {
            yaml_value(&line.text).map(clean_yaml_scalar)
        } else {
            None
        }
    })
}

fn path_template_parameters(api_path: &str) -> BTreeSet<String> {
    let mut parameters = BTreeSet::new();
    let mut remainder = api_path;
    while let Some(open_index) = remainder.find('{') {
        let after_open = &remainder[open_index + 1..];
        let Some(close_index) = after_open.find('}') else {
            break;
        };
        let parameter = after_open[..close_index].trim();
        if !parameter.is_empty() {
            parameters.insert(parameter.to_string());
        }
        remainder = &after_open[close_index + 1..];
    }
    parameters
}
