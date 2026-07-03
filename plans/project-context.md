# Anvil APS Project Context

| Type  | Authority     | Owner  | Status | Freshness                                                                            |
| ----- | ------------- | ------ | ------ | ------------------------------------------------------------------------------------ |
| Guide | Authoritative | APSCAN | Live   | Last reviewed 2026-05-25 against `plans/archive/modules/aps-canonical-alignment.aps.md` (APSCAN archived) |

| Upstream                                                  | Downstream                                      |
| --------------------------------------------------------- | ----------------------------------------------- |
| `plans/aps-rules.md`, `AGENTS.md`, `docs/guides/documentation-governance.md` | `AGENTS.md`, `plans/aps-rules.md`, agents using APS |

This file records Anvil-specific planning, execution, release, and documentation
rules that deliberately sit outside portable APS guidance. Keep
[`plans/aps-rules.md`](aps-rules.md) close to the canonical APS scaffold and put
local operating-model context here.

## Authority

- `plans/aps-rules.md` is the APS-managed rule surface: portable vocabulary,
  document shape, work-item rules, action-plan guidance, and canonical file
  layout.
- `plans/project-context.md` is anvil-owned context: Worktrunk branching,
  Council review, release lifecycle prose, feature flags, documentation
  governance, and repository-specific validation.
- `AGENTS.md` remains the top-level agent contract and links to both files.
- Source code, schemas, tests, and generated artefacts remain implementation
  truth.

## anvil Lifecycle Narrative

anvil uses lifecycle prose in index commentary, release records, and closeout
notes. These labels are useful operational context, but they are not portable
APS schema values:

```text
APS Draft -> APS Proposed -> APS Ready -> In Progress -> Merged -> Released/Shipped -> Complete/Archived
```

- `Merged` means code or docs reached the integration target but have not
  necessarily shipped.
- `Released` / `Shipped` means a release record proves inclusion in a verified
  release.
- `Complete` in prose means no remaining active closeout work; a module may be
  archived. In schema fields, use canonical `Done`.
- `Archived` means the module moved to `plans/archive/modules/` and is
  historical.
- `Committed` is legacy wording for `Merged`; new text should prefer `Merged`.

### Project Status Extensions

The canonical APS work item status vocabulary
([`plans/aps-rules.md#status-vocabulary`](aps-rules.md#status-vocabulary)) is the
portable contract. anvil locally extends that vocabulary with the lifecycle
labels above so a work item's `Status:` field can carry release evidence inline
rather than tracking it in a separate field. The accepted extensions are:

| Status | Origin | Maps to canonical | When to use |
| ------ | ------ | ----------------- | ----------- |
| `Proposed`, `Ready`, `In Progress`, `Done`, `Blocked` | Canonical APS | self | Default; any portable APS reader understands these. |
| `Merged` | anvil extension | `Done` | Integration target reached (PR merged), release inclusion not yet proven. |
| `Released/Shipped` | anvil extension | `Done` | Release record proves inclusion in a verified release. |
| `Complete` | Legacy alias normalised to canonical | `Done` | Historical text; new modules should write `Done`, `Merged`, or `Released/Shipped`. |
| `Archived` | anvil extension (module-level prose) | `Done` | Module moved to `plans/archive/modules/`; not used in work-item `Status:` fields. |

Rules for using extensions:

1. **New work items** SHOULD write a canonical status (`Ready`, `In Progress`,
   `Done`, `Blocked`) when execution state is the only fact being recorded.
2. **Closeout text** SHOULD prefer `Merged YYYY-MM-DD via PR #N` once integration
   evidence exists, and `Released/Shipped via vX.Y.Z` once release evidence
   exists, so progress counters and downstream tools have a single field to
   read. The drift-check prefix-matches the leading `Merged` /
   `Released/Shipped` token, so any trailing prose (date, PR ref, version) is
   accepted for both progress accounting and release-record alignment checks.
3. **Status casing matters.** Write `Done`, `Merged`, `Released/Shipped`,
   `Complete` exactly as shown. Matching in
   `scripts/aps/drift-check.mjs` is case-sensitive; lowercase variants parse
   but are silently ignored by the status-keyed drift checks (progress
   accounting, candidate-record alignment, published-record alignment, and the
   literal `Complete` validation-evidence check).
4. **Portable readers** SHOULD treat any non-canonical value as opaque or map it
   to `Done` for progress accounting. `scripts/aps/drift-check.mjs` does the
   latter via the `DONE_PATTERNS` set.
5. **Release metadata fields** (see below) are separate from the status field;
   they describe release intent, not execution state.

## Repository Workflow

anvil work follows this lifecycle:

```text
APS (Ready) -> Worktrunk Branch -> Code -> Council -> PR -> Merged -> cleanup offer -> Released/Shipped -> Complete
```

Key rules:

1. Read `plans/index.aps.md` before starting implementation.
2. Work in a Worktrunk-managed worktree from `main`.
3. Branch names reference the APS module, such as
   `docs/apscan-002-context-split`.
4. Follow TDD for code changes; docs-only changes still need docs validation.
5. Run Council before opening non-trivial PRs.
6. After PR creation, run the PR-review remediation loop for CI and review
   comments.
7. Offer local worktree cleanup only when local state is clean and review fixes
   are not expected.

Authoritative details live in `AGENTS.md`,
[`docs/guides/branching-strategy.md`](../docs/guides/branching-strategy.md), and
[`docs/guides/worktree-policy.md`](../docs/guides/worktree-policy.md).

## Single Source of Truth

`plans/index.aps.md` is the canonical index of active modules. Do not create
separate module lists, status summaries, or shadow indexes.

Active modules live under `plans/modules/`. Completed modules move to
`plans/archive/modules/` with `git mv`; update the index path in the same
change.

## Progress Counters

Active modules carry an at-a-glance progress counter (`Done/Total`) in two
places:

1. The module header table (`| MODULE | — | Status | X/Y |`).
2. The module's row in `plans/index.aps.md`.

Per-item `Status:` lines are authoritative. The stored `N/M` is an
advisory-derived convenience refreshed by `scripts/aps/index-counts.mjs`
(ADR-053). Feature PRs flip only their own item's `Status:` line; they do
**not** edit the aggregate `N/M` cells.

`scripts/aps/index-counts.mjs --check` (CI via `pnpm aps:index:check`) derives
counts from work-item statuses, reports freshness drift, and exits 0 so
concurrent same-module PRs do not collide on the count token. Write mode
(`pnpm aps:index`) is the single-writer reconcile.

`scripts/aps/drift-check.mjs` still reconciles the stored count against the
status-derived count using `DONE_PATTERNS` (canonical `Done`/`Complete` plus
the anvil extensions `Merged`/`Released/Shipped` — see
[`#project-status-extensions`](#project-status-extensions)) and emits
`aps-progress-mismatch` (module header) or `aps-index-progress-mismatch`
(index row) as advisory warnings when the numbers diverge.

Rules:

1. **Feature PRs never bump `N/M`.** Complete an item by updating its `Status:`
   line only. Refresh stored counts in a separate reconcile commit via
   `pnpm aps:index` when the at-a-glance rollup should be current.
2. **New modules MAY omit the counter** when status-derived progress is
   enough (the drift-check only fires when both `progressDone` and
   `progressTotal` are non-null in the header). Omitting opts out of both
   `aps-progress-mismatch` and `aps-index-progress-mismatch`.
3. **Status casing matches the canonical/extension vocabulary**
   (`Done`/`Merged`/`Released/Shipped`/`Complete` exactly) — `DONE_PATTERNS`
   in `scripts/aps/drift-check.mjs` is case-sensitive, so lowercase variants
   are not counted as done. Counter numbers themselves (`X/Y`) have no casing
   concerns.
4. **Released/Shipped without release-record evidence** triggers
   `shipped-aps-without-release-record` (advisory). It is a real signal: the
   work was tagged Released/Shipped but the release-record file does not list
   the item. Resolve by either updating the release record or downgrading the
   status to `Merged` (if the item is not actually in a tagged release).

## Execution Plans (`plans/execution/`)

Action plans live under `plans/execution/`. anvil follows the canonical
`.actions.md` naming convention going forward; legacy `.steps.md` plans remain
readable for historical context but are excluded from canonical active APS lint
(`scripts/aps/active-lint.mjs`).

Rules:

1. **New execution plans MUST use `.actions.md`**
   (`plans/execution/<WORK-ITEM-ID>.actions.md` or
   `plans/execution/<MODULE>.actions.md`).
2. **Rename legacy `.steps.md` to `.actions.md` when the surrounding work is
   touched again** (a new wave is started, the plan is re-opened, or a follow-up
   work item refers to it). Use `git mv` so blame history is preserved, and
   update any inline references in the owning module file in the same commit.
3. **Do not bulk-rename historical `.steps.md` plans.** Plans that have not
   been touched since their original work landed are historical; they stay
   `.steps.md` until APSCAN or a future module explicitly reopens them.
4. **Archived execution plans stay where they are.** When a module moves to
   `plans/archive/modules/`, the related execution plan files stay in
   `plans/execution/` with their existing suffix; they document history.

## Keeping Plans Current

Agents update APS state as they work:

1. Before starting substantive implementation, mark the module or work item
   `In Progress` where applicable.
2. After completing a work item, update its status and closeout evidence in the
   module file — do not bump stored `N/M` counts in feature PRs (ADR-053).
3. Reconcile stored progress counts with `pnpm aps:index` when a refresh is
   needed (typically a dedicated bookkeeping commit after a wave).
4. Archive completed modules when all active work is done and release/closeout
   evidence is complete.

## Release Metadata Extensions

anvil work items may carry release metadata as project-specific prose fields:

```yaml
changeType: fix | feature | docs | internal | breaking
releaseIntent: candidate | hold | never
holdCondition: required when releaseIntent is hold
releaseScope: patch | minor | major | none
releaseNote:
  audience: user | operator | developer | none
  type: added | fixed | changed | removed | security
  text: optional one-sentence release note
validation:
  - command to prove the item
```

These fields are not portable APS schema fields. They are human and tooling
conventions used by anvil release workflows.

Rules:

1. `changeType` describes the change shape, not the git commit type.
2. `releaseIntent: candidate` means the item is eligible for a release candidate
   once merged.
3. `releaseIntent: hold` means merged work should not ship until
   `holdCondition` is satisfied.
4. `releaseIntent: never` is for docs/internal work that should not drive a
   product release by itself.
5. `releaseScope` is `none` for non-releasable work.
6. `releaseNote.audience: none` means no user/operator/developer-facing note is
   expected.
7. CI remains the validation authority for release readiness.

## Cross-Cutting Modules

Cross-cutting modules coordinate work that touches multiple domains without
owning a single product surface. Such modules must:

1. Own their own work items and progress counts.
2. Cross-reference related modules with prose callouts such as `Coordinates
   with:`, `Blocks on:`, `Supersedes:`, and `Superseded by:`.
3. Sweep and close callouts when completing work or archiving modules.
4. Avoid separate dependency graphs unless a future APS item explicitly adds one.

When changing this convention, update active cross-cutting modules that cite it.

## Feature Flag Governance

When introducing or modifying feature flags, follow
[`docs/guides/feature-flag-governance.md`](../docs/guides/feature-flag-governance.md).

Key APS-facing rules:

1. Every flag needs `createdFor` linking to an APS work item.
2. Rollout flags must have a sunset or review date.
3. Retirement follows `active` -> `retiring` -> `retired` -> delete.
4. Kill switch and entitlement flags fail closed on error.

## Documentation Governance

Documentation changes are operational changes. When changing `docs/**`,
`plans/**`, `README.md`, `CONTRIBUTING.md`, `AGENTS.md`, or package/crate
READMEs, complete the closeout workflow in
[`docs/guides/documentation-governance.md`](../docs/guides/documentation-governance.md).

Minimum closeout:

1. Classify changed documents by type and authority.
2. Check whether APS, ADRs, as-built docs, runbooks, guides, public docs, or
   READMEs need cross-link updates.
3. Update required indexes, especially `plans/index.aps.md`,
   `plans/decisions/DECISION-LOG.md`, and local README indexes.
4. Mark stale or superseded information inline, or track unresolved drift in
   APS.
5. Run relevant validation, or state why it was not run.
6. Include a `Docs Closeout` note in the final response.

## Local Validation Expectations

Default full validation before committing code changes:

```bash
pnpm format:check && pnpm lint:check && pnpm typecheck && pnpm test
```

Docs-only APS changes usually need at least:

```bash
pnpm docs:check
pnpm aps:drift --json
```

Rust changes usually need targeted `cargo test` and `cargo clippy` commands in
addition to formatting checks. Exact commands should come from the relevant APS
work item.

## Test Infrastructure Summary

anvil tests are split across three stacks:

| Stack | Location | Runner |
| ----- | -------- | ------ |
| TypeScript unit | `packages/**/__tests__`, co-located tests | Vitest via Nx |
| Rust unit | `crates/**/src/**/tests` | `cargo test` |
| E2E | `apps/e2e/src/**/*.e2e.test.ts` | Vitest workspace |
| Rego | `policies/fixtures/*.rego` | `opa test`, Regal |

CI is authoritative. Local validation gives evidence before commit/PR.

## Repository Operations

Use `gx` for repository management. Never use raw `git clone` for repository
setup.

| Task | Command |
| ---- | ------- |
| Clone a repo | `gx clone <url-or-name>` |
| Jump to a project | `gx <name>` |
| Scaffold configs | `gx init` |
| List projects | `gx list` |

## Commit Format

Use conventional commits with imperative mood, lowercase subject, and no final
period:

```text
<type>(<scope>): <subject>
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `chore`,
`ci`.

The `Authored-By:` trailer is added automatically; do not add it manually.

## Local Deviations to Preserve

- Numeric module filename prefixes are allowed when dependency order benefits
  from them, even though current active modules mostly use stable kebab slugs.
- Active migration work may temporarily accept legacy field aliases while
  `APSCAN` moves the repository toward canonical APS terms.
- Historical archive content remains historical unless a future item explicitly
  reopens it.
- APS governs repository work,not user prerequisites.
