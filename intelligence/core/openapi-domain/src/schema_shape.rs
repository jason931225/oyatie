use crate::error::OpenApiSourceError;
use crate::schema_match::canonical_rust_type;
use crate::yaml::{
    LogicalLine, clean_yaml_scalar, component_schema_ref, find_next_at_or_above_indent,
    list_item_scalar, logical_lines, scalar_value_at_indent, top_level_block, yaml_key, yaml_value,
};
use std::collections::BTreeMap;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OpenApiSchemaShape {
    pub(crate) schema_name: String,
    pub(crate) contract_path: String,
    pub(crate) properties: BTreeMap<String, OpenApiSchemaProperty>,
    pub(crate) required: BTreeSet<String>,
    pub(crate) unsupported_keywords: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RustStructShape {
    pub(crate) fields: BTreeMap<String, RustFieldShape>,
    pub(crate) required_fields: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OpenApiSchemaProperty {
    pub(crate) type_name: Option<String>,
    pub(crate) format: Option<String>,
    pub(crate) rust_type: Option<String>,
    pub(crate) ref_schema: Option<String>,
    pub(crate) items_ref_schema: Option<String>,
    pub(crate) items_type_name: Option<String>,
    pub(crate) items_format: Option<String>,
    pub(crate) nullable: bool,
    pub(crate) unsupported_keywords: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RustFieldShape {
    pub(crate) rust_type: String,
    pub(crate) required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ExpectedOpenApiScalar {
    pub(crate) type_name: &'static str,
    pub(crate) format: Option<&'static str>,
}

pub(crate) fn collect_component_schemas(
    path: &str,
    contents: &str,
) -> Result<Vec<OpenApiSchemaShape>, OpenApiSourceError> {
    let lines = logical_lines(contents);
    let Some(components_range) = top_level_block(&lines, "components") else {
        return Ok(Vec::new());
    };
    let Some(schemas_index) = lines[components_range.clone()].iter().position(|line| {
        line.indent == 2 && yaml_key(&line.text).is_some_and(|found| found == "schemas")
    }) else {
        return Ok(Vec::new());
    };
    let schemas_index = components_range.start + schemas_index;
    let schemas_indent = lines[schemas_index].indent;
    let schemas_end = find_next_at_or_above_indent(
        &lines,
        schemas_index + 1,
        components_range.end,
        schemas_indent,
    );
    let schema_indent = schemas_indent + 2;
    let mut schemas = Vec::new();
    let mut index = schemas_index + 1;
    while index < schemas_end {
        let line = &lines[index];
        if line.indent != schema_indent {
            index += 1;
            continue;
        }
        let Some(schema_name) = yaml_key(&line.text).map(str::to_string) else {
            index += 1;
            continue;
        };
        let schema_end =
            find_next_at_or_above_indent(&lines, index + 1, schemas_end, schema_indent);
        let schema_body_range = index + 1..schema_end;
        schemas.push(OpenApiSchemaShape {
            schema_name,
            contract_path: path.into(),
            properties: collect_schema_properties(&lines, schema_body_range.clone()),
            required: collect_schema_required_names(&lines, schema_body_range.clone()),
            unsupported_keywords: unsupported_schema_keywords(
                &lines,
                schema_body_range,
                schema_indent + 2,
            ),
        });
        index = schema_end;
    }
    Ok(schemas)
}

fn collect_schema_properties(
    lines: &[LogicalLine],
    schema_range: std::ops::Range<usize>,
) -> BTreeMap<String, OpenApiSchemaProperty> {
    let Some(properties_index) = lines[schema_range.clone()]
        .iter()
        .position(|line| yaml_key(&line.text).is_some_and(|found| found == "properties"))
    else {
        return BTreeMap::new();
    };
    let properties_index = schema_range.start + properties_index;
    let properties_indent = lines[properties_index].indent;
    let property_indent = properties_indent + 2;
    let properties_end = find_next_at_or_above_indent(
        lines,
        properties_index + 1,
        schema_range.end,
        properties_indent,
    );
    let mut properties = BTreeMap::new();
    let mut index = properties_index + 1;
    while index < properties_end {
        let line = &lines[index];
        if line.indent != property_indent {
            index += 1;
            continue;
        }
        let Some(property_name) = yaml_key(&line.text).map(str::to_string) else {
            index += 1;
            continue;
        };
        let property_end =
            find_next_at_or_above_indent(lines, index + 1, properties_end, property_indent);
        properties.insert(
            property_name,
            collect_schema_property(lines, index + 1..property_end, property_indent + 2),
        );
        index = property_end;
    }
    properties
}

fn collect_schema_property(
    lines: &[LogicalLine],
    property_range: std::ops::Range<usize>,
    field_indent: usize,
) -> OpenApiSchemaProperty {
    OpenApiSchemaProperty {
        type_name: scalar_value_at_indent(lines, property_range.clone(), field_indent, "type"),
        format: scalar_value_at_indent(lines, property_range.clone(), field_indent, "format"),
        nullable: scalar_value_at_indent(lines, property_range.clone(), field_indent, "nullable")
            .is_some_and(|value| value == "true"),
        ref_schema: scalar_value_at_indent(lines, property_range.clone(), field_indent, "$ref")
            .and_then(|value| component_schema_ref(&value)),
        items_ref_schema: schema_items_ref(lines, property_range.clone(), field_indent),
        items_type_name: schema_items_scalar_value(
            lines,
            property_range.clone(),
            field_indent,
            "type",
        ),
        items_format: schema_items_scalar_value(
            lines,
            property_range.clone(),
            field_indent,
            "format",
        ),
        unsupported_keywords: unsupported_schema_keywords(
            lines,
            property_range.clone(),
            field_indent,
        ),
        rust_type: scalar_value_at_indent(
            lines,
            property_range,
            field_indent,
            "x-oyatie-rust-type",
        )
        .map(|value| canonical_rust_type(&value)),
    }
}

fn schema_items_ref(
    lines: &[LogicalLine],
    property_range: std::ops::Range<usize>,
    field_indent: usize,
) -> Option<String> {
    let items_index = lines[property_range.clone()].iter().position(|line| {
        line.indent == field_indent && yaml_key(&line.text).is_some_and(|found| found == "items")
    })? + property_range.start;
    let items_end = find_next_at_or_above_indent(
        lines,
        items_index + 1,
        property_range.end,
        lines[items_index].indent,
    );
    let item_field_indent = lines[items_index].indent + 2;
    lines[items_index + 1..items_end]
        .iter()
        .find(|line| {
            line.indent == item_field_indent
                && yaml_key(&line.text).is_some_and(|found| found == "$ref")
        })
        .and_then(|line| yaml_value(&line.text))
        .map(clean_yaml_scalar)
        .and_then(|value| component_schema_ref(&value))
}

fn schema_items_scalar_value(
    lines: &[LogicalLine],
    property_range: std::ops::Range<usize>,
    field_indent: usize,
    key: &str,
) -> Option<String> {
    let items_index = lines[property_range.clone()].iter().position(|line| {
        line.indent == field_indent && yaml_key(&line.text).is_some_and(|found| found == "items")
    })? + property_range.start;
    let items_end = find_next_at_or_above_indent(
        lines,
        items_index + 1,
        property_range.end,
        lines[items_index].indent,
    );
    scalar_value_at_indent(
        lines,
        items_index + 1..items_end,
        lines[items_index].indent + 2,
        key,
    )
}

fn unsupported_schema_keywords(
    lines: &[LogicalLine],
    range: std::ops::Range<usize>,
    indent: usize,
) -> Vec<String> {
    lines[range]
        .iter()
        .filter(|line| line.indent == indent)
        .filter_map(|line| yaml_key(&line.text))
        .filter(|key| unsupported_schema_keyword(key))
        .map(str::to_string)
        .collect()
}

fn unsupported_schema_keyword(key: &str) -> bool {
    matches!(
        key,
        "allOf"
            | "anyOf"
            | "oneOf"
            | "not"
            | "additionalProperties"
            | "patternProperties"
            | "dependentSchemas"
            | "unevaluatedProperties"
            | "if"
            | "then"
            | "else"
            | "contains"
            | "const"
    )
}

fn collect_schema_required_names(
    lines: &[LogicalLine],
    schema_range: std::ops::Range<usize>,
) -> BTreeSet<String> {
    let Some(required_index) = lines[schema_range.clone()]
        .iter()
        .position(|line| yaml_key(&line.text).is_some_and(|found| found == "required"))
    else {
        return BTreeSet::new();
    };
    let required_index = schema_range.start + required_index;
    let required_indent = lines[required_index].indent;
    let required_end =
        find_next_at_or_above_indent(lines, required_index + 1, schema_range.end, required_indent);
    lines[required_index + 1..required_end]
        .iter()
        .filter(|line| line.indent > required_indent)
        .filter_map(|line| list_item_scalar(&line.text))
        .collect()
}
