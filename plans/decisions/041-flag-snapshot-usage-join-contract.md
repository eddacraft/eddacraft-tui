# ADR-041: Feature flag snapshot contract for usage joins

## Status

Accepted

## Date

2026-05-11

## Context

USAGE-002 needs a durable answer to how command-invocation usage rows correlate
with feature flag context. The open question had three parts:

1. whether FLAGS or the catalogue publishes a per-invocation resolved snapshot
   that USAGE can reference;
2. which identifier is stable enough to join historical usage rows to flag
   definitions after renames and retirements;
3. whether ADR-019's rule that only gate-affecting outcomes land in Kindling
   leaves non-gate-affecting flags without join data.

ADR-019 deliberately kept routine flag evaluations off Kindling. ADR-035 then
made Kindling the source-of-truth pipe for durable governance facts and tracing
the non-authoritative debugging pipe. USAGE rows are durable usage facts, but
they must not turn routine flag evaluations into standalone Kindling facts by
accident.

FLAGCAT-002 will create the catalogue package and manifest, but this decision
does not require that package to exist first. The current per-surface flag
definitions already expose the de facto stable key that the future manifest will
carry forward.

## Decision

### D-1: No separate per-invocation flag snapshot row

Neither the feature flag catalogue nor FLAGS publishes a separate
per-invocation resolved snapshot row to Kindling.

USAGE-002 stores the resolved flag context inline on the usage observation it
already emits for a user-initiated invocation. The usage producer is the
publisher of that inline snapshot because it owns the invocation boundary and
the `traceparent` correlation context.

The inline field is a deterministic snapshot, not a reference to another row:

```text
flag_set: [
  {
    key: <canonical flag key>,
    variant: <resolved variant>,
    source: <snapshot | override | default>,
    gate_affecting: <true | false>
  }
]
```

Rows sort `flag_set` by `key` so query fixtures and diffs are stable. USAGE may
add producer-local metadata such as `snapshot_version` if the resolver exposes
it, but the join contract does not depend on that metadata.

Population semantics:

- `flag_set` contains the flags the invocation boundary actually resolved or
  inherited as active context for the user-initiated command. It is not a full
  dump of every manifest entry.
- For CLI commands, this means flags consulted while authorising or routing the
  command, plus any resolved flag snapshot already attached to that invocation
  context by the caller.
- For JSON-RPC methods, this means flags resolved for the user-initiated method
  dispatch. Internal RPCs triggered by the same command do not add duplicate
  rows; they share the invocation's `traceparent`.
- If no flag is resolved or inherited for an invocation, `flag_set` is an empty
  array, not omitted.
- Producers must not re-evaluate unrelated flags solely to populate `flag_set`.
  The field records observed invocation context, not a global feature matrix.

### D-2: The manifest `key` is the stable join key

The canonical identifier for a flag definition is the manifest entry's `key`
string. The current per-surface definitions already treat `key` as the runtime
identifier; `flags/manifest.json` must preserve those strings when FLAGCAT-002
lands.

Rules:

- A rename that only changes display names, constants, accessor names, owner
  text, intent, or documentation keeps the same `key`.
- Changing `key` creates a new logical flag. The old key remains queryable in
  historical usage rows and must either stay in the manifest with retired status
  until retention expires or be covered by an explicit migration note that maps
  old key to new key for historical queries.
- Retirement does not reuse a key. Retired keys remain reserved forever for
  historical joins.
- `createdFor` remains mandatory provenance, but it is not a join key. Multiple
  flags can share an APS work item and a flag can outlive that work item's
  planning context.

### D-3: ADR-019 remains gate-affecting-only for Kindling flag facts

ADR-019 is not widened. Non-gate-affecting flags do not get standalone Kindling
join rows merely because USAGE wants context.

The inline `flag_set` on a usage row can include non-gate-affecting flags as
context for that invocation, but those entries are not independent flag
evaluation facts. They only answer "what flag context was active when this
command ran?" They do not answer "which non-gate flag evaluations happened?"

For gate-affecting outcomes, joins use the canonical `key` shared by the usage
row's inline `flag_set` and ADR-019's `flags_consulted` field on
`gate_evaluated` or the equivalent constraint observation. For
non-gate-affecting flags, USAGE has inline context only; there is no separate
Kindling row to join to.

## Rationale

### Alternatives considered

| Option | Pros | Cons |
|--------|------|------|
| Inline snapshot on the usage row (chosen) | Unblocks USAGE-002 without implementing FLAGCAT-002 first; keeps one durable row per user invocation; avoids a new high-volume Kindling observation kind | Duplicates small flag context across usage rows; non-gate flags have context but no independent evaluation history |
| Separate per-invocation snapshot row | Normalised shape; USAGE rows could reference a row ID | Adds another Kindling write per invocation and effectively creates routine flag-evaluation provenance, weakening ADR-019's boundary |
| Reference only ADR-019 `flags_consulted` rows | No duplication; preserves existing Kindling facts | Only works when a gate fires; commands with non-gate flags or no gate-affecting outcome lose their active flag context |
| Use generated UUIDs as flag IDs | Allows display-key renames without historical ambiguity | Requires FLAGCAT-002 implementation before USAGE can proceed; adds another identifier when the runtime key already exists |
| Use `createdFor` as the join key | Connects directly to APS provenance | Not unique enough, not a runtime identifier, and unsuitable after work items archive |

## Consequences

- **Positive:** USAGE-002 can move forward without waiting for FLAGCAT-002 to
  exist. The future catalogue must preserve the already-shipped key strings,
  but it does not need to invent a new identifier.
- **Positive:** ADR-019's Kindling noise boundary remains intact. USAGE adds
  invocation context to usage rows, not standalone routine flag facts.
- **Negative:** Historical rows duplicate the resolved flag context instead of
  normalising it through a snapshot table. This is accepted because the flag set
  is small and query stability matters more than normalisation.
- **Risk:** A future contributor might treat inline non-gate entries as proof
  that a non-gate flag evaluation happened. Mitigation: USAGE docs and fixtures
  must describe the field as invocation context only.
- **Risk:** A future catalogue migration may be tempted to rename keys. Mitigation:
  FLAGCAT tasks and the feature flag governance guide must treat key changes as
  new logical flags, not refactors.

## References

- [ADR-019](019-flags-observability-alignment.md) - feature flag telemetry and
  Kindling boundary
- [ADR-035](035-three-pipe-observability-rule.md) - three-pipe observability rule
- [USAGE module](../archive/modules/usage-analytics.aps.md) - USAGE-002 flag-context
  correlation
- [FLAGCAT module](../modules/feature-flag-catalogue.aps.md) - FLAGCAT-007
  contract task
