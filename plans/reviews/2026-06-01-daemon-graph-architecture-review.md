# Daemon + Graph Architecture Review

**Date:** 2026-06-01
**Reviewer:** Codex
**Scope:** `origin/main` at `96c963833`; ADR-061, ADR-063, the daemon
save-time validation contract, the Sub-phase A implementation plan, Graph v2
foundation APS, and current kernel/intercept graph code.

## Verdict

Conditional accept for the product direction: save-time governance should be
daemon-mediated, verdict-shaped, and backed by warm graph state. Do not promote
Graph v2 to Ready, and do not start Sub-phase A implementation, until the
blockers below are resolved.

The high-level taxonomy is directionally accepted: semantic, dependency/impact,
trust/policy, control/session, and plan/provenance graphs are the right split for
Anvil. ADR-063 now closes the GV2 hot-/non-hot-path boundary for sub-phase A',
but the implementation contract is still not coherent enough to close the full
Graph v2 Ready checklist because stable identity, the first-slice validation
claim, and the canonical GV2 architecture artefact remain incomplete.

## Findings

### Critical: `coverage: certified` can claim more than the implementation evaluates

The contract defines `coverage` as certified iff workspace assurance is clean
(`plans/specs/2026-06-01-daemon-save-time-validation-contract.md:37-43`). The
Sub-phase A plan then says `validate_paths` will run
`run_antipattern_check(changed_paths, config, workspace_root)` as the diagnostic
engine (`plans/execution/2026-06-01-daemon-save-time-subphase-a.md:180-188`).
That is not the same surface as the current kernel structural policy path, which
registers `CrossLayerViolation`, `NewDependencyIntroduction`,
`PublicApiExpansion`, and `PrivilegeExpansion`
(`crates/anvil-kernel/src/embedded.rs:119-133`).

Risk: the daemon can return a clean/certified workspace for a change that was
only checked by the antipattern scanner, while architecture boundary and graph
policy invariants were not evaluated. That directly weakens Anvil's core
save-time architecture-drift claim.

Required fix: either make `validate_paths` run the graph policy engine over the
certified closure before returning `coverage: certified`, or split coverage by
check family so callers cannot interpret an antipattern-only pass as complete
architecture assurance.

### Critical: reverse-impact closure depends on an index that Sub-phase A does not cache

The contract requires certifiability to use `DependencyGraph.reverse`
(`dependents_of(F)`) for the bounded reverse-impact closure
(`plans/specs/2026-06-01-daemon-save-time-validation-contract.md:155-170`).
That API exists on `DependencyGraph` (`crates/anvil-kernel/src/graph/dependency.rs:7-45`),
but the Sub-phase A cache is specified as a per-`WorktreeKey` `SymbolGraph`
only (`plans/execution/2026-06-01-daemon-save-time-subphase-a.md:168-178`), and
the proposed `certify` signature accepts `&SymbolGraph`
(`plans/execution/2026-06-01-daemon-save-time-subphase-a.md:156-166`).

Current `update_file` also mutates only `SymbolGraph`; it collects known files
from symbol nodes and resolves import edges there
(`crates/anvil-kernel/src/graph/incremental.rs:89-156`). It does not maintain a
`DependencyGraph.reverse` index for daemon reads.

Risk: the load-bearing soundness property for unchanged importers cannot be
implemented from the state the plan says the daemon owns.

Required fix: make the interim cache hold and maintain a dependency/impact index
alongside `SymbolGraph`, or explicitly pull GV2-011/GV2-022 forward before any
certified reverse-impact claim. Task 6 should take that index as input, not just
`SymbolGraph`.

### Major: export-surface detection is underspecified against the current delta model

The contract certifies a content modify only when there is no export-surface
change (`plans/specs/2026-06-01-daemon-save-time-validation-contract.md:161-165`).
The current `GraphDelta` records added/removed symbol ids, added/removed edges,
and previous public/privileged baseline keys
(`crates/anvil-kernel/src/graph/incremental.rs:9-24`), but the update path
removes all symbols for the file and re-adds the new symbols
(`crates/anvil-kernel/src/graph/incremental.rs:40-87`). Stable symbol identity
and changed-node semantics remain Draft GV2 work
(`plans/archive/modules/graph-v2-foundation.aps.md:158-192`).

Risk: Sub-phase A cannot reliably tell "implementation body changed" from
"export surface changed" without either a conservative fallback or a real stable
identity/export-diff helper. Misclassification here is exactly how the daemon
would skip importers that should have been revalidated.

Required fix: add an explicit export-surface diff contract with fixtures for
rename, delete/recreate, re-export, and same-name symbols in different scopes.
Until that exists, default changed exported files to partial/stale rather than
certified.

### Major: Sub-phase A must not blur interim delta application with ADR-063 hot reads

ADR-063 resolves the GV2 sub-phase A' boundary: hot-path-admissible GV2 reads
must come from resident warm indexes, with no parse, no cross-file resolution,
no unbounded traversal, and no blocking I/O. A warm miss maps to a typed stale
result rather than doing slow work on the hot path.

The remaining issue is Sub-phase A wording. The implementation plan still says
the interim path reads bytes, applies deltas, and certifies before diagnostics
(`plans/execution/2026-06-01-daemon-save-time-subphase-a.md:187-188`), while
current graph delta application resolves imports during `update_file`
(`crates/anvil-kernel/src/graph/incremental.rs:127-144`). That may be acceptable
for the interim SymbolGraph cache, but it is not the same thing as an ADR-063
GV2 hot-read.

Risk: implementers can accidentally carry interim parse/resolve behaviour into
the GV2 hot-read API, or reviewers can incorrectly reject the interim path as an
ADR-063 violation.

Required fix: update the Sub-phase A plan to name two lanes explicitly:
interim delta-maintenance work behind the SymbolGraph cache, and ADR-063
resident-index reads for A'. The GV2 API must keep the ADR-063 warm-miss
contract; the interim lane still needs its own latency and soundness tests.

### Major: the frozen wire refers to a scan-buffer envelope type that is not owned by the proto crate

Task 1 places `ValidatePathsResponse` in `anvil-intercept-proto` and references
`ScanDiagnostics` as "reuse the scan_buffer envelope type verbatim"
(`plans/execution/2026-06-01-daemon-save-time-subphase-a.md:91-99`). The current
scan-buffer response type is daemon-local (`crates/anvil-intercept/src/midedit.rs`)
and the MCP client mirrors the JSON shape locally rather than importing a proto
response type.

Risk: the "frozen" wire can drift during implementation because the plan names a
type boundary that does not exist. Moving the type later becomes a breaking
refactor if clients already copied a draft shape.

Required fix: first move or define the shared scan-buffer result envelope in
`anvil-intercept-proto`, then define `ValidatePathsResponse` in terms of that
actual type. Add parity tests proving `scan_buffer` and `validate_paths`
diagnostic envelopes serialise through the same shared struct.

### Major: the canonical Graph v2 architecture spec file is still absent

GV2-001 names `docs/architecture/graph-v2-foundation-spec.md` as the architecture
spec and taxonomy authority (`plans/archive/modules/graph-v2-foundation.aps.md:141-154`).
That file is not present on `origin/main`; only the APS module and ADR-061
consumer contract currently carry the taxonomy.

Risk: the first Ready checklist item cannot be closed from a canonical artefact,
and downstream modules will continue to cite APS prose instead of a durable
as-built/design authority.

Required fix: land the GV2 architecture spec before marking "Graph taxonomy
accepted by architecture review". The review can accept the split, but the
artefact still needs to exist.

## Ready Checklist Disposition

| Gate | Disposition |
| --- | --- |
| Graph taxonomy accepted by architecture review | Directionally accepted, but do not tick until `docs/architecture/graph-v2-foundation-spec.md` lands. |
| Hot-path/non-hot-path boundary agreed with INTD and DRVR owners | Accepted by ADR-063 for sub-phase A'. The Graph v2 APS row may still need reconciliation to reflect that. |
| Stable identity model reviewed against git rename and symbol rename cases | Not accepted. GV2-002 remains Draft and is needed for export-surface confidence. |
| Persistence strategy ADR drafted or explicitly assigned to GV2-021 | Accepted as assigned/speced for later; persistence must remain default-off and restore indexes only, never verdicts. |
| Privacy review completed for persisted provenance/session fields | Not complete for the full GV2 persisted provenance surface; ADR-061 only covers the warm-index privacy line. |
| GCTX module updated to depend on GV2 rather than owning foundation work | Not reviewed in this pass. |
| Validation commands for the first implementation slice are concrete | Partially accepted; the commands exist, but the parity gate must include graph policy/architecture diagnostics, not only antipattern findings. |

## Required Follow-ups Before Implementation

1. Update the Sub-phase A plan so `validate_paths` evaluates graph policy
   invariants or narrows its coverage claim.
2. Add an interim dependency/impact index to the daemon cache, or pull the GV2
   hot-read index earlier.
3. Define export-surface diff semantics before any certified no-export-change
   fast path.
4. Clarify the Sub-phase A vs A' wording so interim delta maintenance cannot
   leak into the ADR-063 GV2 hot-read API.
5. Move the diagnostic envelope type into the proto boundary before freezing the
   new wire types.
6. Create the canonical Graph v2 architecture spec and rerun this review against
   that file.

## Evidence Reviewed

- `plans/decisions/061-save-time-daemon-delta-validation.md`
- `plans/decisions/063-gv2-hot-path-boundary.md`
- `plans/specs/2026-06-01-daemon-save-time-validation-contract.md`
- `plans/execution/2026-06-01-daemon-save-time-subphase-a.md`
- `plans/archive/modules/graph-v2-foundation.aps.md`
- `docs/architecture/intercept-as-built.md`
- `docs/architecture/kernel-as-built.md`
- `crates/anvil-kernel/src/graph/dependency.rs`
- `crates/anvil-kernel/src/graph/incremental.rs`
- `crates/anvil-kernel/src/embedded.rs`
- `crates/anvil-intercept-proto/src/protocol.rs`
- `crates/anvil-intercept/src/ipc.rs`
