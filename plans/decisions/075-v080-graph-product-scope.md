# ADR-075: Expand v0.8.0-beta to the Graph V2 product

## Status

**Proposed** — 2026-06-08, Josh. Ready for council review. Reverses a
release-scope deferral recorded in
[`RELEASE-PLAN.md`](../../RELEASE-PLAN.md), not an architectural decision; the
hot-path boundary it relies on is already Accepted
([ADR-063](063-gv2-hot-path-boundary.md)). Acceptance gates the module
re-sequencing and the release-plan rewrite that derive from it.

## Date

2026-06-08

## Context

The `v0.8.0-beta` "The Save-Time Daemon" window was scoped as the **interim-cache
slice only**: the frozen `validate_paths` wire ([ADR-061](061-save-time-daemon-delta-validation.md))
plus the `watch`/MCP clients, backed by a rebuild-on-restart `SymbolGraph` cache
(DSV Sub-phase A, now Merged 9/9 incl. cross-platform A-W). The Graph V2 hot-read
backing (Sub-phase A′), the `graph-v2-foundation` (GV2) and
`graph-context-delivery` (GCTX) modules, and warm-start persistence (Sub-phase B)
were **explicitly deferred** out of the window, and "scope creep pulls GV2 into
the window" is recorded there as a *risk to mitigate*.

A pre-cut honesty check of what that slice actually delivers to a default user
surfaced the problem this ADR responds to:

- The `watch` daemon-routing path is **opt-in behind `ANVIL_WATCH_DAEMON`**
  (`crates/anvil-cli/src/commands/watch.rs` → `daemon_routing_enabled()`,
  default off) and only for `check` watches. A stock install sees **no change**.
- The MCP `anvil_validate_write` path still defaults to `DaemonStatus::NotWired`
  with an embedded fallback and remains on `scan_buffer`, not the new wire.
- Hot-path coverage is `check_families: ["antipattern"]` only, 1-hop, no
  persistence.

So as scoped the window is an **architectural milestone, not a user-visible
feature drop**: the engine moved, but the default user experience does not, and
the thing that reads as a *product* — a persistent, joined structural model
backing both Anvil's own decisions and assistant context — is the deferred GV2 /
GCTX work.

The deferral's two stated prerequisites have since landed:
`docs/architecture/graph-v2-foundation-spec.md` (GV2-001, PR #2350) and the
stable-identity + export-diff primitive (GV2-002, PR #2387) plus the delta/event
contract (GV2-003, PR #2391). The A′ hot-path boundary is Accepted
([ADR-063](063-gv2-hot-path-boundary.md), which *closes* the A′ gate and freezes
the GV2-022 hot-read API). GV2 is therefore 4/19 with its hard blockers cleared —
the deferral is now a timing choice, not a dependency wall.

## Decision

Re-scope `v0.8.0-beta` from the interim-cache slice into the **Graph V2
product**. Bring the following into the active window:

1. **GV2 foundation** — the remaining `graph-v2-foundation` items (010, 011, 012,
   013, 014, 020, 022, 023, 024, 025, 026, 027, 028, 029, 030), sequenced by the
   module's dependency graph.
2. **The A→A′ backing swap** — GV2-027 retires the interim `KernelGraphCache`
   re-derive and points `validate_paths`/`save_time` at the resident GV2 hot-read
   index under the **unchanged frozen wire**, with a verdict-parity property
   proof and the ADR-031 Criterion perf gate (GV2-025) green. `backing_schema_version`
   moves `interim-symbolgraph-v1` → `gv2-hotindex-v1`.
3. **Graph context delivery** — the `graph-context-delivery` (GCTX) module (13
   items) as the assistant-facing projection over the same model.
4. **Default-on save-time daemon routing** — flip `ANVIL_WATCH_DAEMON` to
   default-on for `check` watches once the GV2 backing + the ADR-061 §8
   correctness bar are green, so the save-time improvement reaches every user
   rather than only opt-in users.

The frozen `validate_paths` wire (ADR-061) and the hot-/non-hot-path boundary
(ADR-063) are **unchanged** — this ADR builds on them; it does not supersede
them. Warm-start persistence (Sub-phase B, GV2-021 / [ADR-069](069-graph-v2-persistence.md)
already Accepted) stays **deferred** unless a follow-up pulls it in; it is
orthogonal to a default-on, rebuild-on-restart product.

The cut is still **quality-gated, not calendar-gated** (release-cadence policy):
cut when the slice is ready and gates are green. This ADR accepts a materially
later cut in exchange for a release a default user actually experiences.

## Rationale

The window exists to deliver save-time governance value. As scoped it delivers
that value only to opt-in users on one check family — which does not earn a
minor's user-facing claim. The graph substrate is what turns the daemon from an
internal re-plumbing into a product: default-on save-time validation backed by a
real resident model, plus assistant context delivery over that same trusted
model. The blockers that justified deferral (spec, stable identity, boundary
gate) are now resolved, so the cost of pulling it in is build time, not
architectural risk.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Expand to the GV2 product (chosen)** | v0.8.0 becomes a real, default user-facing release; one coherent graph story; substrate unblocks INTD/DRVR/WEAVE/provenance | Largest scope (~28 items, mostly dep-chained `Draft`); materially later cut; biggest review surface |
| Ship the interim slice, defer all graph (status quo) | Cut now; minimal risk | Default user sees nothing; flag-off antipattern-only; "minor" claim is thin |
| Flip `ANVIL_WATCH_DAEMON` default-on only, no GV2 | Small change; real default CPU fix without graph | Still antipattern-only, interim re-derive backing; no graph product or context delivery |
| A′ slice only (GV2-010/011/012/022/024/025/028/029/027) | Real graph *backing* for the daemon; ~9 items | No multi-graph registry / query contract / GCTX — backs the daemon but ships no assistant-facing graph product |

## Consequences

- **Positive:** v0.8.0-beta ships a default-on, graph-backed save-time validator
  plus assistant graph-context delivery — a genuine product, not a re-plumbing.
  The GV2 substrate lands once and stops INTD/DRVR/WEAVE/provenance from each
  inventing partial graph models.
- **Negative:** The window grows ~3× and the cut slips materially. The release
  plan, NBI, and several module statuses all churn. Until GV2-028's production
  parser feed lands, `ContentModify` verdicts stay `partial` — the swap must not
  ship before it.
- **Risks:** (a) GCTX is the largest unproven surface (0/13) and could dominate
  the timeline; (b) the A′ swap must prove verdict parity *and* hold the ADR-031
  budget or it regresses the very CPU problem the window exists to fix;
  (c) default-on daemon routing widens the blast radius of any daemon defect.
- **Mitigations:** GV2-027 is gated on the GV2-025 Criterion CI gate + a parity
  property test (both already specified); default-on is gated behind the
  ADR-061 §8 correctness bar; GCTX can be sub-staged so a parser-feed or registry
  delay does not block the daemon-backing payload. If GCTX threatens the cut,
  fall back to the **A′ slice** scope (GCTX re-deferred) without re-opening this
  decision.

## References

- Related ADRs: [ADR-061](061-save-time-daemon-delta-validation.md) (frozen
  wire), [ADR-063](063-gv2-hot-path-boundary.md) (hot-path boundary — closes the
  A′ gate), [ADR-064](064-intercept-graph-cache-crate-boundary.md)
  (`anvil-graph-cache` crate), [ADR-031](031-validation-latency-rubric.md)
  (latency budget), [ADR-069](069-graph-v2-persistence.md) / GV2-021
  (persistence, stays deferred)
- APS modules: GV2 (`graph-v2-foundation`), GCTX (`graph-context-delivery`),
  DSV (`daemon-save-time-validation`, Sub-phase A′)
- Release: [`RELEASE-PLAN.md`](../../RELEASE-PLAN.md) (the deferral this reverses),
  [`docs/policies/release-cadence.md`](../../docs/policies/release-cadence.md)
- Spec: [`docs/architecture/graph-v2-foundation-spec.md`](../../docs/architecture/graph-v2-foundation-spec.md)
