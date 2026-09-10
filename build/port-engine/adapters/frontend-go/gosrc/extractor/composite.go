package main

import (
	"go/ast"
	"go/types"
)

// Composite literals, and the fields the source leaves out.
//
// The source zero-fills what a literal omits; the target rejects an incomplete literal outright.
// Which fields a struct HAS is a fact go/types holds and the engine does not, so the omitted ones
// are recorded here as `zero` nodes carrying their type — leaving the target's spelling of that
// zero to the rule pack.

// compositeNode records a struct literal with every DECLARED field present.
func compositeNode(lit *ast.CompositeLit, ctx *extractCtx) node {
	fields := compositeStruct(lit, ctx)
	if fields == nil {
		// A slice, map or array literal. It reached here as a composite with no struct behind it,
		// and the previous shape emitted an empty struct literal for it — silently constructing
		// nothing. Recording it as unsupported refuses it by name instead.
		return unsupportedNode(lit)
	}

	written := make(map[string]node, len(lit.Elts))
	for _, element := range lit.Elts {
		keyed, ok := element.(*ast.KeyValueExpr)
		if !ok {
			// A POSITIONAL composite depends on field order, which the target does not reproduce
			// for a named struct — and getting it silently wrong swaps two fields of the same type
			// with no diagnostic anywhere.
			return unsupportedNode(element)
		}
		key, ok := keyed.Key.(*ast.Ident)
		if !ok {
			return unsupportedNode(keyed)
		}
		written[key.Name] = expressionNode(keyed.Value, ctx)
	}

	out := node{Kind: kindComposite, Type: compositeType(lit, ctx)}
	for index := 0; index < fields.NumFields(); index++ {
		field := fields.Field(index)
		value, present := written[field.Name()]
		if !present {
			value = node{Kind: kindZero, Name: field.Name(), Type: typeTree(field.Type())}
		}
		out.Children = append(out.Children, node{
			Kind:     kindKeyed,
			Name:     field.Name(),
			Children: []node{value},
		})
	}
	return out
}

// compositeStruct reports the struct a composite literal constructs, or nil when it constructs
// something else.
func compositeStruct(lit *ast.CompositeLit, ctx *extractCtx) *types.Struct {
	tv, ok := ctx.info.Types[lit]
	if !ok || tv.Type == nil {
		return nil
	}
	underlying := tv.Type.Underlying()
	if pointer, ok := underlying.(*types.Pointer); ok {
		underlying = pointer.Elem().Underlying()
	}
	structured, ok := underlying.(*types.Struct)
	if !ok {
		return nil
	}
	return structured
}

// compositeType records what a composite literal constructs.
func compositeType(lit *ast.CompositeLit, ctx *extractCtx) *typeNode {
	if tv, ok := ctx.info.Types[lit]; ok && tv.Type != nil {
		return typeTree(tv.Type)
	}
	return nil
}
