# ADR-019: Feature flag telemetry alignment with OBS and Kindling

## Status

Proposed

## Date

2026-04-12

## Context

Three observability layers exist in Anvil, each with a different purpose:

1. **Kindling** — system of record for session/gate/action observations. Already
   built. 11 observation kinds, 4 query scopes. Write-once facts, no inference.
   Stored in SQLite.

2. **OBS (observability-foundation)** — Draft module defining production health
   signals, alert thresholds, dashboard data contracts, and runbooks. Not yet
   Ready. Exposes a canonical observability contract (OBS-001) that hasn't been
   written yet.

3. **FLAGS-006** — task within the feature-flagging module that defines OTEL
   telemetry and audit contracts for flag evaluation. References
   `observability-foundation.aps.md` in its scope but doesn't specify how it
   relates to OBS-001's event contract or to Kindling's observation model.

Two questions need answering before FLAGS work begins:

**Q1: Should FLAGS-006 define its own OTEL conventions or wait for OBS-001?**
OBS is still Draft and has no ready checklist items ticked. FLAGS is Ready and
high priority. If FLAGS-006 waits, it blocks a Ready module on a Draft one. If
it proceeds independently, OBS-001 may later define conventions that conflict.

**Q2: Should feature flag evaluations emit Kindling observations?**
Kindling already has `gate_evaluated` — gate checks record their result as
immutable facts. Feature flag evaluations are conceptually similar (a runtime
decision with inputs and an outcome), but they're higher frequency and lower
stakes than gate checks. Emitting every flag evaluation to Kindling could create
noise; omitting them entirely means flag decisions aren't in the system of
record.

## Decision

### Q1: FLAGS-006 defines a thin OTEL convention; OBS-001 ratifies or extends it

FLAGS-006 proceeds without waiting for OBS-001. It defines a minimal, namespaced
OTEL convention scoped to feature flags:

- **Namespace prefix:** `anvil.flags.*`
- **Session-start metric:** `anvil.flags.snapshot_loaded` — emitted once per
  session with snapshot version, environment, runtime, and feature count
- **Per-feature metric:** `anvil.flags.evaluated` — emitted on first evaluation
  of each feature per session with feature key, resolved variant, resolution
  source (snapshot/override/default), and coarse tier/channel
- **Debug span:** `anvil.flags.evaluation_detail` — available on demand only,
  includes full rule-matching trace

When OBS-001 is written, it adopts the `anvil.flags.*` namespace as-is or
negotiates changes. FLAGS-006's convention becomes the reference input for the
signals inventory, not the other way round.

**Constraint:** FLAGS-006 must not define conventions outside the `anvil.flags.*`
namespace. General signal semantics (error rate, latency, severity levels) remain
OBS-001's responsibility.

### Q2: Flag evaluations emit Kindling observations only at gate boundaries

Feature flag evaluations do **not** emit standalone Kindling observations.
Instead:

- When a gate check's outcome is influenced by a feature flag, the existing
  `gate_evaluated` observation includes a `flags_consulted` field listing the
  flag keys, resolved variants, and resolution sources that contributed to the
  gate result.
- When a kill-switch fires, a dedicated `constraint_applied` observation is
  emitted (this observation kind already exists in Kindling's contract and fits
  semantically — an action was prevented by a constraint).
- Routine flag evaluations outside gate boundaries (e.g. UI rendering, CLI
  licence checks) are covered by OTEL metrics only, not Kindling.

This keeps Kindling's signal-to-noise ratio intact while ensuring flag decisions
that affect governance outcomes are part of the provenance chain.

## Rationale

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| FLAGS-006 defines thin convention, OBS-001 ratifies (chosen) | Unblocks FLAGS; sets precedent for other modules to contribute conventions; OBS-001 gets concrete input | OBS-001 may want different naming; small risk of rework |
| FLAGS-006 waits for OBS-001 | Perfect alignment from day one | Blocks a Ready/high-priority module on a Draft one; OBS timeline is uncertain |
| FLAGS-006 ignores OBS entirely | Maximum independence | Guaranteed convention mismatch; duplicated effort later |
| Emit all flag evaluations to Kindling | Complete provenance | High volume; most evaluations are routine and don't affect governance; degrades query usefulness |
| Emit no flag evaluations to Kindling (chosen for routine) | Clean separation | Flag-influenced gate outcomes lose context without the `flags_consulted` enrichment |
| New Kindling observation kind `flag_evaluated` | Explicit, queryable | Adds a 12th observation kind for something that's better served by OTEL metrics; Kindling scope creep |

## Consequences

- **Positive:** FLAGS module is unblocked. OBS-001 gets a concrete, tested
  convention to adopt rather than designing in a vacuum. Kindling stays focused
  on governance-relevant facts. Kill-switch activations are in the provenance
  chain.
- **Negative:** Small risk that OBS-001 renames `anvil.flags.*` attributes,
  requiring a migration. Mitigated by keeping the surface minimal (two metrics,
  one debug span).
- **Risks:** If other modules (e.g. APGOV, KERN) follow the same pattern and
  define their own `anvil.<domain>.*` conventions before OBS-001, the signals
  inventory becomes a ratification exercise rather than a design exercise. This
  is acceptable if each module's convention is small and namespaced.
- **Mitigations:** OBS-001's scope statement should be updated to note that it
  will collect and ratify domain-specific conventions contributed by other
  modules, not design them from scratch.

## References

- APS modules: FLAGS (feature-flagging), OBS (observability-foundation)
- Kindling contracts: `packages/kindling-integration/CONTRACTS.md`
- FLAGS-006 task: `plans/modules/feature-flagging.aps.md`
- OBS-001 task: `plans/modules/observability-foundation.aps.md`
- Related ADR: ADR-018 (product/IP architecture — tier gating consumes auth context)
