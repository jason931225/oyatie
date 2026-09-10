use super::channel_literal_violations;

/// A synthetic channel throughout: writing the live one here would make this
/// file the very duplicate the rule refuses.
#[test]
fn a_rust_literal_equal_to_the_declared_channel_is_refused() {
    let source = "const A: &str = \"9.9.9\";\nconst B: &str = \"9.9.10\";\n";
    let violations = channel_literal_violations("9.9.9", "t.rs", source);
    assert_eq!(
        violations
            .iter()
            .map(|v| v.split(':').nth(1).unwrap())
            .collect::<Vec<_>>(),
        ["1"],
        "only the line equal to the channel is refused: {violations:?}"
    );
}

#[test]
fn a_literal_that_differs_from_the_channel_is_an_oracle_and_is_untouched() {
    let source = "assert_eq!(delta(\"9.9.8\", \"9.9.10\"), Forward);\n";
    let violations = channel_literal_violations("9.9.9", "t.rs", source);
    assert!(
        violations.is_empty(),
        "a comparison oracle must survive the rule: {violations:?}"
    );
}

#[test]
fn a_channel_embedded_in_a_byte_string_declaration_is_still_a_duplicate() {
    let source = "const F: &[u8] = b\"[toolchain]\\nchannel = \\\"9.9.9\\\"\\n\";\n";
    let violations = channel_literal_violations("9.9.9", "h.rs", source);
    assert!(
        !violations.is_empty(),
        "an embedded declaration is the clearest duplicate: {violations:?}"
    );
}

#[test]
fn a_source_that_derives_the_channel_is_clean() {
    let source = "let channel = declared_channel(include_str!(\"../rust-toolchain.toml\"))?;\n";
    let violations = channel_literal_violations("9.9.9", "d.rs", source);
    assert!(violations.is_empty(), "{violations:?}");
}
