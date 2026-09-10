use crate::error::OpenApiSourceError;
use crate::parameter::parameter_name;
use crate::yaml::{
    LogicalLine, clean_yaml_scalar, find_next_at_or_above_indent, yaml_key, yaml_value,
};
use data_boundary_kernel::parse_data_class_label;

pub(crate) fn validate_data_class_annotations(
    document_path: &str,
    lines: &[LogicalLine],
) -> Result<usize, OpenApiSourceError> {
    let mut annotations_checked = 0usize;
    for (index, line) in lines.iter().enumerate() {
        if yaml_key(&line.text).is_some_and(|key| key == "properties") {
            annotations_checked +=
                validate_schema_properties_data_class(document_path, lines, index)?;
        }
        if yaml_key(&line.text).is_some_and(|key| key == "parameters") {
            annotations_checked += validate_parameters_data_class(document_path, lines, index)?;
        }
    }
    Ok(annotations_checked)
}

fn validate_schema_properties_data_class(
    document_path: &str,
    lines: &[LogicalLine],
    properties_index: usize,
) -> Result<usize, OpenApiSourceError> {
    let properties_indent = lines[properties_index].indent;
    let property_indent = properties_indent + 2;
    let end =
        find_next_at_or_above_indent(lines, properties_index + 1, lines.len(), properties_indent);
    let schema_name = parent_schema_name(lines, properties_index, properties_indent);
    let mut annotations_checked = 0usize;
    let mut index = properties_index + 1;
    while index < end {
        let line = &lines[index];
        if line.indent != property_indent {
            index += 1;
            continue;
        }
        let Some(property_name) = yaml_key(&line.text).map(str::to_string) else {
            index += 1;
            continue;
        };
        let property_end = find_next_at_or_above_indent(lines, index + 1, end, property_indent);
        let location = match &schema_name {
            Some(schema_name) => format!("schema {schema_name}.{property_name}"),
            None => format!("schema property {property_name}"),
        };
        annotations_checked += validate_data_class_annotation(
            document_path,
            &location,
            lines,
            index + 1..property_end,
        )?;
        index = property_end;
    }
    Ok(annotations_checked)
}

fn validate_parameters_data_class(
    document_path: &str,
    lines: &[LogicalLine],
    parameters_index: usize,
) -> Result<usize, OpenApiSourceError> {
    let parameters_indent = lines[parameters_index].indent;
    let end =
        find_next_at_or_above_indent(lines, parameters_index + 1, lines.len(), parameters_indent);
    let mut annotations_checked = 0usize;
    let mut index = parameters_index + 1;
    while index < end {
        let line = &lines[index];
        if !line.text.starts_with("- ") {
            index += 1;
            continue;
        }
        let parameter_end = find_next_at_or_above_indent(lines, index + 1, end, line.indent);
        if let Some(parameter_name) = parameter_name(lines, index, parameter_end) {
            let location = format!("parameter {parameter_name}");
            annotations_checked += validate_data_class_annotation(
                document_path,
                &location,
                lines,
                index..parameter_end,
            )?;
        }
        index = parameter_end;
    }
    Ok(annotations_checked)
}

fn validate_data_class_annotation(
    document_path: &str,
    location: &str,
    lines: &[LogicalLine],
    range: std::ops::Range<usize>,
) -> Result<usize, OpenApiSourceError> {
    let value = lines[range].iter().find_map(|line| {
        if yaml_key(&line.text).is_some_and(|key| key == "x-oyatie-data-class") {
            yaml_value(&line.text).map(clean_yaml_scalar)
        } else {
            None
        }
    });
    let Some(data_class) = value else {
        return Err(OpenApiSourceError::MissingDataClassAnnotation {
            path: document_path.into(),
            location: location.into(),
        });
    };
    if !valid_data_class(&data_class) {
        return Err(OpenApiSourceError::InvalidDataClassAnnotation {
            path: document_path.into(),
            location: location.into(),
            data_class,
        });
    }
    Ok(1)
}

fn parent_schema_name(
    lines: &[LogicalLine],
    properties_index: usize,
    properties_indent: usize,
) -> Option<String> {
    if properties_indent < 2 {
        return None;
    }
    lines[..properties_index]
        .iter()
        .rev()
        .find(|line| line.indent + 2 == properties_indent)
        .and_then(|line| yaml_key(&line.text))
        .map(str::to_string)
}

fn valid_data_class(value: &str) -> bool {
    parse_data_class_label(value).is_some()
}
