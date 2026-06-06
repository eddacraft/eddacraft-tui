# ADR-073: Durable `anvil/` vs Local `.anvil/` State Boundary

## Status

Proposed

## Date

2026-06-06

## Context

Anvil writes to two top-level directories, and the split between them has been
applied *de facto* but never stated as a decision — so it is applied
inconsistently.

The intended, already-shipped convention:

```text
anvil/   = tracked, durable governance state that must travel with the repo
.anvil/  = local runtime state (cache, SQLite, logs, scratch) — gitignored
```

Evidence the boundary already exists in code:

- `anvil-witness` deliberately lives under `anvil/` so the chain "survives
  `git worktree add`" (`crates/anvil-witness/src/lib.rs` module docs; ADR-037).
- `anvil/baseline.json` (ADR-039) and `anvil/project-id` are tracked.
- `anvil/drift/edges.ndjson` is tracked, trunk-shared evidence (ADR-052).
- `.gitignore` ignores `.anvil/cache/`, `.anvil/logs/`, `.anvil/snapshots/`,
  `.anvil/first-run`, `.anvil/release-state.json`, `.anvil/gates.json`, etc.

Two places **violate** the convention today:

1. **Exceptions** — `anvil-policy`'s `ExceptionStore` reads/writes
   `.anvil/exceptions.json` (`crates/anvil-policy/src/exceptions.rs`). Because
   `.anvil/` is local/gitignored, policy exceptions **do not travel with the
   repository, are invisible in PR review, and vanish on a fresh clone.** That
   directly contradicts the governance posture (an exception is a deliberate,
   attributable deviation that reviewers and auditors must see). Note this is
   distinct from the inline `@anvil-ignore` suppression syntax (ADR-004), which
   is already in-tree because it lives in source comments.
2. **Edda memory** — the Edda store defaults to `.anvil/edda/`
   (`packages/edda-stack/src/edda/config.ts`), even though Edda is *durable,
   human-curated institutional memory* that should be tracked. The Edda
   component README still describes `.anvil/edda/` as the tracked store, which
   conflicts with this boundary.

This ADR ratifies the boundary and records the two reconciliations as required
follow-up, rather than letting the conflict persist silently.

## Decision

**`anvil/` is tracked durable governance/memory/evidence state. `.anvil/` is
local runtime state and is gitignored.** New durable governance artefacts MUST
be written under `anvil/`; new runtime/cache/log/database artefacts MUST be
written under `.anvil/`.

Classification:

| State | Tree | Tracked? |
|-------|------|----------|
| Witness chain, baseline, project-id, drift ledger | `anvil/` | yes (already) |
| Policy, rules, config-derived policy digests | `anvil/` (config) / `.anvil/cache/policy` (compiled) | source yes; cache no |
| **Policy exceptions** | **`anvil/exceptions/`** | **yes (moves — EXCEPT)** |
| **Edda institutional memory + sealed provenance** | **`anvil/edda/`** | **yes (moves — EDDA-SEAL)** |
| Review capsules (when staged in-repo) | `anvil/evidence/capsules/` | yes (on request) |
| Release attestations | `anvil/releases/` | yes (future) |
| Kindling DB, Ember DB | `.anvil/kindling.db`, `.anvil/ember.db` | no |
| Graph-V2 cache, policy cache, snapshots | `.anvil/cache/`, `.anvil/snapshots/` | no |
| Daemon runtime, logs, scratch, first-run, gates state | `.anvil/runtime`, `.anvil/logs`, … | no |

**Required reconciliations (tracked as work, not done in this ADR):**

- **EXCEPT-001/002:** move `ExceptionStore` to a tracked path under `anvil/`
  with a backward-compatible read of the legacy `.anvil/exceptions.json` and a
  one-time migration. Enrich the schema (owner/attribution, revocation audit
  trail) per the EXCEPT module.
- **EDDA-SEAL-001:** change the Edda default storage path to `anvil/edda/`,
  migrate existing stores, and update the Edda README/config docs. Sequenced
  after the known Edda correctness fixes (see the EDDA-SEAL roadmap item).

Migration discipline: moves provide a read-fallback to the legacy path and do
**not** delete legacy data automatically; the user is given an explicit cleanup
step. Schemas are versioned so a migration is detectable.

## Rationale

The boundary is already the design intent and is already enforced for the
highest-value artefacts (witness, baseline, drift). Stating it makes the two
violations visible and fixable, and gives every future evidence type an
unambiguous home. Leaving exceptions in `.anvil/` is not a neutral status quo —
it actively breaks the "exceptions must be reviewable" property that motivates
first-class exceptions at all.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| Ratify `anvil/` durable, `.anvil/` local; reconcile the two violations (chosen) | Matches shipped design; fixes a live governance gap; one rule everywhere | Requires migration code + a docs sweep for exceptions and Edda |
| Leave exceptions/Edda in `.anvil/` (gitignored) | No migration work | Exceptions/memory don't travel with the repo — defeats their purpose; perpetuates the documented conflict |
| Make everything tracked under `anvil/` (incl. caches/DBs) | One tree | Pollutes history with high-churn, regenerable, secret-bearing runtime state; the witness/Graph privacy lines (ADR-069) exist precisely to avoid this |
| Invert (local-first under `.anvil/`, opt-in tracking) | Minimal default footprint | Witness/baseline/drift already rely on tracked `anvil/`; would regress shipped behaviour |

## Consequences

- **Positive:** One stated, testable rule for where state lives; fixes the
  exception-portability bug; aligns Edda storage with its "durable memory" role.
- **Positive:** Gives capsules, sealed provenance, and release attestations an
  unambiguous default location under `anvil/`.
- **Negative:** Two migrations (exceptions now, Edda later) with read-fallback
  and docs updates; a `.gitignore` review to ensure nothing under `anvil/` is
  accidentally ignored and nothing high-churn under `.anvil/` is accidentally
  tracked.
- **Risks:** A migration that deletes legacy data, or a path move that strands a
  consumer still reading the old location.
- **Mitigations:** Read-fallback to legacy paths; no auto-delete; versioned
  schemas; the EXCEPT slice ships with migration tests; Edda move is sequenced
  behind its correctness fixes.

## References

- Related ADRs: ADR-001 (local-first), ADR-004 (`@anvil-ignore` suppressions —
  the in-source sibling of file-based exceptions), ADR-037 (witness under
  `anvil/`), ADR-039 (baseline), ADR-052 (drift ledger), ADR-069 (runtime cache
  privacy line), ADR-072 (Git substrate)
- APS modules: EXCEPT (`plans/modules/git-native-exceptions.aps.md`),
  EDDA-SEAL (future), GITGOV (`plans/modules/git-native-governance.aps.md`)
- Code: `crates/anvil-policy/src/exceptions.rs`,
  `packages/edda-stack/src/edda/config.ts`
