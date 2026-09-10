//! The typed IR: precedence decided by the tree, visibility by position, docs carried through.

use port_engine_api::{RegionId, Renderer, TargetIr};
use port_engine_rust_ir::{
    BinaryOp, EmptyRenderer, Receiver, RustExpr, RustFn, RustIr, RustItem, RustParam, RustRenderer,
    RustStmt, RustType, StructShape, UnaryOp, Visibility, w0_ready,
};

fn render(items: Vec<RustItem>) -> String {
    let mut ir = RustIr::new(&["r"]);
    ir.set_items("r", items).expect("region is declared");
    let out = RustRenderer::new()
        .render_rust_ir(&ir)
        .expect("items must assemble and format");
    String::from_utf8(out[&RegionId("r".into())].clone()).expect("utf-8")
}

fn binary(op: BinaryOp, lhs: RustExpr, rhs: RustExpr) -> RustExpr {
    RustExpr::Binary {
        op,
        lhs: Box::new(lhs),
        rhs: Box::new(rhs),
    }
}

fn path(name: &str) -> RustExpr {
    RustExpr::Path(name.to_owned())
}

fn free_fn(name: &str, body: Vec<RustStmt>) -> RustItem {
    RustItem::Function(RustFn {
        docs: Vec::new(),
        vis: Visibility::Public,
        name: name.to_owned(),
        receiver: None,
        params: vec![
            RustParam {
                name: "a".into(),
                ty: RustType::path("i64"),
            },
            RustParam {
                name: "b".into(),
                ty: RustType::path("i64"),
            },
            RustParam {
                name: "c".into(),
                ty: RustType::path("i64"),
            },
        ],
        ret: Some(RustType::path("i64")),
        body: Some(body),
    })
}

#[test]
fn claims_readiness() {
    assert!(w0_ready());
}

// ------------------------------------------------------------------------------- precedence ---

/// The whole reason the IR stopped being text. A builder that cannot see its own nesting has to
/// parenthesise everything; a tree only brackets what the grammar would otherwise reassociate.
#[test]
fn tighter_binding_operand_needs_no_parentheses() {
    let expr = binary(
        BinaryOp::Add,
        path("a"),
        binary(BinaryOp::Mul, path("b"), path("c")),
    );
    let text = render(vec![free_fn("f", vec![RustStmt::Tail(expr)])]);
    assert!(text.contains("a + b * c"), "{text}");
}

#[test]
fn looser_binding_operand_keeps_its_parentheses() {
    let expr = binary(
        BinaryOp::Mul,
        binary(BinaryOp::Add, path("a"), path("b")),
        path("c"),
    );
    let text = render(vec![free_fn("f", vec![RustStmt::Tail(expr)])]);
    assert!(text.contains("(a + b) * c"), "{text}");
}

/// Associativity, not precedence: `a - (b - c)` and `(a - b) - c` are different values and only
/// one of them needs its brackets kept.
#[test]
fn right_operand_of_equal_precedence_keeps_its_parentheses() {
    let right_nested = binary(
        BinaryOp::Sub,
        path("a"),
        binary(BinaryOp::Sub, path("b"), path("c")),
    );
    assert!(render(vec![free_fn("f", vec![RustStmt::Tail(right_nested)])]).contains("a - (b - c)"),);

    let left_nested = binary(
        BinaryOp::Sub,
        binary(BinaryOp::Sub, path("a"), path("b")),
        path("c"),
    );
    let text = render(vec![free_fn("f", vec![RustStmt::Tail(left_nested)])]);
    assert!(text.contains("a - b - c"), "{text}");
}

/// A prefix operator binds tighter than any binary one, so a binary operand under it must be
/// bracketed or the negation applies to the wrong subtree.
#[test]
fn prefix_operator_brackets_a_binary_operand() {
    let expr = RustExpr::Unary {
        op: UnaryOp::Neg,
        operand: Box::new(binary(BinaryOp::Add, path("a"), path("b"))),
    };
    let text = render(vec![free_fn("f", vec![RustStmt::Tail(expr)])]);
    assert!(text.contains("-(a + b)"), "{text}");
}

// --------------------------------------------------------------------------------- structure ---

/// A trait item may not carry a visibility qualifier. The previous IR concatenated a `"pub "`
/// prefix into the trait body, which `syn` accepts and `rustc` rejects — the defect that made the
/// compile proof necessary in the first place.
#[test]
fn a_trait_method_carries_no_visibility() {
    let text = render(vec![RustItem::Trait {
        docs: vec![" Anything that can render its own name.".into()],
        vis: Visibility::Public,
        name: "Named".into(),
        supertraits: Vec::new(),
        methods: vec![RustFn {
            docs: Vec::new(),
            vis: Visibility::Inherited,
            name: "name".into(),
            receiver: Some(Receiver::Shared),
            params: Vec::new(),
            ret: Some(RustType::path("String")),
            body: None,
        }],
    }]);
    assert!(text.contains("pub trait Named"), "{text}");
    assert!(text.contains("fn name(&self) -> String;"), "{text}");
    assert!(
        !text.contains("pub fn name"),
        "a trait item must not be `pub`: {text}"
    );
}

/// A mutating method needs an exclusive receiver, and the IR can now say so — the previous
/// renderer emitted `&self` for every method, which no mutating implementation can satisfy.
#[test]
fn a_receiver_mode_is_a_choice_the_ir_can_express() {
    for (receiver, expected) in [
        (Receiver::Shared, "fn m(&self)"),
        (Receiver::Exclusive, "fn m(&mut self)"),
        (Receiver::Owned, "fn m(self)"),
    ] {
        let text = render(vec![RustItem::Trait {
            docs: Vec::new(),
            vis: Visibility::Public,
            name: "T".into(),
            supertraits: Vec::new(),
            methods: vec![RustFn {
                docs: Vec::new(),
                vis: Visibility::Inherited,
                name: "m".into(),
                receiver: Some(receiver),
                params: Vec::new(),
                ret: None,
                body: None,
            }],
        }]);
        assert!(text.contains(expected), "{expected} missing from: {text}");
    }
}

/// Documentation survives the translation. It used to be dropped entirely — 17 doc-comment lines
/// in the fixture corpus reached the emitted output as none.
#[test]
fn doc_comments_render_as_doc_comments() {
    let text = render(vec![RustItem::Const {
        docs: vec![" Bounds the retry loop.".into()],
        vis: Visibility::Public,
        name: "MAX_RETRIES".into(),
        ty: RustType::path("i64"),
        value: "3".into(),
    }]);
    assert!(text.contains("/// Bounds the retry loop."), "{text}");
    assert!(text.contains("pub const MAX_RETRIES: i64 = 3;"), "{text}");
}

#[test]
fn a_struct_renders_fields_and_an_inherent_impl() {
    let text = render(vec![RustItem::Struct {
        docs: Vec::new(),
        vis: Visibility::Public,
        name: "Point".into(),
        shape: StructShape::Named(vec![port_engine_rust_ir::RustField {
            docs: vec![" The horizontal coordinate.".into()],
            vis: Visibility::Public,
            name: "x".into(),
            ty: RustType::path("i64"),
        }]),
        methods: vec![RustFn {
            docs: Vec::new(),
            vis: Visibility::Public,
            name: "area".into(),
            receiver: Some(Receiver::Shared),
            params: Vec::new(),
            ret: Some(RustType::path("i64")),
            body: Some(vec![RustStmt::Tail(RustExpr::Todo)]),
        }],
    }]);
    assert!(text.contains("pub struct Point"), "{text}");
    assert!(text.contains("/// The horizontal coordinate."), "{text}");
    assert!(text.contains("impl Point"), "{text}");
    assert!(text.contains("pub fn area(&self) -> i64"), "{text}");
}

// ---------------------------------------------------------------------------------- refusals ---

#[test]
fn an_undeclared_region_refuses() {
    let mut ir = RustIr::new(&["declared"]);
    assert!(ir.set_items("other", Vec::new()).is_err());
}

/// A type spelling the target cannot parse fails HERE, where it can name the spelling, rather than
/// downstream where the failure is a compiler error about generated code.
#[test]
fn an_unparseable_type_refuses_by_name() {
    let mut ir = RustIr::new(&["r"]);
    ir.set_items(
        "r",
        vec![RustItem::TypeAlias {
            docs: Vec::new(),
            vis: Visibility::Public,
            name: "Bad".into(),
            ty: RustType::path("map[string]int"),
        }],
    )
    .expect("declared");
    let err = RustRenderer::new()
        .render_rust_ir(&ir)
        .expect_err("a source type spelling is not a target type");
    assert!(format!("{err}").contains("map[string]int"), "{err}");
}

#[test]
fn the_trait_object_render_path_is_fail_closed() {
    let ir = RustIr::new(&["r"]);
    assert!(
        RustRenderer::new().render(&ir).is_err(),
        "emitting through the untyped path would claim a formatting run that never happened"
    );
}

#[test]
fn the_empty_renderer_matches_declared_regions() {
    let ir = RustIr::new(&["root"]);
    let out = EmptyRenderer::new("fmt-stub-v0")
        .render(&ir)
        .expect("empty stub must succeed");
    assert_eq!(out.len(), 1);
    assert!(out[&RegionId("root".into())].is_empty());
}

/// The axis must name a formatter, not a label somebody typed. The version it carries is
/// `CARGO_PKG_VERSION` — this workspace's, not the one `Cargo.lock` resolves for prettyplease — so
/// it does NOT move when the formatter does. Correcting that moves a receipt axis: its own lane.
#[test]
fn the_formatter_identity_names_the_formatter_but_not_its_version() {
    let digest = RustRenderer::new().formatter_digest();
    assert!(digest.0.contains("prettyplease"), "{}", digest.0);
    assert!(
        digest.0.chars().any(|c| c.is_ascii_digit()),
        "the identity must carry a version: {}",
        digest.0
    );
}

/// An embedded interface becomes a SUPERTRAIT, not a copy of its methods.
///
/// The difference is what the trait REQUIRES. Flattening the embedded interface's methods into the
/// outer trait compiles and means something weaker — a type could satisfy the outer trait without
/// satisfying the embedded one, which the source does not allow.
#[test]
fn an_embedded_interface_becomes_a_supertrait() {
    let text = render(vec![RustItem::Trait {
        docs: Vec::new(),
        vis: Visibility::Public,
        name: "Job".into(),
        supertraits: vec![RustType::path("Runner"), RustType::path("Describer")],
        methods: Vec::new(),
    }]);
    assert!(text.contains("trait Job: Runner + Describer"), "{text}");
}
