use crate::error::OpenApiSourceError;
use crate::yaml::{
    LogicalLine, find_next_at_or_above_indent, list_item_yaml_value, scalar_value_at_indent,
    top_level_block, yaml_key,
};
pub(crate) const BEARER_SECURITY_SCHEME: &str = "bearerAuth";

pub(crate) fn validate_bearer_security_scheme(
    document_path: &str,
    lines: &[LogicalLine],
) -> Result<(), OpenApiSourceError> {
    let components_range = top_level_block(lines, "components").ok_or_else(|| {
        OpenApiSourceError::MissingBearerSecurityScheme {
            path: document_path.into(),
        }
    })?;
    let security_schemes_index = lines[components_range.clone()]
        .iter()
        .position(|line| {
            line.indent == 2 && yaml_key(&line.text).is_some_and(|key| key == "securitySchemes")
        })
        .map(|offset| components_range.start + offset)
        .ok_or_else(|| OpenApiSourceError::MissingBearerSecurityScheme {
            path: document_path.into(),
        })?;
    let security_schemes_end = find_next_at_or_above_indent(
        lines,
        security_schemes_index + 1,
        components_range.end,
        lines[security_schemes_index].indent,
    );
    let scheme_index = lines[security_schemes_index + 1..security_schemes_end]
        .iter()
        .position(|line| {
            line.indent == 4
                && yaml_key(&line.text).is_some_and(|key| key == BEARER_SECURITY_SCHEME)
        })
        .map(|offset| security_schemes_index + 1 + offset)
        .ok_or_else(|| OpenApiSourceError::MissingBearerSecurityScheme {
            path: document_path.into(),
        })?;
    let scheme_end = find_next_at_or_above_indent(lines, scheme_index + 1, security_schemes_end, 4);
    let type_value = scalar_value_at_indent(lines, scheme_index + 1..scheme_end, 6, "type");
    if type_value.as_deref() != Some("http") {
        return Err(OpenApiSourceError::InvalidBearerSecurityScheme {
            path: document_path.into(),
            reason: "bearerAuth type must be http".into(),
        });
    }
    let scheme_value = scalar_value_at_indent(lines, scheme_index + 1..scheme_end, 6, "scheme");
    if scheme_value.as_deref() != Some("bearer") {
        return Err(OpenApiSourceError::InvalidBearerSecurityScheme {
            path: document_path.into(),
            reason: "bearerAuth scheme must be bearer".into(),
        });
    }
    Ok(())
}

pub(crate) fn operation_security_requires(
    lines: &[LogicalLine],
    range: std::ops::Range<usize>,
    scheme: &str,
) -> bool {
    let Some(security_index) = lines[range.clone()].iter().position(|line| {
        line.indent > 4 && yaml_key(&line.text).is_some_and(|key| key == "security")
    }) else {
        return false;
    };
    let security_index = range.start + security_index;
    let security_end = find_next_at_or_above_indent(
        lines,
        security_index + 1,
        range.end,
        lines[security_index].indent,
    );
    lines[security_index + 1..security_end].iter().any(|line| {
        line.text.starts_with("- ") && list_item_yaml_value(&line.text, scheme).is_some()
    })
}
