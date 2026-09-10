package main

import (
	"go/ast"
	"go/token"
)

// Statements.
//
// The body walk is deliberately SMALL and deliberately COMPLETE. Small, because only a few forms
// have a translation the engine can defend today. Complete, because everything else is still
// recorded — as an `unsupported` node naming the Go AST type it stands for — rather than dropped.
// A dropped construct would make an untranslatable function indistinguishable from an empty one,
// and the engine would emit a green, silently wrong body. Recorded, it becomes a refusal the
// transform can name.

func bodyNode(block *ast.BlockStmt, ctx *extractCtx) node {
	return node{Kind: kindBody, Children: statementNodes(block.List, ctx)}
}

// destructuringBind records `a, b := expr` as the names it binds and the expression they come
// from, in that order.
func destructuringBind(stmt *ast.AssignStmt, ctx *extractCtx) node {
	out := node{Kind: kindLetTuple}
	for _, lhs := range stmt.Lhs {
		name, ok := lhs.(*ast.Ident)
		if !ok {
			return unsupportedNode(stmt)
		}
		out.Children = append(out.Children, node{Kind: kindBind, Name: name.Name})
	}
	out.Children = append(out.Children, node{
		Kind:     kindValue,
		Children: []node{expressionNode(stmt.Rhs[0], ctx)},
	})
	return out
}

// localDeclaration records `var x T`, `var x = e` and `var x T = e` inside a body.
//
// A single-name spec only. A grouped `var ( a = 1; b = 2 )` is several bindings in one statement,
// and a statement list that silently gained entries would make the tail-expression position — which
// is decided by INDEX — mean something different from what the source wrote.
func localDeclaration(stmt *ast.DeclStmt, ctx *extractCtx) node {
	decl, ok := stmt.Decl.(*ast.GenDecl)
	if !ok || decl.Tok != token.VAR || len(decl.Specs) != 1 {
		return unsupportedNode(stmt)
	}
	spec, ok := decl.Specs[0].(*ast.ValueSpec)
	if !ok || len(spec.Names) != 1 || len(spec.Values) > 1 {
		return unsupportedNode(stmt)
	}

	out := node{
		Kind:  kindLet,
		Name:  spec.Names[0].Name,
		Flags: bindingFlags(spec.Names[0], ctx),
	}
	if spec.Type != nil {
		out.Type = typeTree(ctx.info.TypeOf(spec.Type))
	}
	if len(spec.Values) == 1 {
		out.Children = []node{expressionNode(spec.Values[0], ctx)}
	}
	return out
}

// bindingFlags reports what a binding needs from the target, which today is only whether it is
// written again.
//
// Observed rather than assumed. Every binding in the source is mutable and most are never written
// again: assuming mutable warns on each one, and assuming immutable fails to compile the first time
// one is assigned. Only the body knows which, so the body is asked.
func bindingFlags(name *ast.Ident, ctx *extractCtx) []string {
	object := ctx.info.Defs[name]
	if object == nil || ctx.assigned == nil || !ctx.assigned[object] {
		return nil
	}
	return []string{flagMutated}
}

func statementNodes(stmts []ast.Stmt, ctx *extractCtx) []node {
	if len(stmts) == 0 {
		return nil
	}
	out := make([]node, 0, len(stmts))
	for _, stmt := range stmts {
		out = append(out, statementNode(stmt, ctx))
	}
	return out
}

func statementNode(stmt ast.Stmt, ctx *extractCtx) node {
	switch typed := stmt.(type) {
	case *ast.ReturnStmt:
		return node{Kind: kindReturn, Children: expressionNodes(typed.Results, ctx)}

	case *ast.BlockStmt:
		return node{Kind: kindBlock, Children: statementNodes(typed.List, ctx)}

	case *ast.IfStmt:
		// An `if` with an init statement (`if x := f(); x != nil`) scopes a binding to the
		// condition, which Rust has no direct form for. Recorded as unsupported rather than
		// silently hoisted, because hoisting changes the binding's lifetime.
		if typed.Init != nil {
			return unsupportedNode(stmt)
		}
		children := []node{
			{Kind: "cond", Children: []node{expressionNode(typed.Cond, ctx)}},
			{Kind: kindThen, Children: statementNodes(typed.Body.List, ctx)},
		}
		if typed.Else != nil {
			children = append(children, node{
				Kind:     kindElse,
				Children: []node{statementNode(typed.Else, ctx)},
			})
		}
		return node{Kind: kindIf, Children: children}

	case *ast.AssignStmt:
		// A DESTRUCTURING bind takes several names from one expression. It is the shape every
		// fallible call in the source has, so it is recorded rather than refused — what the target
		// does with it is a rule, and a rule needs the shape to reach it.
		if typed.Tok == token.DEFINE && len(typed.Rhs) == 1 && len(typed.Lhs) > 1 {
			return destructuringBind(typed, ctx)
		}
		// The remaining multi-assignment and op-assign forms each carry a question — parallel
		// assignment order, read-modify-write — that needs a rule rather than a default.
		if len(typed.Lhs) != 1 || len(typed.Rhs) != 1 {
			return unsupportedNode(stmt)
		}
		switch typed.Tok {
		case token.DEFINE:
			name, ok := typed.Lhs[0].(*ast.Ident)
			if !ok {
				return unsupportedNode(stmt)
			}
			return node{
				Kind:     kindLet,
				Name:     name.Name,
				Children: []node{expressionNode(typed.Rhs[0], ctx)},
			}
		case token.ASSIGN:
			return node{
				Kind: kindAssign,
				Children: []node{
					expressionNode(typed.Lhs[0], ctx),
					expressionNode(typed.Rhs[0], ctx),
				},
			}
		default:
			return unsupportedNode(stmt)
		}

	case *ast.DeclStmt:
		return localDeclaration(typed, ctx)

	case *ast.ExprStmt:
		return node{Kind: kindExprStmt, Children: []node{expressionNode(typed.X, ctx)}}

	case *ast.BranchStmt:
		// `break` maps directly. `continue` does NOT, because a three-clause loop lowers to a
		// `while` whose post-statement a `continue` would skip — a different program. `goto` and
		// labelled breaks have no target form at all.
		if typed.Tok == token.BREAK && typed.Label == nil {
			return node{Kind: kindBreak}
		}
		return unsupportedNode(stmt)

	case *ast.ForStmt:
		return forNode(typed, ctx)

	case *ast.RangeStmt:
		return rangeNode(typed, ctx)

	case *ast.SwitchStmt:
		return switchNode(typed, ctx)

	default:
		return unsupportedNode(stmt)
	}
}

// forNode records a three-clause or condition-only `for`.
//
// The clauses are recorded SEPARATELY rather than pre-lowered, because which target loop they
// deserve is a translation decision: an ascending integer counter is a range, and anything else is
// a `while` whose post-statement has to run on every path.
func forNode(stmt *ast.ForStmt, ctx *extractCtx) node {
	out := node{Kind: kindFor}
	if stmt.Init != nil {
		out.Children = append(out.Children, node{
			Kind:     kindInit,
			Children: []node{statementNode(stmt.Init, ctx)},
		})
	}
	if stmt.Cond != nil {
		out.Children = append(out.Children, node{
			Kind:     kindCond,
			Children: []node{expressionNode(stmt.Cond, ctx)},
		})
	}
	if stmt.Post != nil {
		out.Children = append(out.Children, node{
			Kind:     kindPost,
			Children: []node{statementNode(stmt.Post, ctx)},
		})
	}
	out.Children = append(out.Children, node{
		Kind:     kindThen,
		Children: statementNodes(stmt.Body.List, ctx),
	})
	return out
}

// rangeNode records a `range` loop, with the key and value names it binds.
func rangeNode(stmt *ast.RangeStmt, ctx *extractCtx) node {
	out := node{Kind: kindRange}
	out.Attrs = withAttr(out.Attrs, "key", identName(stmt.Key))
	out.Attrs = withAttr(out.Attrs, "value", identName(stmt.Value))
	out.Children = append(out.Children,
		node{Kind: "over", Children: []node{expressionNode(stmt.X, ctx)}},
		node{Kind: kindThen, Children: statementNodes(stmt.Body.List, ctx)},
	)
	return out
}

// switchNode records an expression switch.
//
// A switch with an init statement, or a TYPE switch, is not recorded as a switch at all: the first
// scopes a binding to the switch and the second dispatches on dynamic type, and neither has a
// target form that a value match reproduces.
func switchNode(stmt *ast.SwitchStmt, ctx *extractCtx) node {
	if stmt.Init != nil {
		return unsupportedNode(stmt)
	}
	out := node{Kind: kindSwitch}
	if stmt.Tag != nil {
		out.Children = append(out.Children, node{
			Kind:     "tag",
			Children: []node{expressionNode(stmt.Tag, ctx)},
		})
	}
	for _, clause := range stmt.Body.List {
		caseClause, ok := clause.(*ast.CaseClause)
		if !ok {
			return unsupportedNode(clause)
		}
		out.Children = append(out.Children, node{
			Kind: kindCase,
			Children: append(
				[]node{{Kind: "patterns", Children: expressionNodes(caseClause.List, ctx)}},
				node{Kind: kindThen, Children: statementNodes(caseClause.Body, ctx)},
			),
		})
	}
	return out
}
