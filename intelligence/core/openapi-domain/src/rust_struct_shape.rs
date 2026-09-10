use crate::rust_lexer::rust_code_without_comments_and_literals;
use crate::rust_source::find_public_rust_struct;
use crate::rust_status_mapping::balanced_brace_end;
use crate::schema_match::{canonical_rust_type, strip_option_type};
use crate::schema_shape::{RustFieldShape, RustStructShape};
use crate::serde_attribute::{
    apply_supported_serde_rename_all, serde_alias_attribute, serde_default_attribute,
    serde_flatten_attribute, serde_rename_all_attribute, serde_rename_attribute,
    serde_skip_attribute, serde_skip_serializing_if_attribute, supported_serde_rename_all_rule,
    unsupported_serde_field_attribute, unsupported_serde_struct_attribute,
};
use std::collections::BTreeMap;
use std::collections::BTreeSet;

pub(crate) fn parse_rust_struct_fields(
    contents: &str,
    rust_struct: &str,
) -> Result<Option<RustStructShape>, String> {
    let code = rust_code_without_comments_and_literals(contents);
    let Some(struct_index) = find_public_rust_struct(&code, rust_struct) else {
        return Ok(None);
    };
    let Some(relative_brace_index) = code[struct_index..].find('{') else {
        return Err(format!("{rust_struct} struct declaration must have a body"));
    };
    let brace_index = struct_index + relative_brace_index;
    let Some(block_end) = balanced_brace_end(&code, brace_index) else {
        return Err(format!(
            "{rust_struct} struct declaration has unbalanced braces"
        ));
    };
    let mut fields = BTreeMap::new();
    let mut required_fields = BTreeSet::new();
    let rename_all = serde_rename_all_before_item(contents, struct_index, rust_struct)?;
    let source_body = &contents[brace_index + 1..block_end];
    let code_body = &code[brace_index + 1..block_end];
    let mut pending_serde_rename = None::<String>;
    let mut pending_serde_skip = false;
    let mut pending_serde_flatten = false;
    let mut pending_serde_skip_serializing_if = false;
    let mut pending_serde_default = false;
    let mut pending_serde_alias = None::<String>;
    let mut pending_unsupported_serde = None::<String>;
    for (source_line, code_line) in source_body.lines().zip(code_body.lines()) {
        let source_trimmed = source_line.trim();
        let code_trimmed = code_line.trim();
        if source_trimmed.starts_with("#[") {
            if let Some(unsupported) = unsupported_serde_field_attribute(source_trimmed) {
                pending_unsupported_serde.get_or_insert(unsupported);
            }
            if let Some(rename) = serde_rename_attribute(source_trimmed) {
                pending_serde_rename = Some(rename);
            }
            if serde_skip_attribute(source_trimmed) {
                pending_serde_skip = true;
            }
            if serde_flatten_attribute(source_trimmed) {
                pending_serde_flatten = true;
            }
            if serde_skip_serializing_if_attribute(source_trimmed) {
                pending_serde_skip_serializing_if = true;
            }
            if serde_default_attribute(source_trimmed) {
                pending_serde_default = true;
            }
            if let Some(alias) = serde_alias_attribute(source_trimmed) {
                pending_serde_alias = Some(alias);
            }
            continue;
        }
        if let Some((field_name, field)) = parse_rust_public_field(code_trimmed) {
            if let Some(unsupported) = pending_unsupported_serde.as_deref() {
                return Err(format!(
                    "unsupported serde {unsupported} on field {field_name}"
                ));
            }
            if pending_serde_flatten {
                return Err(format!("unsupported serde flatten on field {field_name}"));
            }
            if let Some(alias) = pending_serde_alias.as_deref() {
                return Err(format!(
                    "unsupported serde alias {alias} on field {field_name}"
                ));
            }
            if pending_serde_skip {
                pending_serde_rename = None;
                pending_serde_skip = false;
                pending_serde_flatten = false;
                pending_serde_skip_serializing_if = false;
                pending_serde_default = false;
                pending_serde_alias = None;
                pending_unsupported_serde = None;
                continue;
            }
            let field_name = pending_serde_rename
                .take()
                .or_else(|| {
                    rename_all
                        .as_deref()
                        .map(|style| apply_supported_serde_rename_all(&field_name, style))
                })
                .unwrap_or(field_name);
            let field = if pending_serde_skip_serializing_if || pending_serde_default {
                RustFieldShape {
                    required: false,
                    rust_type: field.rust_type,
                }
            } else {
                field
            };
            if field.required {
                required_fields.insert(field_name.clone());
            }
            fields.insert(field_name, field);
            pending_serde_skip_serializing_if = false;
            pending_serde_default = false;
            pending_serde_alias = None;
            pending_unsupported_serde = None;
        } else if !code_trimmed.is_empty() {
            pending_serde_rename = None;
            pending_serde_skip = false;
            pending_serde_flatten = false;
            pending_serde_skip_serializing_if = false;
            pending_serde_default = false;
            pending_serde_alias = None;
            pending_unsupported_serde = None;
        }
    }
    Ok(Some(RustStructShape {
        fields,
        required_fields,
    }))
}

fn serde_rename_all_before_item(
    source: &str,
    item_index: usize,
    rust_struct: &str,
) -> Result<Option<String>, String> {
    let item_line_start = source[..item_index]
        .rfind('\n')
        .map_or(0usize, |index| index + 1);
    let mut rename_all = None;
    for line in source[..item_line_start].lines().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !trimmed.starts_with("#[") {
            break;
        }
        if let Some(unsupported) = unsupported_serde_struct_attribute(trimmed) {
            return Err(format!(
                "unsupported serde {unsupported} on struct {rust_struct}"
            ));
        }
        if let Some(found) = serde_rename_all_attribute(trimmed) {
            if !supported_serde_rename_all_rule(&found) {
                return Err(format!("unsupported serde rename_all rule {found}"));
            }
            rename_all = Some(found);
        }
    }
    Ok(rename_all)
}

fn parse_rust_public_field(line: &str) -> Option<(String, RustFieldShape)> {
    let visible = line.split("//").next()?.trim();
    let field = visible.strip_prefix("pub ")?;
    let (name, field_type) = field.split_once(':')?;
    let name = name.trim();
    if name.is_empty()
        || !name
            .bytes()
            .all(|byte| byte == b'_' || byte.is_ascii_alphanumeric())
    {
        return None;
    }
    let rust_type = canonical_rust_type(field_type.trim().trim_end_matches(',').trim());
    Some((
        name.to_string(),
        RustFieldShape {
            required: strip_option_type(&rust_type).is_none(),
            rust_type,
        },
    ))
}
