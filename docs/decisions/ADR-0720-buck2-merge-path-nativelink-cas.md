---
doc_status: published
id: ADR-0720
title: "Committed to buck2 as the merge path and a cache-only NativeLink CAS"
status: Accepted
planning_impact: true
deciders: founder
date: 2026-09-08
door: two-way
owner: council-architecture
supersedes: []
superseded_by: []
amends: [ADR-0716]
amended_by: []
depends_on: []
related: [ADR-0556, ADR-0560, ADR-0515, ADR-0525, ADR-0718, ADR-0719]
milestone: W0
deliverables:
  - id: ADR-0720-D1
    description: "Record the commitment to buck2 as the destination merge path and NativeLink as a cache-only CAS, and record what ADR-0716's own overturn_when already requires of it."
  - id: ADR-0720-D2
    description: "Admit the manifest/reindeer cargo exception in the build meta-root grammar, which currently refuses the test-support binaries that exception requires."
---

# ADR-0720: Committed to buck2 as the merge path and a cache-only NativeLink CAS

## Status

**Accepted — 2026-09-08.** A new record rather than an edit to ADR-0716, for two
independent reasons. The layout gate refuses any change to a frozen non-root Markdown
file, so ADR-0716 cannot be edited at all. And ADR-0718 established the same shape for the
same family of change, because ADR-0716-D5's replacement-window mechanism deliberately
requires an ADR that does not exist on the protected merge base.

ADR-0716-D4 (self-explanatory debranded CI names) and D6 (reduced PR paperwork) are
untouched and remain in force.

## Context

ADR-0716 ruled the Cargo workspace graph the CI merge path, demoted buck2 to a local
hermeticity tool guarded by a weekly non-blocking smoke, and retired the warm-cache trust
chain — leaving `warm_reads_licensed: false` as a declarative kill-switch "if a CAS is ever
stood up". It superseded ADR-0560, which had specified exactly the NativeLink cache-only
substrate this record now resumes.

That was a defensible reading of the arithmetic. The measured required CI is roughly 4.4
minutes of `test` plus 2.9 minutes of `clippy`, against a CAS that must be deployed,
secured, monitored and garbage-collected. At this scale the wall-clock prize is small and
the operational cost is permanent.

Two things have since made the posture untenable rather than merely conservative.

**The policy layer went buck-shaped while execution stayed cargo-shaped.** The build
meta-root grammar refuses filesystem-discovered binaries, refuses explicit `[[bin]]`
unconditionally in every manifest, and freezes `build/dependency-declarations` to six named
crates. Those are correct rules for a target-graph build system and wrong ones for cargo,
whose target model *is* filesystem discovery. The collision is not hypothetical: the
Reindeer qualification lane is unmergeable because the grammar forbids every available way
to declare the two test-support binaries that the repository's own cargo-nextest job
requires. All three shapes were tried and refused. There is no fix inside that change's
envelope.

**A weekly non-blocking smoke cannot carry a commitment.** ADR-0716-D2 names "or buck2
retired" as an acceptable outcome of a red smoke. Seven days of latency means rot
accumulates between signals and each repair is larger than the last; the stated exit is
decay.

The wall-clock argument that justified ADR-0716 is not the argument that decides this.
Build systems at this shape are chosen for hermeticity, correctness and one graph — not for
minutes on a seven-minute CI. Measurement manages the rollout; it does not gate the
commitment.

## Decision

1. **Buck2 is the destination merge path, and NativeLink is the cache substrate.** This is
   the commitment. What follows is what ADR-0716's own `overturn_when` already requires of
   it, because three plausible ways of acting on it are forbidden by the record being
   amended.

2. **The overturn is same-wave, not incremental.** `overturn_when` names one wave: the
   cloud serves `pipeline/` with buck2 onto `compute/`, CAS in `storage/`, and tenant #0
   presubmit **is** that buck2 graph. Promoting buck2 legs domain by domain is not that wave
   and does not overturn ADR-0716.

3. **A dual cargo+buck2 merge proof stays forbidden.** Running cargo legs as required while
   buck2 legs come up beside them is the prohibited state, not a safe transition through it.
   The changeover is a swap, not an overlap; the guard against an unguarded merge path is
   the wave's own readiness.

4. **CAS alone does not overturn, and its scope is cache-only.** `warm_reads_licensed:
   false` remains the admission control. A CAS plus action cache stores blobs and is
   architecture-agnostic, so OCI A1 (aarch64) capacity may serve amd64 builds. Remote
   execution is not architecture-agnostic — it needs workers matching each action's
   platform, which that capacity cannot supply for amd64 — and stays out of scope until its
   own record.

5. **`manifest/reindeer` remains a cargo exception after the overturn**, as `overturn_when`
   already states. That is not a concession to trade away later: the dependency-declarations
   domain is the buckifier's own bootstrap and cannot be buckified by the thing it produces.
   The grammar must admit what that named exception requires; the exception is not evidence
   against the commitment.

## Consequences

- **Positive:** one graph. The grammar's constraints become earned rather than unpaid-for.
  The class of blockage sitting on the Reindeer qualification lane stops being reachable.

- **Negative, and accepted:** CI wall clock regresses before it improves, and cold buck2 on
  a hosted runner with no cache is the worst point on the curve. ADR-0716's "one CI file,
  wall clock dominated by one cached cargo build" is given up. NativeLink is a service to
  run, secure and garbage-collect, and its buy-versus-run question has no hyperscaler
  precedent to appeal to — Google and Meta built internal remote execution and never faced
  it.

- **Risks:** the dual period is the dangerous window. `presubmit.yml` is at its inline-shell
  budget and protected, so the wave requires the replacement-window mechanism ADR-0716-D5
  defines, with a record absent from the merge base as its authorization — which this record,
  once merged, will no longer be.

## Rules carry why

- **achieves:** one source of truth about what a build target is, so a policy gate and a CI
  runner cannot disagree about whether a declaration is legal.
- **origin:** the grammar went buck-shaped while execution stayed cargo-shaped, and the seam
  surfaced only as a gate refusing what the runner required.
- **rule:** the merge path and the target grammar are the same system and move together.
- **ensure:** this record, plus the grammar amendment D2 names, plus the wave's own
  replacement-window authorization.
- **overturn_when:** the same-wave overturn has not landed and this record is being cited to
  justify a dual merge proof, an incremental promotion, or a CAS standing in for the wave. In
  that case the commitment has failed and this record should be revisited rather than
  stretched.
