package main

import (
	"crypto/sha256"
	"encoding/hex"
	"sort"
	"strconv"
)

// The cross-language digest preimage.
//
// Length-prefixed and arity-tagged, and MIRRORED byte for byte in the Rust snapshot adapter. The
// mirror is the point: an encoder that drifts from its twin shows up as a digest mismatch at
// admission rather than as a snapshot that decodes into something else.

// ---------------------------------------------------------------------------------
// Snapshot preimage (mirrored by port_engine_snapshot::snapshot_preimage_v1)
// ---------------------------------------------------------------------------------
//
// `F(s)` is the decimal byte length of s, a `:`, then s. Every node encodes as
//
//	F(kind) F(name) T(type) F(len(flags)) flags...
//	    F(len(attrs)) (F(key) F(value))... F(len(children)) children...
//
// where T(type) is F("0") for an absent type, and otherwise
//
//	F("1") F(kind) F(name) F(package) F(len(args)) args...
//
// Length prefixes plus explicit arity make the encoding injective: no value, however it
// is spelled, can imitate a delimiter or absorb a sibling. That is why the digest does not
// depend on JSON canonicalization — and why the same preimage can be computed in Go here
// and in Rust there, with any drift between the two surfacing as a digest mismatch at
// admission rather than as a silently accepted snapshot.

func preimage(model *snapshot) []byte {
	out := make([]byte, 0, 4096)
	field(&out, "snapshot")
	field(&out, model.Language)
	field(&out, strconv.Itoa(len(model.Packages)))
	for _, pkg := range model.Packages {
		field(&out, "package")
		field(&out, pkg.UnitID)
		field(&out, pkg.Producer)
		field(&out, strconv.Itoa(len(pkg.Declarations)))
		for _, decl := range pkg.Declarations {
			encodeNode(&out, decl)
		}
	}
	return out
}

func encodeNode(out *[]byte, n node) {
	field(out, n.Kind)
	field(out, n.Name)
	encodeType(out, n.Type)
	field(out, strconv.Itoa(len(n.Flags)))
	for _, flag := range n.Flags {
		field(out, flag)
	}
	// Sorted, so the map has exactly one encoding. A map with two orderings is a map with two
	// digests, and the receipt would then attribute a byte-identical corpus to a moved axis.
	attrKeys := make([]string, 0, len(n.Attrs))
	for key := range n.Attrs {
		attrKeys = append(attrKeys, key)
	}
	sort.Strings(attrKeys)
	field(out, strconv.Itoa(len(attrKeys)))
	for _, key := range attrKeys {
		field(out, key)
		field(out, n.Attrs[key])
	}
	field(out, strconv.Itoa(len(n.Children)))
	for _, child := range n.Children {
		encodeNode(out, child)
	}
}

// encodeType covers the type TREE. Leaving it out would put every type outside the snapshot
// identity: change a field's type and `snapshot_digest` would not move, so the receipt would find
// emitted bytes changed with all six axes held and call a fully explainable change Unexplained.
func encodeType(out *[]byte, t *typeNode) {
	if t == nil {
		field(out, "0")
		return
	}
	field(out, "1")
	field(out, t.Kind)
	field(out, t.Name)
	field(out, t.Package)
	field(out, strconv.Itoa(len(t.Args)))
	for _, arg := range t.Args {
		encodeType(out, arg)
	}
}

func field(out *[]byte, value string) {
	*out = append(*out, strconv.Itoa(len(value))...)
	*out = append(*out, ':')
	*out = append(*out, value...)
}

func digest(preimage []byte) string {
	sum := sha256.Sum256(preimage)
	return "sha256:" + hex.EncodeToString(sum[:])
}
