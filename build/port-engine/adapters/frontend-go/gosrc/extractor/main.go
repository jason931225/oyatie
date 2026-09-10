// Command extractor is the bootstrap Go front end for the owned deterministic port
// engine (ADR-0638 D3).
//
// It reads a Go corpus with go/parser + go/types and writes a SourceModel snapshot
// envelope as JSON. It runs OUT OF BAND ONLY: the engine's verify() path consumes the
// snapshot artifact and must never invoke a Go toolchain. The Rust side enforces that
// with architecture tests over its own library sources; nothing here is linked into the
// engine.
//
// Usage:
//
//	go run ./extractor -corpus ./corpus -module oyatie.example/portengine-fixture \
//	    -out ../../snapshot/src/fixture-snapshot-v1.json
package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"os"
)

// producerBootstrapGo mirrors port_engine_frontend_go::PRODUCER_BOOTSTRAP_GO. The Rust
// decoder refuses any other identity during bootstrap admission, so drift here is a red
// at admission rather than a silent relabel.

// ---------------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------------

func main() {
	corpus := flag.String("corpus", "./corpus", "directory whose subdirectories are Go packages")
	module := flag.String("module", "oyatie.example/portengine-fixture", "module path prefix for unit ids")
	root := flag.String("root", ".", "module root; unit ids are import paths relative to it")
	out := flag.String("out", "", "output file; empty writes to stdout")
	flag.Parse()

	model, err := extract(*corpus, *module, *root)
	if err != nil {
		fmt.Fprintf(os.Stderr, "extractor: %v\n", err)
		os.Exit(1)
	}

	encoded, err := render(model)
	if err != nil {
		fmt.Fprintf(os.Stderr, "extractor: %v\n", err)
		os.Exit(1)
	}

	if *out == "" {
		os.Stdout.Write(encoded)
		return
	}
	if err := os.WriteFile(*out, encoded, 0o644); err != nil {
		fmt.Fprintf(os.Stderr, "extractor: write %s: %v\n", *out, err)
		os.Exit(1)
	}
}

func render(model *snapshot) ([]byte, error) {
	// Indented JSON with a trailing newline, so the committed artifact is reviewable and
	// byte-stable. The digest is computed over the preimage below rather than over these
	// bytes, so JSON formatting is never load-bearing for identity.
	encoded, err := json.MarshalIndent(model, "", "  ")
	if err != nil {
		return nil, fmt.Errorf("marshal snapshot: %w", err)
	}
	return append(encoded, '\n'), nil
}
