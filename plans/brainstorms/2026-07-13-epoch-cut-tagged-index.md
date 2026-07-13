# Epoch Cut: Trim + Archive + Tagged Central Index

| Field   | Value      |
| ------- | ---------- |
| Status  | Draft      |
| Created | 2026-07-13 |
| Source  | anvil-plan-spec session (post-v0.5.0 upstream release) |

## Decision Proposed

Keep **one canonical `plans/index.aps.md`** and adopt the **tagged monorepo
tier** (upstream `docs/monorepo.md`: `Packages:` scoping + by-package views).
Do **not** migrate to nested/federated child indexes.

**Why tags, not federation:** anvil-001 is one team, one backlog, one release
train, and its work items *regularly span packages* — the exact case where a
federation fragments items (or leaves them in the parent, gaining nothing)
while tags simply label them. Nested plans (upstream v0.5.0) are for packages
with independent owners/backlogs/cadences, which anvil-001 does not have.

**Recorded trigger to revisit:** a package or app acquires its own owner or
release cadence (e.g. an SDK split to an external consumer cycle). Until that
happens, this decision should not be re-litigated.

## Current State (measured 2026-07-13)

- `plans/index.aps.md`: 1,291 lines. Active modules: **87** (39 In Progress,
  24 Draft, 11 Proposed, 8 Ready, 2 Blocked). Archived: 148.
- `Packages:` scoping used by **1 of 87** active modules — the tagged tier was
  extracted upstream from this repo's pain but never adopted here (and until
  now had no CLI tooling upstream either; see "Upstream dependency" below).
- No module file is >60 days untouched, so the trim must be **semantic triage**
  (below), not a mechanical staleness cut.

## Part 1 — Trim the WIP surface

39 In Progress modules is the disease; index size is the symptom. Proposed
buckets (owner to confirm each — recommendations only):

### A. Release-legacy — verify against release records, then close/archive

| Module | Last touch |
| --- | --- |
| early-access-migration | 2026-05-29 |
| early-access-tests | 2026-05-29 |
| v050-release-followups | 2026-05-31 |
| v060-release-candidates | 2026-06-21 |

### B. Dormant 3–6 weeks — triage: still real? demote to Ready/Draft or archive

| Module | Last touch |
| --- | --- |
| feature-flag-catalogue | 2026-06-09 |
| realtime-ai-validation | 2026-06-13 |
| rust-cli-tier3 | 2026-06-17 |
| aps-dashboard-starter | 2026-06-18 |
| lang-python | 2026-06-18 |
| lang-tail-wave | 2026-06-18 |
| public-docs-site-host | 2026-06-21 |
| rust-mcp-full-port | 2026-06-21 |
| tui-next | 2026-06-22 |
| lang-tail-wave-2 | 2026-06-30 |

### C. Hardening-adjacent — do NOT archive; re-home under the new conductor

These are the raw material of the hardening phase. Keep them as vertical
modules and let the new conductor coordinate them (see Part 3):

dev-environment-hardening, security, insecure-construction-catalogue,
multilayer-protection-v2, daemon-protection-observability,
compliance-policy-packs, opa-enhancements, tracing-foundation,
test-coverage-uplift, eval-regression-ci-gate.

### D. Actively hot (touched ≤ 7 days) — keep In Progress

activation-tui, first-run-wow, activation-mcp-optional, cli-command-truth,
continuous-improvement-backlog, release-user-journeys, surface-dockerfile,
surface-github-actions, surface-shell, surface-sql-migrations,
git-native-exceptions, resource-load-benchmarking,
architecture-config-validation, documentation-sync, rust-cli-tier2,
dashboard-* (Ready set).

Also triage the 8 Ready modules (api-governance, edge,
lineage-authorship-confidence are ~6 weeks untouched) and the 24 Draft / 11
Proposed with the same lens.

## Part 2 — Archive mechanics (existing conventions, atomic per module)

Per repo discipline: `git mv` to `plans/archive/modules/`, update the index
row/path, and freeze a `completed-index.aps.md` row **in the same commit**.
Anything closed as "shipped" needs its release-record evidence line first
(status dialect: `Merged → Released/Shipped → Complete/Archived`).

## Part 3 — New phase skeleton (hardening + team features)

- **`hardening` conductor module** (`Type: Conductor`, likely `Recurring`) —
  coordinates bucket C via `## Coordinated Modules` / `## Cross-Module Work
  Items` instead of owning everything. This is the upstream v0.4.0 conductor
  pattern; upstream lint (W002/W006) validates the references.
- **`TEAM` vertical module(s)** — customer-facing team features (memberships/
  roles, sharing/permissions, invitations as the domain slices emerge). Owns
  its work items end-to-end in the central index.
- **Both carry `Packages:` from day one** (module metadata + per-item where it
  differs), and trimmed survivors get tagged as they're next touched — no
  big-bang retrofit.
- Index gets the tagged-tier view sections (`## What's Next`, `## Modules by
  Package`) — initially hand-written, replaced by generated output when
  upstream PKG-003 ships.

## Part 4 — Tooling onboarding

1. Upgrade to **aps 0.5.0** (global binary; `aps migrate` was built for
   exactly this vendored-CLI → binary move; `aps update` refreshes templates
   incl. the `Packages` column).
2. Adapt single-tree assumptions in local scripts only as needed
   (`drift-check.mjs`, `advance-released.mjs`, `aps-cleanup.sh` stay valid —
   tags don't change the tree shape; that was the other argument against
   federation).
3. Note: `advance-released.mjs`'s JSON-vs-prose mismatch (2026-07-13) is now
   upstream REL-005 (`aps release close`, markdown-first) — candidate to
   retire the local script when it ships.

## Upstream dependency

Upstream now tracks the missing tagged-tier tooling as the **package-views
(PKG) module** (Ready, high): PKG-001 `aps next --package/--by-package`,
PKG-002 `Packages:` typo lint, PKG-003 generated by-package rollup view.
Adoption here does not block on it — tags are useful for humans immediately —
but the payoff compounds when PKG lands.

## Open Questions

1. Bucket A/B dispositions — owner call per module.
2. TEAM module decomposition — one module or per-slice from the start?
3. Should the epoch cut land as one PR (index + archives) or per-bucket waves?
