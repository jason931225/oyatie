package main

import (
	"go/ast"
	"go/types"
)

// What a CALL actually is.
//
// The source spells three different things with one production. `f(x)` is a call, `value.Method()`
// is a call through a receiver, `package.Function()` is a call to a free function, and `uint32(x)`
// is not a call at all. Syntax cannot separate them — only the type-checker knows which name is a
// package and which is a type — and every defect this file exists to prevent came from deciding by
// shape: a cross-package call emitted as a method call on a binding that does not exist, and a
// conversion emitted as a call to a function with no name.

// conversionTarget reports the type a call CONVERTS to, or nil when the call is a call.
//
// go/types records a conversion as a call whose callee denotes a TYPE. Nothing in the syntax says
// so — `uint32(x)` and `f(x)` are the same production — which is why this asks the type-checker
// rather than the shape.
func conversionTarget(call *ast.CallExpr, ctx *extractCtx) *typeNode {
	if len(call.Args) != 1 {
		return nil
	}
	var name *ast.Ident
	switch typed := call.Fun.(type) {
	case *ast.Ident:
		name = typed
	case *ast.SelectorExpr:
		name = typed.Sel
	case *ast.ArrayType:
		// `[]byte(s)` — the callee is a type expression rather than a name.
		if tv, ok := ctx.info.Types[call.Fun]; ok && tv.IsType() {
			return typeTree(tv.Type)
		}
		return nil
	default:
		return nil
	}
	if _, ok := ctx.info.Uses[name].(*types.TypeName); !ok {
		return nil
	}
	if tv, ok := ctx.info.Types[call]; ok && tv.Type != nil {
		return typeTree(tv.Type)
	}
	return nil
}

// calleeIsMethod reports whether a call goes through a RECEIVER.
func calleeIsMethod(fun ast.Expr, ctx *extractCtx) bool {
	selector, ok := fun.(*ast.SelectorExpr)
	if !ok {
		return false
	}
	method, ok := ctx.info.Uses[selector.Sel].(*types.Func)
	if !ok {
		return false
	}
	signature, ok := method.Type().(*types.Signature)
	return ok && signature.Recv() != nil
}

// sliceBound records a slice's bound, or its ABSENCE as a node of its own.
//
// Absence is recorded rather than omitted because the two ends are positional: dropping a missing
// low bound would make `s[:hi]` indistinguishable from `s[lo:]`, and the two are different programs.
func sliceBound(expr ast.Expr, ctx *extractCtx) node {
	if expr == nil {
		return node{Kind: kindAbsent}
	}
	return expressionNode(expr, ctx)
}
