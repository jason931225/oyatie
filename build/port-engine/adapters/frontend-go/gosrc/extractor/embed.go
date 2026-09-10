package main

import (
	"fmt"
	"go/types"
	"sort"
	"strings"
)

// Embedding, and the methods it PROMOTES.
//
// Go composes by embedding: an anonymous field or an embedded interface lifts the embedded type's
// methods into the outer type's method set, with no forwarding written anywhere. The target has no
// such rule, so what is implicit in the source has to become explicit in the emit — and what shape
// it takes depends on which side is embedding.
//
// An interface embedding an interface becomes a SUPERTRAIT. That is the census's own reading
// (`census/interfaces.md` §6: "Rust supertraits (`trait A: B`) map this directly"), and it is
// faithful: a type satisfying the outer interface satisfies the embedded one in both languages.
//
// A struct embedding a struct becomes FORWARDING METHODS. §11 item 7 records this as a gap the
// census could not close — its instrument recorded a method only from a declaration with a
// receiver, so struct types have method sets larger than it measured and some look like they have
// no methods at all. go/types closes it exactly, and this is where.

// interfaceEmbeds records the interfaces an interface embeds, for the target's supertrait list.
func interfaceEmbeds(iface *types.Interface, ctx *extractCtx) []node {
	embeds := make([]node, 0, iface.NumEmbeddeds())
	for index := 0; index < iface.NumEmbeddeds(); index++ {
		embedded := iface.EmbeddedType(index)
		named, ok := embedded.(*types.Named)
		if !ok {
			// A type-set element from a generic constraint — `~int | ~string`. It is not a trait
			// and has no supertrait form, so it is recorded as present and refused by name rather
			// than dropped into an interface that silently requires less.
			embeds = append(embeds, node{
				Kind:  kindUnsupported,
				Attrs: map[string]string{attrGoNode: fmt.Sprintf("embedded type set %s", embedded)},
			})
			continue
		}
		if _, ok := named.Underlying().(*types.Interface); !ok {
			embeds = append(embeds, node{
				Kind:  kindUnsupported,
				Attrs: map[string]string{attrGoNode: fmt.Sprintf("embedded non-interface %s", named)},
			})
			continue
		}
		embeds = append(embeds, node{Kind: kindEmbeds, Type: typeTree(named)})
	}
	return embeds
}

// promotedMethods records the methods a type gains through EMBEDDING rather than declaration.
//
// The ownership flags come from the embedded method's own body, because that is where the facts
// are: a forwarding method has no body of its own to observe, and the receiver it must bind is
// decided entirely by what the method it forwards to does. A method whose body is in another
// package has no observable facts here, which `ownershipFacts` reports as unknown rather than as
// absent.
func promotedMethods(named *types.Named, ctx *extractCtx) ([]node, error) {
	set := types.NewMethodSet(types.NewPointer(named))
	out := make([]node, 0, set.Len())
	for index := 0; index < set.Len(); index++ {
		selection := set.At(index)
		path := selection.Index()
		if len(path) < 2 {
			// Declared on the type itself; methodChildren already has it.
			continue
		}
		via, ok := fieldPath(named, path[:len(path)-1])
		if !ok {
			continue
		}
		method, ok := selection.Obj().(*types.Func)
		if !ok {
			continue
		}
		signature, ok := method.Type().(*types.Signature)
		if !ok {
			return nil, fmt.Errorf("promoted method %s without signature", method.Name())
		}

		receiver := ""
		if recv := signature.Recv(); recv != nil {
			receiver = recv.Name()
		}
		flags := flagsFor(method.Exported(), signature.Variadic(), false, isPointerReceiver(signature))
		flags = append(flags, ownershipFacts(ctx.bodies[method], receiver)...)
		sort.Strings(flags)

		out = append(out, node{
			Kind:     kindPromoted,
			Name:     method.Name(),
			Flags:    flags,
			Attrs:    withAttr(withDoc(nil, ctx.docs[method]), attrVia, via),
			Children: signatureChildren(signature, ctx.qualify),
		})
	}
	sortNodes(out)
	return out, nil
}

// fieldPath renders the embedded-field indices go/types hands back as a dotted field path.
//
// Indices rather than names, because that is what a selection carries — and the target needs the
// names, since `self.1.0` is not how a named struct is addressed.
func fieldPath(named *types.Named, indices []int) (string, bool) {
	parts := make([]string, 0, len(indices))
	current := named.Underlying()
	for _, index := range indices {
		structured, ok := current.(*types.Struct)
		if !ok || index >= structured.NumFields() {
			return "", false
		}
		field := structured.Field(index)
		parts = append(parts, field.Name())
		next := field.Type()
		if pointer, ok := next.(*types.Pointer); ok {
			next = pointer.Elem()
		}
		current = next.Underlying()
	}
	return strings.Join(parts, "."), true
}

// embeddedInterfaces reports the interfaces an interface embeds, transitively. See satisfy.go on
// why a supertrait is a requirement rather than a copied method set.
func embeddedInterfaces(named *types.Named, seen map[string]bool) []*types.Named {
	iface, ok := named.Underlying().(*types.Interface)
	if !ok {
		return nil
	}
	var out []*types.Named
	for index := 0; index < iface.NumEmbeddeds(); index++ {
		embedded, ok := iface.EmbeddedType(index).(*types.Named)
		if !ok {
			continue
		}
		key := typeKey(embedded)
		if seen[key] {
			continue
		}
		seen[key] = true
		out = append(out, embedded)
		out = append(out, embeddedInterfaces(embedded, seen)...)
	}
	return out
}
