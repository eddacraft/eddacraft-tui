# ADR-077: Save-time certifiability closure is hard-depth-capped

## Status

**Proposed** — 2026-06-09. Resolves the owner/council decision GV2-027
([PR #2446](https://github.com/eddacraft/anvil-001/pull/2446)) deferred and
flagged in `HotReadApi::certify`'s doc comment. Refines — does not supersede —
[ADR-061](061-save-time-daemon-delta-validation.md) §6 and
[ADR-063](063-gv2-hot-path-boundary.md) §3. Ratification gates GV2-024
(hot-read type split + seal).

## Date

2026-06-09

## Context

The A→A′ backing swap (GV2-027) routed `validate_paths` certification through
`HotReadApi::certify` (`crates/anvil-graph-cache/src/hot_index.rs`), which
delegates to `crate::certify::certify`. That swap was deliberately
verdict-preserving and left one decision unmade, because it is verdict-affecting
and two Accepted ADRs are in genuine tension on the save-time hot path:

- **[ADR-061](061-save-time-daemon-delta-validation.md) §6** defines
  certifiability as a "bounded reverse-impact closure". The affected set is the
  1-hop importer set `dependents_of(file)`, **but "re-exports recurse, bounded by
  budget"** — i.e. the closure walks transitively to *unbounded depth*, capped
  only by the file-count `budget`. This is what `impact_closure`
  (`certify.rs:444`) does today: a frontier walk terminated solely by `seen`
  dedup (cycle safety) and `len() > budget` (returns `None`).
- **[ADR-063](063-gv2-hot-path-boundary.md) §3** (hot-path allowlist #3) requires
  reverse impact to be **hard-depth-capped** (`MAX_REVERSE_IMPACT_DEPTH = 2`,
  default lever 1 hop), and states a closure exceeding the configured depth is a
  **miss (`impact-set-overflow`), never an unbounded walk**. This is what
  `HotReadApi::reverse_impact` (`hot_index.rs:204`) implements.

So `HotReadApi` carries **two reverse-traversal models**: every allowlist read is
hard-capped, but `certify` reaches an unbounded transitive walk through
`impact_closure`. GV2-024 must seal the hot-read surface so non-admissible
(unbounded-traversal) ops are uncallable from the hot type — which forces a
choice for `certify` it cannot make silently. The `certify` doc comment records
the fork: **either** replace the body with a depth-capped closure **or** exclude
`certify` from the seal with a recorded ADR-061/063 rationale.

### What actually differs between the two models

The divergence is narrower than "verdict change". `certify` uses the closure
**only to size the stale reason**, and never inline-validates it
(`certify.rs`, surface-change branch):

```rust
match impact_closure(dep, &delta.file, budget) {
    None    => Partial { reason: ImpactSetOverflow },     // closure > budget
    Some(_) => Partial { reason: ExportSurfaceChange },   // set discarded
}
```

Both arms are `Partial`; the `Some(_)` set is discarded. A surface change is
**never** self-certified in the as-built — every surface change is `Partial` and
reconciled by a background full scan. The closure therefore affects **only which
`StaleReason` is reported**, not the `certified | partial` coverage verdict and
not soundness.

Because a depth-capped closure is a subset of the unbounded one, capping can only
make the closure *smaller*, so it can only turn the internal `CertifyStale`
reason `ImpactSetOverflow` → `ExportSurfaceChange` (monotone, never the reverse),
and only for re-export chains **deeper than 2 hops** whose total importer count
exceeds `budget` — a narrow, rare shape.

On the **wire**, the shift is `impact-set-overflow` → `cross-file-resolution-needed`:
`wire_stale_reason` (`validate_paths.rs:90`) already collapses
`CertifyStale::ExportSurfaceChange` (and `UnreliableGraph`) onto
`StaleReason::CrossFileResolutionNeeded`, so the affected saves join the
existing cross-file-resolution bucket rather than introducing a new wire value.
The only consumers of the distinction are **string renderers** for display and
telemetry (`watch_save_time.rs:319`, `telemetry.rs:614`,
`anvil-intercept-proto/src/protocol.rs:743`) — none branch on it behaviourally.

## Decision

**On the save-time hot path, the certifiability reverse-impact closure is bounded
by the ADR-063 hard depth cap (`MAX_REVERSE_IMPACT_DEPTH`) in addition to the
file-count budget.** ADR-061 §6's "re-exports recurse, bounded by budget" is
refined to "re-exports recurse, bounded by the hard depth cap **and** the
budget." ADR-063 §3 governs all reverse traversal on the hot path, with no
exception for `certify`.

Concretely (implemented by GV2-024, not this ADR):

1. `crate::certify::certify`'s `impact_closure` walk is depth-bounded to
   `MAX_REVERSE_IMPACT_DEPTH`, matching `HotReadApi::reverse_impact`'s model.
   Over-cap chains are truncated; the (possibly truncated) closure size selects
   `ImpactSetOverflow` vs `ExportSurfaceChange` exactly as today.
2. This takes **path A** of the `HotReadApi::certify` hand-off: the body adopts
   the depth-capped closure, and the GV2-024 seal then covers the whole
   `HotReadApi` uniformly — no unbounded transitive traversal is reachable from
   the hot type, so the seal needs no documented exception.
3. The cap applies identically to the warm (`HotReadApi::certify`) and cold
   (`KernelGraphCache` rebuild) backings, since both share `impact_closure`. The
   GV2-027 `backing_parity` property test is re-baselined against the capped
   closure; warm and cold stay verdict-identical by construction.
4. The depth stays fixed at the hard cap for the certifiability closure. The
   **runtime-configurable** 1→2-hop lever (ADR-063 §3) remains GV2-026's scope
   and is out of scope here; coupling the wire-visible verdict reason to a
   deployment knob is deferred with it.

## Rationale

The save-time contract's whole posture (ADR-061, ADR-063) is **"bounded and
honest over complete and variable."** ADR-063 exists precisely to remove
unbounded transitive walks from the hot path: *"the moment a 'rare' hot read can
fall back to ... a multi-hop walk, the tail explodes."* Leaving `certify` on an
unbounded `impact_closure` keeps exactly that hole open — a pathological deep
re-export chain that fits the budget walks every hop on every save, inside the
interactive latency boundary ADR-031 guards.

Capping closes the hole at **near-zero behavioural cost**: the coverage verdict
is unchanged, soundness is unchanged (the closure sizes a reason but is not
inline-validated), and the only observable effect is that some >2-hop graphs
report the more specific `export-surface-change` rather than the give-up
`impact-set-overflow`. Either reason already routes to the same background full
scan (the wire reason joins the existing `cross-file-resolution-needed` bucket),
so no downstream consumer loses coverage. Capping also lets GV2-024 seal
the hot-read surface uniformly instead of carving out a documented exception that
future readers must keep re-justifying.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Cap the closure (chosen)** — ADR-063 §3 governs the hot path; `certify` adopts the depth-capped walk | Closes the unbounded-tail hole on the certify path; uniform GV2-024 seal, no exception; coverage-verdict- and soundness-preserving; warm/cold parity by construction | Wire-visible `StaleReason` changes for >2-hop over-budget graphs (`impact-set-overflow` → `export-surface-change`); `backing_parity` must be re-baselined |
| **Exclude `certify` from the seal** — keep unbounded `impact_closure`, document `certify` as an admissible composite (ADR-061 §6 governs) | Zero code and zero wire change; retains the most-specific reason for deep graphs | Leaves an unbounded transitive walk reachable on the hot path — the exact tail-latency hole ADR-063 exists to close; seal becomes non-uniform (a standing documented exception); deep in-budget re-export chains walk every hop every save |
| **Honour the configurable depth lever in `certify`** — thread ADR-063's runtime 1→2-hop lever into the verdict | Operationally tunable in one place | Couples a wire-visible verdict reason to a deployment knob (a depth change silently re-labels `StaleReason`); widens the surface GV2-024 must reason about; the lever is GV2-026's scope, not yet built |

## Consequences

- **Positive:** The ADR-063 hot-path boundary becomes fully enforced — no
  unbounded reverse traversal survives anywhere on the save-time hot path,
  including `certify`. GV2-024 seals `HotReadApi` uniformly. The §6-vs-§3 tension
  is resolved in the decision record rather than carried as a doc-comment caveat.
- **Positive:** Tail latency on pathological deep re-export graphs is bounded by
  construction (≤ 2 hops × budget), consistent with ADR-031.
- **Negative:** The wire-visible `StaleReason` (ADR-061 §5) changes value for a
  narrow class of saves (>2-hop re-export chains over budget):
  `impact-set-overflow` → `cross-file-resolution-needed` (no *new* enum value —
  it joins the existing bucket via `ExportSurfaceChange`'s wire mapping). Both
  already trigger a background full scan, so coverage is unchanged; the only
  affected consumers are display/telemetry string renderers, but any external
  tooling that *counts* the specific reason sees the shift.
- **Risks:** A future change re-raises `MAX_REVERSE_IMPACT_DEPTH` and reasons the
  certify closure is "still bounded" while the tail grows; the depth-lever
  coupling deferred here is mistaken for already-decided.
- **Mitigations:** The cap is the single `MAX_REVERSE_IMPACT_DEPTH` constant
  already gated by the GV2-025 ADR-031 Criterion benchmark; the re-baselined
  `backing_parity` test pins warm/cold verdict identity under the cap; GV2-026
  owns any move of the configurable lever and must cite this ADR.

## References

- Related ADRs: [ADR-061](061-save-time-daemon-delta-validation.md) §5/§6,
  [ADR-063](063-gv2-hot-path-boundary.md) §3,
  [ADR-031](031-validation-latency-rubric.md)
- APS modules: GV2-024 (implements), GV2-027 (deferred this decision), GV2-026
  (owns the configurable depth lever), GV2-025 (latency gate)
- Post-merge plan: `plans/reviews/post-merge/gv2-027-aprime-swap.md` (owner
  decision, step 3)
- Code: `crates/anvil-graph-cache/src/hot_index.rs`
  (`HotReadApi::certify`, `reverse_impact`, `MAX_REVERSE_IMPACT_DEPTH`),
  `crates/anvil-graph-cache/src/certify.rs` (`impact_closure`, `certify`)
