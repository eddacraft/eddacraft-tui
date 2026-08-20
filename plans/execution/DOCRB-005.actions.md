# DOCRB-005 Component Truth Migration Action Plan

**Work item:** DOCRB-005
**Status:** In Progress
**Risk:** high — fourteen large central as-builts contain component invariants,
trust boundaries, source links, and historical context that must be dispositioned
without creating duplicate authority
**Base:** `f0f834b39bbdbc3ff9c8c198ec6098f3afc33389`
**Claim:** advisory `refs/claims/DOCRB-005`

## ReadyItem

- **Goal:** Co-locate the component truth assigned `move` or `merge` by the
  DOCRB corpus disposition, resolve its already-completed `retire` rows, and
  preserve central discovery without retaining duplicate authority.
- **Work item:** DOCRB-005
- **Status:** Ready
- **Expected behaviour:** Each material current invariant and source link from
  the fourteen component-misplaced central as-builts is either carried into its
  owning component document, retained in a source-proved cross-system authority,
  or explicitly recorded as historical/non-authoritative. Central compatibility
  paths link readers to the new authority and governing decisions; they are not
  content-free tombstones. The retained auth authority stays unchanged because
  its host correction is already present. No concern has two apparent live
  authorities.
- **Files:** the fourteen central move/merge as-builts; their eleven bounded
  component destinations; `docs/architecture/README.md`,
  `docs/guides/documentation-governance.md`,
  `docs/guides/architecture-diagrams.md`, the corpus disposition, four
  generated documentation indexes, this action plan, one evidence report, and
  the DOCRB-005 item record. Directly owed downstream freshness metadata may be
  added only when the repository gate proves the edge.
- **Validation commands:** replacement RED/GREEN assertions per slice; fresh
  source/link/metadata trace; `pnpm format:check`; `pnpm docs:index`;
  `pnpm docs:index:check`; `pnpm docs:check`;
  `pnpm docs:owed --since f0f834b39`; `pnpm aps:active-lint`;
  `pnpm aps:index:check`; `pnpm aps:drift --json`;
  `git diff --check`.
- **Dependencies:** DOCRB-003 and DOCRB-004 are Merged.
- **Risk:** high.
- **Design sources:** ADR-123,
  `plans/specs/2026-08-16-docs-rebaseline.md`, and
  `plans/specs/2026-08-17-docrb-corpus-disposition.md`.
- **Constraints / non-goals:** no substantive public DOCSYNC refresh; no
  docs-site or start-here retirement from PR #4050; no DOCRB-008/-009 work; no
  new checker subsystem; no sibling-module status change; no bulk rollout to
  unrelated README-only roots; no product or configuration change.
- **PR base:** exact integration receipt
  `f0f834b39bbdbc3ff9c8c198ec6098f3afc33389`, rebased normally before
  publication if overlapping main changes land.
- **Stack depends on:** none.
- **Decision:** ready.

## Source-truth reconciliation

At the exact base, all fourteen central move/merge candidates still exist and
total 11,787 lines. The retired monitor document is already archived and both
retired central Draw.io sources were removed by DOCRB-006. The retained
`auth-as-built.md` already names `docs-shell` and its private/public renderers
and contains no live `docs-site` wording, so the required host-name repair is a
verified no-op.

The corpus has two levels of placement evidence. For observability, capsules,
and adapters, their component rows classify them as README-only while later
central rows name an `ARCHITECTURE.md` destination or make it optional. The
narrow resolution is README-only: keep their current component classification,
move only maintainable current component truth into the owning README, and
record the choice in the evidence report. This avoids silently reclassifying
three documentation units or expanding the migration.

## Path and slice matrix

### 1. Existing pilot reconciliation

- `docs/architecture/kernel-as-built.md` →
  `crates/anvil-kernel/ARCHITECTURE.md`
- `docs/architecture/intercept-as-built.md` →
  `crates/anvil-intercept/ARCHITECTURE.md`
- `docs/architecture/api-as-built.md` →
  `apps/anvil-api/ARCHITECTURE.md`

Update the three local READMEs only where they still describe the pilot as
subordinate to a central authority. Preserve current local corrections from
DOCRB-004. Convert each central path to a compatibility record that identifies
the successor, retained cross-system authorities, governing decisions, and Git
history.

### 2. Checks

- `docs/architecture/checks-as-built.md` →
  `crates/anvil-checks/ARCHITECTURE.md`
- update `crates/anvil-checks/README.md`

Keep only a source-proved cross-system registry-to-finding/surface relationship
centrally; otherwise use the same compatibility-record pattern. Link the
central quality model rather than copying it.

### 3. CLI consolidation

- `docs/architecture/activation-as-built.md`
- `docs/architecture/mcp-shim-as-built.md`
- `docs/architecture/cli-tui-runner-as-built.md`
- destination: `crates/anvil-cli/ARCHITECTURE.md`
- discovery: `crates/anvil-cli/README.md`

The local document owns activation orchestration, MCP shim internals, fallback
selection, terminal lifecycle, and component failure behaviour. Active specs,
ADRs, runbooks, public instructions, and the TUI component remain linked
authorities.

### 4. TUI consolidation

- `docs/architecture/tui-as-built.md`
- `docs/architecture/widgets-as-built.md`
- `docs/architecture/tutorial-as-built.md`
- destination: `crates/anvil-tui/ARCHITECTURE.md`
- discovery: `crates/anvil-tui/README.md` and, only where needed to preserve
  shared-widget invariants, `crates/eddacraft-tui/README.md`

The anvil TUI document owns surface dispatch, anvil-specific widgets, tutorial
state/control flow, snapshots, and failure behaviour. The shared eddacraft
widget crate remains authoritative for its own theme, keyboard, widget, and
snapshot contracts. Dashboard/operator and public tutorial concerns remain in
their existing authorities.

### 5. Driver and protocol boundary

- `docs/architecture/driver-framework-as-built.md` →
  `packages/anvil-driver-client/ARCHITECTURE.md` plus
  `crates/anvil-intercept-proto/README.md`
- update `packages/anvil-driver-client/README.md`

Retain centrally only a genuinely cross-component protocol, version,
capability, and trust map proved by source review. Do not assign Rust
proto/rules/Win32 internals to the TypeScript client.

### 6. README-only component leaves

- `docs/architecture/observability-as-built.md` →
  `crates/anvil-observability/README.md`
- `docs/architecture/capsule-as-built.md` →
  `crates/anvil-capsule/README.md`
- `docs/architecture/adapter-packages-as-built.md` →
  `packages/adapters/README.md`, linking the existing
  `packages/aps/README.md` and `packages/kindling-integration/README.md`

The three central paths become compatibility records. Current component
contracts and invariants move; stale line counts, resolved-gap narratives,
sibling-module status summaries, public guidance, and decision rationale are
classified explicitly as historical or linked to their owning authority.

### 7. Authority, discovery, and evidence closeout

Update `docs/architecture/README.md`,
`docs/guides/documentation-governance.md`,
`docs/guides/architecture-diagrams.md`, the corpus disposition, the four
canonical documentation indexes, and
`plans/reviews/2026-08-20-docrb-005-component-truth-migration.md`.
Preserve old central filenames so inbound links remain useful. Do not rewrite
historical APS/review documents merely to retarget those links.

PR #4050 overlaps architecture discovery, governance, and one line in the
tutorial as-built. Rebase after it lands and resolve only DOCRB-005-owned text;
do not absorb its docs-site or public-content deletion.

## Replacement-evidence loop

For each slice:

1. **RED:** prove the central path still asserts live component authority and
   the destination either does not exist or lacks the specific current
   invariant/source link.
2. Compare every central section with current source, tests, schemas, ADRs,
   component docs, runbooks, and central cross-system views.
3. Move only current component truth; retain current cross-system truth in its
   central owner; classify obsolete rollout, resolved-gap, or snapshot prose as
   historical in the evidence matrix.
4. **GREEN:** prove every material section and load-bearing source link has one
   disposition, the local authority is discoverable, the central path points to
   the successor/history/retained authorities, and no duplicate live authority
   remains.
5. Run the narrowest component test only when a documentation claim cannot be
   established by direct source/test inspection.

## Final evidence and review

The evidence report records:

- exact base/head and changed paths;
- one row for every section and material source link in the fourteen inputs;
- replacement RED/GREEN commands and results by slice;
- current-source and repository-local-link trace;
- already-resolved retire rows and auth no-op evidence;
- the README-only conflict resolution;
- central remainder justification for checks and driver, if retained;
- generated index, docs-owed, formatting, documentation, APS, and diff results;
- exact-head independent verification and Council outcomes.

Independent verification and Council review receive the exact base/head,
governing sources, acceptance behaviour, and command evidence. Only binding,
in-scope findings are repaired. Review requests for public refresh, checker
automation, unrelated component rollout, or DOCRB-008/-009 implementation are
reported as expansion and not applied.

## Rollback

Revert the DOCRB-005 feature commits as one documentation-only unit. Central
compatibility paths preserve discovery throughout; there is no runtime,
configuration, public content, or mandatory CI behaviour to roll back.
