package main

import (
	"go/ast"
	"go/types"
)

// Interface satisfaction, observed at USE SITES.
//
// Go's interfaces are implicit: nothing in a type's declaration says which interfaces it satisfies,
// and structural matching is combinatorial. docs/programs/k8s-port/census/interfaces.md measured
// it — name-level structural matches outnumber the pairs the source declares outright by a wide
// margin — and its conclusion is that the engine must emit impls from USAGE.
//
// This is that pass. A pair is recorded where a concrete value actually flows into an
// interface-typed position: a declared assertion, an assignment, a call argument, a return. Each
// site is recorded with the pair, because a declared assertion is compile-checked by Go and a
// flow-derived one is this instrument's inference — a reviewer auditing an impl needs to know
// which kind it is looking at.
//
// WHAT THIS DOES NOT FIND, so that the gap is a known one rather than a discovered one:
//   - a concrete value stored into an interface-typed struct FIELD, or into a map or slice of
//     interface, through a composite literal;
//   - satisfaction of an INLINE interface, which has no named trait to implement — that position
//     refuses at type resolution instead, so it is not lost, only refused elsewhere;
//   - anything in a package outside the corpus.
//
// The value/pointer distinction of Go's method set is deliberately NOT recorded. `var _ I = T{}`
// and `var _ I = (*T)(nil)` differ in Go because a value's method set excludes pointer-receiver
// methods; in the target the same distinction survives as borrow checking, and rustc enforces it
// on the emitted impl. Recording it would add a fact nothing reads.

// satisfaction is one observed pair: a concrete named type flowing into a named interface position.
type satisfaction struct {
	concrete *types.Named
	iface    *types.Named
	site     string
	// observedIn is the unit whose source shows the flow, which is not always the unit that
	// declares either side.
	observedIn string
}

// Site kinds, in the order of how much they prove.
const (
	// siteAssertion is `var _ Iface = value`, which the Go compiler checks. The census's proven
	// floor is made entirely of these.
	siteAssertion = "assertion"
	siteAssign    = "assign"
	siteArgument  = "argument"
	siteResult    = "result"
)

// collectSatisfactions walks a type-checked package for every position where a concrete type
// becomes an interface.
func collectSatisfactions(files []*ast.File, info *types.Info, unitID string) []satisfaction {
	found := []satisfaction{}
	record := func(target types.Type, value ast.Expr, site string) {
		iface, ok := namedInterface(target)
		if !ok {
			return
		}
		concrete, ok := concreteNamed(info.TypeOf(value))
		if !ok {
			return
		}
		found = append(found, satisfaction{
			concrete:   concrete,
			iface:      iface,
			site:       site,
			observedIn: unitID,
		})
		// Satisfying an interface satisfies everything it embeds — the Go compiler checks that,
		// and the target needs it written down: a supertrait is a REQUIREMENT, so an impl of the
		// outer trait does not compile without impls of the embedded ones.
		for _, super := range embeddedInterfaces(iface, map[string]bool{}) {
			found = append(found, satisfaction{
				concrete:   concrete,
				iface:      super,
				site:       site,
				observedIn: unitID,
			})
		}
	}

	for _, file := range files {
		for _, decl := range file.Decls {
			switch typed := decl.(type) {
			case *ast.GenDecl:
				valueSpecs(typed, info, record, siteAssertion)
			case *ast.FuncDecl:
				if typed.Body == nil {
					continue
				}
				results := functionResults(typed, info)
				inspectBody(typed.Body, info, results, record)
			}
		}
	}
	return found
}

// valueSpecs records `var x Iface = value` for every value spec carrying an explicit type.
func valueSpecs(
	decl *ast.GenDecl,
	info *types.Info,
	record func(types.Type, ast.Expr, string),
	site string,
) {
	for _, spec := range decl.Specs {
		spec, ok := spec.(*ast.ValueSpec)
		if !ok || spec.Type == nil {
			continue
		}
		for _, value := range spec.Values {
			record(info.TypeOf(spec.Type), value, site)
		}
	}
}

// inspectBody records assignments, call arguments and returns inside one function body.
func inspectBody(
	body *ast.BlockStmt,
	info *types.Info,
	results *types.Tuple,
	record func(types.Type, ast.Expr, string),
) {
	ast.Inspect(body, func(n ast.Node) bool {
		switch typed := n.(type) {
		case *ast.DeclStmt:
			if decl, ok := typed.Decl.(*ast.GenDecl); ok {
				valueSpecs(decl, info, record, siteAssertion)
			}

		case *ast.AssignStmt:
			// Only the aligned one-to-one form. A multi-value assignment from one call has no
			// per-value expression to attribute the flow to, and pairing them by position would
			// be attributing a fact to syntax that does not carry it.
			if len(typed.Lhs) != len(typed.Rhs) {
				return true
			}
			for index, lhs := range typed.Lhs {
				record(info.TypeOf(lhs), typed.Rhs[index], siteAssign)
			}

		case *ast.CallExpr:
			recordArguments(typed, info, record)

		case *ast.ReturnStmt:
			if results == nil || results.Len() != len(typed.Results) {
				return true
			}
			for index, value := range typed.Results {
				record(results.At(index).Type(), value, siteResult)
			}
		}
		return true
	})
}

// recordArguments pairs a call's arguments with the parameters they are passed to.
func recordArguments(
	call *ast.CallExpr,
	info *types.Info,
	record func(types.Type, ast.Expr, string),
) {
	signature, ok := info.TypeOf(call.Fun).(*types.Signature)
	if !ok {
		return
	}
	params := signature.Params()
	// A variadic call spreads its tail into one parameter, so position stops being the pairing.
	// Fixed arguments still pair, and the tail is left to the composite-literal gap named above.
	fixed := params.Len()
	if signature.Variadic() {
		fixed--
	}
	for index, arg := range call.Args {
		if index >= fixed {
			return
		}
		record(params.At(index).Type(), arg, siteArgument)
	}
}

// functionResults reports the result tuple of a declared function, for attributing returns.
func functionResults(decl *ast.FuncDecl, info *types.Info) *types.Tuple {
	obj, ok := info.Defs[decl.Name]
	if !ok || obj == nil {
		return nil
	}
	signature, ok := obj.Type().(*types.Signature)
	if !ok {
		return nil
	}
	return signature.Results()
}

// namedInterface reports the named interface a position requires, if it requires one.
func namedInterface(t types.Type) (*types.Named, bool) {
	named, ok := t.(*types.Named)
	if !ok {
		return nil, false
	}
	if _, ok := named.Underlying().(*types.Interface); !ok {
		return nil, false
	}
	return named, true
}

// concreteNamed reports the named non-interface type a value has, through one level of pointer.
//
// An interface flowing into an interface position is not a satisfaction to emit: the target
// expresses it as a supertrait or a blanket impl, and which of those is right is a decision no
// single use site can make.
func concreteNamed(t types.Type) (*types.Named, bool) {
	if pointer, ok := t.(*types.Pointer); ok {
		t = pointer.Elem()
	}
	named, ok := t.(*types.Named)
	if !ok {
		return nil, false
	}
	if _, ok := named.Underlying().(*types.Interface); ok {
		return nil, false
	}
	return named, true
}
