# ADR-075: Back v0.8.0-beta with the Graph V2 A′ slice (GCTX → v0.9)

## Status

**Accepted** — 2026-06-08, Josh, via full council review (session
`council-614e422c`, 5 reviewers; accept-with-changes — changes applied). The
council endorsed reversing the deferral but recommended committing the **A′
slice** rather than the full graph product; this ADR reflects that. Reverses a
release-scope deferral recorded in [`RELEASE-PLAN.md`](../../RELEASE-PLAN.md), not
an architectural decision — the hot-path boundary it relies on is already
Accepted ([ADR-063](063-gv2-hot-path-boundary.md)).

## Date

2026-06-08

## Context

The `v0.8.0-beta` "The Save-Time Daemon" window was scoped as the **interim-cache
slice only**: the frozen `validate_paths` wire ([ADR-061](061-save-time-daemon-delta-validation.md))
plus the `watch`/MCP clients, backed by a rebuild-on-restart `SymbolGraph` cache
(DSV Sub-phase A, now Merged 9/9 incl. cross-platform A-W). The Graph V2 hot-read
backing (Sub-phase A′), the `graph-v2-foundation` (GV2) and
`graph-context-delivery` (GCTX) modules, and warm-start persistence (Sub-phase B)
were **explicitly deferred** out of the window.

A pre-cut honesty check of what that slice actually delivers to a default user
surfaced the problem this ADR responds to:

- The `watch` daemon-routing path is **opt-in behind `ANVIL_WATCH_DAEMON`**
  (`crates/anvil-cli/src/commands/watch_save_time.rs` → `daemon_routing_enabled()`,
  default off; called from `watch.rs`) and only for `check` watches. A stock
  install sees **no change**.
- The MCP `anvil_validate_write` path still defaults to `DaemonStatus::NotWired`
  with an embedded fallback and remains on `scan_buffer`, not the new wire.
- Hot-path coverage is `check_families: ["antipattern"]` only, 1-hop, no
  persistence.

So as scoped the window is an **architectural milestone, not a user-visible
feature drop**: the engine moved, but the default user experience does not.

The deferral's prerequisites have since landed: `docs/architecture/graph-v2-foundation-spec.md`
(GV2-001, PR #2350), the stable-identity + export-diff primitive (GV2-002, PR
#2387), the delta/event contract (GV2-003, PR #2391), and the persistence ADR
(GV2-021 / [ADR-069](069-graph-v2-persistence.md), PR #2301). The A′ hot-path
boundary is Accepted ([ADR-063](063-gv2-hot-path-boundary.md), which *closes* the
A′ gate and freezes the GV2-022 hot-read API). GV2 is therefore **4/19** with its
hard blockers cleared — the deferral is now a timing choice, not a dependency
wall.

The first draft of this ADR pulled in the full GV2 product (foundation + A′ +
GCTX, ~28 items). The council (3/5 reviewers) flagged that GCTX is 0/13 and
**unproven**, with an unresolved *architectural* prerequisite (GCTX-002 — which
MCP target it lands on), so "build time, not architectural risk" did not hold for
it; and that an assistant-facing **egress** surface needs a privacy review the
2026-06-08 GV2 privacy verdict (persistence-only, PV-9) does not provide. This
ADR adopts the council's recommended scope.

## Decision

Re-scope `v0.8.0-beta` from the interim-cache slice to the **graph-backed daemon
(A′ slice)**, and deliver it to **every user by default**:

1. **GV2 A′-critical-path foundation** — the items on the `GV2-027` dependency
   closure: **GV2-010** (semantic schema), **011** (incremental hot indexes),
   **012** (trust/policy graph contract), **022** (hot-read API + guardrails),
   **024** (hot-read type split + asserts), **025** (Criterion CI gate, ADR-031),
   **028** (production parser feed), **029** (privilege containment on certify).
2. **The A→A′ backing swap** — **GV2-027** retires the interim `KernelGraphCache`
   re-derive and points `validate_paths`/`save_time` at the resident GV2 hot-read
   index under the **unchanged frozen wire**; `backing_schema_version` moves
   `interim-symbolgraph-v1` → `gv2-hotindex-v1`. Gated on a verdict-parity
   property proof, the GV2-025 Criterion gate green, **and GV2-028 Done** (until
   the parser feed lands, every `ContentModify` returns `partial`). Dependency
   chain is **7-deep** (e.g. 010→011→022→024→027, with 028 and 012→029 also
   gating 027), not a parallel batch — this is the timeline driver.
3. **Default-on save-time daemon routing** — flip `ANVIL_WATCH_DAEMON` to
   default-on for `check` watches once the A′ swap and the ADR-061 §8 correctness
   bar are green, **with rollout controls** (see below).

**Deferred to `v0.9.0`** (the next window's opener): the **GCTX** context-delivery
module (13 items) and the **non-critical-path GV2** items — **013** (control/session
contract), **014** (plan/provenance contract), **020** (multi-graph registry),
**023** (consumer query contract), **026** (reverse-impact lever). GCTX, as an
assistant-facing egress surface, additionally requires its own **context-egress
privacy review** (distinct from the persistence verdict; PV-9) as a v0.9 cut
prerequisite. Warm-start persistence (Sub-phase B, GV2-030 sealed-DTO no-leak
guard, [ADR-069](069-graph-v2-persistence.md)) stays deferred — nothing in-scope
persists (the A′ backing is rebuild-on-restart), so deferring its guard leaks
nothing.

### Default-on rollout controls (cut prerequisites)

Flipping a previously opt-in persistent daemon to default-on is a *rollout*
problem, not only a correctness gate. The flip ships only with:

- a documented **opt-out** (`ANVIL_WATCH_DAEMON=0`), exercised in the release
  runbook;
- **daemon presence handling** — default-on routing is **conditional on a live
  daemon** (or an auto-start path); it must not degrade every non-daemon user to
  constant `daemon-absent` warnings (worse than today's opt-in status quo). On
  Windows specifically, gate on the served-verb set (DSV-010b);
- a named **revert signal** (e.g. p95 over the ADR-031 budget, or WARN-rate above
  threshold) and a staged rollout (beta channel before GA).

The frozen `validate_paths` wire (ADR-061) and the hot-/non-hot-path boundary
(ADR-063) are **unchanged** — this ADR builds on them; it does not supersede
them.

The cut is **quality-gated, not calendar-gated** (release-cadence policy): cut
when the A′ slice is ready and gates are green.

## Rationale

The window exists to deliver save-time governance value, and the interim slice
delivers it only to opt-in users on one check family. The **A′ slice + default-on
flip** is the smallest scope that fixes that completely: every user gets a
persistent, graph-backed save-time validator. It backs the daemon with the real
resident GV2 model under the already-frozen wire, and the blockers that justified
deferral (spec, stable identity, boundary gate) are cleared — so its cost is
build time, not architectural risk.

The full graph *product* (multi-graph registry, consumer query contract, GCTX
context delivery) is deferred to v0.9 because GCTX is 0/13 with an unresolved
architectural prerequisite (GCTX-002) and an unmet egress-privacy review — i.e.
not "build time only" — and bundling it would put a long, unproven tail on the
critical path behind a soft escape hatch the council judged unreliable under
schedule pressure. Splitting it lets the user-visible payload ship weeks sooner
while v0.9 carries the product surface with its privacy gate.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **A′ slice + default-on (chosen)** | Every user gets a graph-backed save-time daemon; smallest scope that fully fixes the "default user sees nothing" problem; ~9 GV2 items; no unproven tail | No multi-graph registry / query contract / GCTX in v0.8.0 — the assistant-facing product lands in v0.9 |
| Expand to the full GV2 product (first draft) | One coherent graph story incl. assistant context delivery | ~28 items incl. GCTX 0/13 with unresolved GCTX-002 + missing egress-privacy review; materially later cut; escape hatch fragile (council 3/5 against) |
| Ship the interim slice, defer all graph (status quo) | Cut now; minimal risk | Default user sees nothing; flag-off antipattern-only; "minor" claim is thin |
| Flip `ANVIL_WATCH_DAEMON` default-on only, no GV2 | Smallest; real default CPU fix without graph | Still antipattern-only on the interim re-derive backing; no real graph model |

## Consequences

- **Positive:** v0.8.0-beta ships a **default-on, graph-backed** save-time
  validator — a release every user experiences — without taking on an unproven
  context-delivery surface. The A′ critical-path GV2 items also lay the
  foundation v0.9's registry/GCTX build on.
- **Negative:** The cut still slips relative to "cut the interim slice now" (the
  7-deep GV2-027 chain is the driver). The assistant-facing graph product (GCTX)
  waits for v0.9. RELEASE-PLAN, NBI, and several module statuses churn.
- **Risks:** (a) the A′ swap must prove verdict parity *and* hold the ADR-031
  budget or it regresses the very CPU problem the window exists to fix; (b)
  default-on widens the blast radius of any daemon defect; (c) GV2-028 (parser
  feed, medium-confidence) slipping silently leaves verdicts `partial`.
- **Mitigations:** GV2-027 is gated on the GV2-025 Criterion CI gate + a parity
  property test + GV2-028 Done (all explicit cut criteria); default-on ships only
  with the rollout controls above; GV2-025's CI must name a concrete quiet-box
  runner/job (its bench infra is otherwise flaky in non-TTY shells).

## References

- Council: session `council-614e422c` (5 reviewers; accept-with-changes applied),
  recorded on PR #2406.
- Related ADRs: [ADR-061](061-save-time-daemon-delta-validation.md) (frozen
  wire), [ADR-063](063-gv2-hot-path-boundary.md) (hot-path boundary — closes the
  A′ gate), [ADR-064](064-intercept-graph-cache-crate-boundary.md)
  (`anvil-graph-cache` crate), [ADR-031](031-validation-latency-rubric.md)
  (latency budget), [ADR-069](069-graph-v2-persistence.md) / GV2-021
  (persistence, stays deferred)
- APS modules: GV2 (`graph-v2-foundation`), GCTX (`graph-context-delivery`, → v0.9),
  DSV (`daemon-save-time-validation`, Sub-phase A′)
- Release: [`RELEASE-PLAN.md`](../../RELEASE-PLAN.md) (the deferral this reverses),
  [`docs/policies/release-cadence.md`](../../docs/policies/release-cadence.md)
- Spec: [`docs/architecture/graph-v2-foundation-spec.md`](../../docs/architecture/graph-v2-foundation-spec.md)
