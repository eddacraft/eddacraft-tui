# APS CLI Adoption Boundary

| Type | Authority     | Owner  | Status | Freshness                                                                                                |
| ---- | ------------- | ------ | ------ | -------------------------------------------------------------------------------------------------------- |
| Spec | Authoritative | APSCAN | Live   | Created 2026-05-25 against `anvil-plan-spec` v0.3.0 and current `packages/aps` + `scripts/aps` consumers |

| Upstream                                                                          | Downstream                                                                                                              |
| --------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `plans/aps-rules.md`, `plans/project-context.md`, `anvil-plan-spec` v0.3.0, `packages/aps/**`, `scripts/aps/**` | `packages/aps/**` (kept), `scripts/aps/**` (kept), `crates/anvil-cli/src/plan_dashboard.rs`, agent dev-workflow routing |

## Purpose

Close APSCAN-008 by recording the authority split between the canonical
`anvil-plan-spec` CLI and Anvil's local `@eddacraft/anvil-aps` package +
`scripts/aps/*` tooling. The split decides where future APS work lands and what
agents reach for at each stage of the dev-workflow loop.

## Context

Anvil is migrating its plan dialect toward the canonical `anvil-plan-spec`
([`anvil-plan-spec` v0.3.0](https://github.com/eddacraft/anvil-plan-spec)).
Three CLI / library surfaces exist today:

1. **Canonical `aps` CLI** (`anvil-plan-spec/cli`) — portable commands such as
   `aps lint`, `aps next`, `aps start`, `aps complete`, `aps graph`. Speaks the
   canonical schema and the canonical status vocabulary
   (`Proposed`/`Ready`/`In Progress`/`Done`/`Blocked`).
2. **Local `@eddacraft/anvil-aps` package** (`packages/aps/**`) — parser, loader,
   validator, state, templates, and types used by `anvil plan dashboard`, the
   TUI, drift checks, and other in-tree consumers. Already accepts canonical
   `## Work Items` plus the legacy `## Tasks` form (APSCAN-003) and the
   `Outcome:` alias.
3. **Anvil-specific scripts** (`scripts/aps/**`) — `drift-check.mjs`,
   `active-lint.mjs`, and helpers that enforce Anvil-only checks: progress
   counter consistency, release-record evidence, Anvil status extensions
   (`Merged`, `Released/Shipped`, `Complete`), index-vs-module reconciliation.

The question APSCAN-008 asked: **do we replace surfaces (2) and (3) with the
canonical CLI, run them side by side, or keep them as a local compatibility
layer above canonical?**

## Decision

**Hybrid: keep `@eddacraft/anvil-aps` and `scripts/aps/*` as the local authority
layer; adopt the canonical `aps` CLI for portable read-only operations and any
future cross-repo planning workflows.**

Concretely:

| Concern                                          | Canonical `aps` CLI | Local `@eddacraft/anvil-aps` + `scripts/aps/*` |
| ------------------------------------------------ | :-----------------: | :--------------------------------------------: |
| Parse canonical `## Work Items` / `Outcome:`     |          ✓          |              ✓ (alias-aware)                   |
| Parse legacy `## Tasks` / `Expected Outcome:`    |          ✗          |              ✓ (APSCAN-003)                    |
| Validate against canonical schema                |          ✓          |              ✓                                 |
| Enforce active-lint scope (exclude archive)      |          ✗          |              ✓ (APSCAN-001)                    |
| Anvil status extensions (Merged / Released)      |          ✗          |              ✓ (APSCAN-006)                    |
| Progress counter / index reconciliation          |          ✗          |              ✓ (`aps:drift`)                   |
| Release-record evidence checks                   |          ✗          |              ✓ (`aps-released-shipped-...`)    |
| `anvil plan dashboard` snapshot                  |          —          |              ✓ (APSCAN-011)                    |
| Cross-repo portable planning (`aps next`)        |          ✓          |              —                                 |
| Reference templates (`aps init`, scaffolds)      |          ✓          |              ✓ (`packages/aps/templates/`)     |

## Authority split

1. **Canonical `aps` CLI is the source of truth for portable APS semantics.**
   Anvil's parser and validator stay close to canonical shape; whenever
   `anvil-plan-spec` releases a clarification, `@eddacraft/anvil-aps` mirrors
   the canonical behaviour (APSCAN-003 set the precedent for this).
2. **`@eddacraft/anvil-aps` is the local compatibility + extension layer.**
   It accepts canonical syntax, retains legacy aliases during APSCAN migration,
   and exposes the typed API that in-tree TypeScript + Rust consumers (TUI,
   dashboard, drift checks) call directly. Removing this package would force
   every in-tree consumer to shell out to a CLI on every read.
3. **`scripts/aps/*` is the Anvil-only enforcement layer.** Drift, release
   evidence, progress counter consistency, active-lint scope, and the Anvil
   status extensions documented in `plans/project-context.md` live here. None of
   these checks belong in canonical `aps` — they encode Anvil's release
   lifecycle, not portable APS semantics.
4. **Agents pick by stage.** Planning + reading (start of session, dashboard,
   `aps next`-style "what's ready") may use either surface. Validation +
   enforcement before push (drift, release, counter) MUST use the local layer
   because canonical `aps` has no knowledge of Anvil's release extensions.

## Why hybrid (and not "adopt canonical CLI fully")

- **Anvil's drift authority is project-specific.** `scripts/aps/drift-check.mjs`
  is ~478 lines and encodes release-record evidence requirements, Anvil status
  extensions, and index/module counter consistency. None of that ships in
  `aps`; replacing it with canonical lint would silently weaken the gate.
- **In-tree consumers need typed access.** `anvil plan dashboard`
  (`crates/anvil-cli/src/plan_dashboard.rs`) and the TUI surface use the
  TypeScript types out of `@eddacraft/anvil-aps`; replacing those with CLI
  spawns adds latency, JSON round-trips, and removes static typing.
- **Migration period needs alias tolerance.** Until APSCAN-004 finishes the
  active-module heading migration, the parser MUST accept both legacy and
  canonical forms. The canonical CLI accepts canonical only.
- **Distribution risk.** `aps` is a separate npm/binary surface; pinning a
  specific upstream version inside Anvil's CI adds a dependency lifecycle that
  buys us nothing today.

## Why hybrid (and not "drop canonical CLI entirely")

- **Cross-repo planning workflows benefit from portable tooling.** Operators
  managing multiple Anvil-adjacent projects (kindling, anvil-plan-spec itself,
  Eddacraft skills) can use `aps next` / `aps start` / `aps complete` as a
  uniform interface; Anvil should not block that by silently diverging.
- **Conformance pressure is valuable.** Keeping the canonical CLI runnable
  against Anvil's plans is a forcing function for staying close to canonical
  shape; if `aps lint --strict` ever breaks on active Anvil plans, that is a
  drift signal worth catching.
- **Scaffold parity.** `aps init` scaffolds the canonical templates; Anvil's
  `packages/aps/templates/` should track those and any divergence is a bug.

## Implementation guidance

### What we are NOT changing in this spec

- No code changes in `packages/aps/**` or `scripts/aps/**` land with this
  decision; the existing surfaces already match the authority split.
- No new CI gate is added; canonical-CLI conformance can stay an opt-in local
  invocation until a future APSCAN item motivates a gate.

### What follows from the decision

1. **APSCAN-004 / APSCAN-005 (heading + filename migration)** continue, because
   migrating to canonical shape lets the canonical CLI lint Anvil plans without
   alias hacks.
2. **APSCAN-009 (counter reconciliation)** stays with the local drift check;
   canonical `aps` does not own progress counters.
3. **APSCAN-010 (active-module migration wave)** uses the local validator as
   the primary gate, with an optional `aps lint` invocation to confirm
   canonical conformance on the migrated batch.
4. **Future canonical CLI bumps** should be tracked as APSCAN follow-ups when
   they introduce schema or behaviour drift (analogous to APSCAN-003 for
   `## Work Items`).

### Agent guidance

- For **reading and dashboards**, prefer `@eddacraft/anvil-aps` (typed API,
  cached, no spawn cost).
- For **pre-push validation**, run the local `pnpm aps:drift`,
  `pnpm aps:active-lint`, and `pnpm docs:check` commands as authority.
- For **cross-repo / portable interop**, `aps lint` and `aps next` against the
  local `plans/` tree are safe read-only operations.
- For **release evidence**, the canonical CLI has no opinion; defer to
  `scripts/aps/drift-check.mjs` and the release-record schema in
  `plans/project-context.md`.

## Open questions

None. The decision is recorded; the implementation is the existing local layer.
Future canonical-CLI version bumps that change semantics will reopen the
question via a new APSCAN item.

## Amendment — 2026-05-29 (CIB-036: corpus brought to canonical conformance)

The original decision reserved canonical `aps lint` for an already-migrated
subset. CIB-036 resolved the remaining divergence in favour of **full corpus
conformance to canonical `## Work Items`** (option a), so canonical `aps lint`
is now authoritative over the whole active module corpus, not just a subset:

- The active corpus was migrated to canonical structure — `## Work Items`
  section, `| ID | … | Status |` metadata table, per-item fields — via PR #2095
  (mechanical: `## Tasks`→`## Work Items`, casing, phase-nesting, `Scope`→`ID`)
  plus a cleanup pass (metadata tables, field fills, `issues.md` headings).
  Phase-grouped modules keep their phases nested under `## Work Items`.
- **Where canonical `aps` was genuinely unfit, the spec was changed rather than
  the modules contorted** (consistent with Anvil being the canonical `aps`
  consumer): anvil-plan-spec PR #56 exempts terminal work items
  (Done/Complete/Merged/Released/Shipped) from the E005 required-field check,
  matching Anvil's done-item compaction convention.
- **Non-module document types stay out of canonical scope.**
  `clawpatch-pre-tag-v0.7.0-beta.aps.md` is a release-findings tracker (bug
  findings with Severity/Resolution, not Intent/Outcome/Validation work items);
  it is listed in `NON_CANONICAL_MODULES` in `scripts/aps/active-lint.mjs` and
  excluded, pending archival once its findings close out.
- `pnpm aps:active-lint` now passes across the active corpus and is wired as a
  CI gate, so structural drift cannot silently return (it previously could —
  see CIB-037, which fixed `aps lint` only validating one file).

## Related

- [APSCAN-001](../archive/modules/aps-canonical-alignment.aps.md): defined the active
  APS lint scope (`scripts/aps/active-lint.mjs`).
- [APSCAN-002](../archive/modules/aps-canonical-alignment.aps.md): split portable APS
  rules from Anvil project context.
- [APSCAN-003](../archive/modules/aps-canonical-alignment.aps.md): added canonical
  parser/validator aliases for `## Work Items` and `Outcome:`.
- [APSCAN-006](../archive/modules/aps-canonical-alignment.aps.md): documented canonical
  status vocabulary vs. Anvil project status extensions.
- [`plans/aps-rules.md`](../aps-rules.md): canonical APS rules surface.
- [`plans/project-context.md`](../project-context.md): Anvil-specific project
  context including release lifecycle extensions.
