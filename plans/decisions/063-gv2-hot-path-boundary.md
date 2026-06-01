# ADR-063: Graph V2 hot-/non-hot-path read boundary

## Status

**Proposed** — ratification by the **INTD, DRVR, and GV2 owners** closes the
sub-phase A′ gate that [ADR-061](061-save-time-daemon-delta-validation.md) §9
leaves open ("hot-/non-hot-path boundary agreed with INTD and DRVR owners").
Until all three accept, no hot-read API field (GV2-022) may be frozen.

## Date

2026-06-01

## Context

[ADR-061](061-save-time-daemon-delta-validation.md) makes the intercept daemon
the save-time validation authority via a verdict-shaped, frozen `validate_paths`
wire. Sub-phase A ships that wire over an **interim SymbolGraph cache**
(rebuild-on-restart). Sub-phase A′ swaps the backing to the **Graph V2 warm
hot-read slice** (GV2-010/011/020/022) **under the unchanged wire** — but ADR-061
explicitly does **not** close the boundary: it forbids freezing any hot-read API
field until INTD + DRVR + GV2 ratify *which* graph reads are allowed on the save-
time hot path.

The hot path is latency-critical. ADR-061 §6 requires certifiability from a
**bounded reverse-impact closure** and states the hot path does **no parse,
resolve, or transitive traversal**; the latency budget itself lives in
[ADR-031](031-validation-latency-rubric.md). GV2-011 and GV2-022 already split
"warmed indexes the daemon may read on the hot path" from "transitive impact
traversal [which] remains explicitly non-hot-path", but the **admission rule, the
operation allowlist, and the miss/stale policy** are not yet agreed across the
three owning surfaces:

- **INTD** (intercept daemon) executes reads inside `validate_paths`.
- **DRVR** (surface drivers, ADR-030) issue mid-edit reads on the same indexes.
- **GV2** owns the index shapes and the `hot_index.rs` read API.

Without one agreed rule, each surface could admit a different, creeping set of
"cheap enough" reads, and the latency budget would erode by accretion — the
"hot path accidentally includes expensive traversal" risk GV2 already flags.

## Decision

A Graph V2 read is **hot-path-admissible** — i.e. it MAY execute inside a
`validate_paths` (or a driver mid-edit) call, against the ADR-031 budget — **if
and only if** it satisfies the **admission invariant**:

> It is answerable from **resident warm indexes** in **O(1) or O(bounded fan-out)**
> with **no parse, no cross-file symbol resolution, no transitive traversal beyond
> a single reverse-impact hop, and no blocking I/O.**

Everything else is **non-hot-path** and runs only in the background pool.

### Hot-path allowlist (the only admissible reads)

1. **Resident per-file symbol/extract lookup** — read the warm GV2-010 record for
   an already-extracted file. (A file whose extract is absent/stale is a miss; see
   miss policy.)
2. **Known-edge existence** — "does edge `A→B` exist?" answered O(1)/O(degree)
   from the warm dependency/boundary index (GV2-011).
3. **One-hop reverse impact** — `dependents_of(symbol|file)` to exactly **1 hop**,
   the ADR-061 §6 certifiability closure. The result set is capped; overflow is a
   miss (`impact-set-overflow`), never a deeper walk.
4. **Precomputed architectural-index check** — resident layer/boundary membership
   and rule-compliance flags (GV2-011), not recomputed on read.

Each returns an explicit **`warm` / `stale`** marker (GV2-022).

### Non-hot-path denylist (background pool only)

Parsing / re-extraction; cross-file symbol resolution; transitive impact or
reachability beyond 1 hop; full-graph scans; index (re)builds; persistence load/
store (GV2-021); any read requiring disk or network.

### Miss/stale policy (the load-bearing rule)

A hot read that cannot be served from warm, resident state **MUST** return a
typed `stale`/`warm-miss` result that maps to an ADR-061 `StaleReason`
(`cross-file-resolution-needed`, `warm-state-evicted`, `impact-set-overflow`, …)
and degrades to the existing fallback. It **MUST NOT** escalate to an on-hot-path
parse, resolve, traversal, rebuild, or I/O. "Slower but complete" is never a
hot-path option — completeness is the background pool's job; the hot path trades
coverage for a bounded, honest verdict.

### Invariants binding all three owners

- **One admission rule, one allowlist.** INTD and DRVR consume GV2-022 through the
  GV2-023 consumer contract; neither adds surface-local "cheap" reads. New
  admissible operations require a GV2-022 change *and* an amendment to this ADR.
- **Behind the frozen wire.** This boundary lives entirely behind the verdict-
  shaped `validate_paths` wire (ADR-061). Sub-phase A → A′ swaps the backing with
  **zero** wire change; the boundary is an internal contract, not a wire field.
- **Enforced, not aspirational.** Admissibility is guarded by (a) a GV2-022 type
  split so non-admissible ops are not even callable from the hot-read API, and
  (b) the ADR-031 Criterion benchmark (GV2-011/-022 validation) that fails CI on
  budget regression. A debug assertion trips if a hot call performs parse/resolve/
  traverse/I/O.

## Rationale

The save-time contract's value is a **bounded, honest** verdict on every save; the
moment a "rare" hot read can fall back to parsing or a multi-hop walk, the tail
latency and the single-pool oversubscription ADR-061 set out to kill both return.
Pinning admissibility to *resident warm reads with a miss-degrades-to-fallback
rule* keeps the hot path's worst case bounded by construction, and makes the
A→A′ backing swap safe because the wire never learns what the backing is.

Fixing the rule **once, across all three surfaces** is the point of the gate:
INTD, DRVR, and GV2 each touch these indexes, so a boundary owned by only one of
them would drift. The 1-hop reverse-impact cap is chosen because it is exactly the
certifiability closure ADR-061 §6 already commits to — no new latency claim is
introduced here.

### Alternatives Considered

- **Allow on-hot-path rebuild/parse on a warm miss ("slower but complete").**
  Rejected — reintroduces unbounded tail latency and the oversubscription ADR-061
  removes; the daemon-absent fallback already provides completeness off the hot
  path.
- **Let drivers (DRVR) traverse the graph directly for mid-edit richness.**
  Rejected — a second admission policy that drifts from the daemon's; mid-edit
  reads go through the same GV2-022 allowlist.
- **Budget-only boundary (admit anything that fits ADR-031 today).** Rejected —
  "fits today" erodes as graphs grow; the boundary must be a *shape* rule
  (resident/bounded), with ADR-031 as the regression guard, not the definition.
- **Defer the boundary until Graph V2 lands.** Rejected — it is precisely the
  blocker on freezing GV2-022; A′ cannot start without it.

## Consequences

- **Positive:** unblocks ADR-061 sub-phase A′ and the freeze of GV2-022; gives
  INTD, DRVR, and GV2 one shared, testable admission rule; keeps the hot path's
  worst case bounded by construction; the A→A′ backing swap stays wire-invisible.
- **Negative:** the hot path is deliberately *incomplete* — some real cross-file
  or deep-impact issues surface only via background full validation, not at save
  time. This is the intended trade (honest-and-bounded over complete-and-variable).
- **Risks:** allowlist creep (a surface lobbies for "just one more" read); the
  1-hop cap missing genuine 2-hop breakage that users expect at save time.
- **Mitigations:** allowlist changes require this ADR + GV2-022 to move together;
  the ADR-031 benchmark + the hot-path debug assertion fail CI on violation; the
  background scheduler (sub-phase B) closes the coverage gap asynchronously, and
  `workspace_status` reports `stale` honestly in the meantime.

## References

- Related ADRs: [ADR-061](061-save-time-daemon-delta-validation.md) (save-time
  daemon delta validation — opens this gate), ADR-031 (validation-latency rubric),
  ADR-030 (surface drivers / DRVR), ADR-015 (intercept loop)
- Spec: [`plans/specs/2026-06-01-daemon-save-time-validation-contract.md`](../specs/2026-06-01-daemon-save-time-validation-contract.md)
- APS modules: GV2-010, GV2-011, GV2-020, GV2-022, GV2-023
  ([`plans/modules/graph-v2-foundation.aps.md`](../modules/graph-v2-foundation.aps.md));
  RTAI, RLB (consumers of the daemon hot path)
- Ratification: closes the INTD + DRVR + GV2 boundary gate in ADR-061 §9
