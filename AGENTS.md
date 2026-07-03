# Agent Guidelines

These conventions apply to all agents working in this project.

> For repo orientation — vocabulary, where things live, and where to go next —
> see [`CONTEXT.md`](CONTEXT.md). This file is the _how to behave_ source;
> `CONTEXT.md` is the _where to find things_ map.

## Planning — Anvil Plan Spec (APS)

All multi-step work MUST use APS format. Read `plans/aps-rules.md` for portable
APS rules and `plans/project-context.md` for anvil-specific workflow, release,
and documentation-governance context before writing or modifying any `.aps.md`
file.

### Single source of truth

`plans/index.aps.md` is the canonical index of all modules. Do NOT create
separate module lists, summary files, or shadow indexes — they drift and cause
confusion.

### Key rules

- **Read before writing:** check `plans/index.aps.md` for active work and
  current status before starting any implementation
- **Modules:** `plans/modules/<module>.aps.md` (active),
  `plans/archive/modules/` (completed)
- **Work item IDs:** `PREFIX-NNN` (3-digit zero-padded)
- **Module schema statuses:** Proposed → Ready → In Progress → Done → Blocked
- **Work item lifecycle extensions:** anvil also allows `Merged` and
  `Released/Shipped` in work-item `Status:` fields; see
  `plans/project-context.md#project-status-extensions`
- **Wave-based** parallel execution for independent work items
- **UK English** spelling in all plan text

### Keeping plans current

Agents MUST update APS state as they work — do not leave this for later:

1. **Before starting:** mark module status **In Progress** in the module file
2. **After completing a work item:** update its status (checkbox, Status field,
   or table row) in the module file — do **not** bump the module header or index
   `N/M` count in feature PRs (ADR-053; counts are advisory-derived)
3. **After all active items done:** update schema status to **Done**; use
   **Complete** only as lifecycle prose when closeout/archive evidence supports
   it
4. **Reconcile stored counts when needed:** run `pnpm aps:index` in a dedicated
   bookkeeping commit when the at-a-glance `N/M` should be refreshed
5. **Archive completed modules:** `git mv` to `plans/archive/modules/` and
   update the path in `index.aps.md`

Reference spec: <https://github.com/eddacraft/anvil-plan-spec> Project context:
[`plans/project-context.md`](plans/project-context.md)

### No inline TODOs

Do NOT leave `TODO`, `FIXME`, `XXX`, or `HACK` markers in code, docs, plans, or
config. Inline markers are invisible to the APS index, drift out of sync, and
never get scheduled.

Every piece of deferred or follow-up work MUST be tracked as one of:

- an **APS work item** in `plans/modules/<module>.aps.md` — for planned,
  multi-step, or roadmap-bound work, or
- a **GitHub Issue** — for standalone bugs, smaller follow-ups, or work without
  an owning module yet.

If you are tempted to write a `TODO`, instead add the APS item or open the issue
and reference its ID in the commit or surrounding doc. Pre-existing markers you
encounter should be converted to an APS item or GitHub Issue, not propagated.

## Architecture and Design Decisions

Before proposing new architecture, changing technology choices, or planning work
that touches system boundaries, check existing decisions first:

1. **Decision log:** `plans/decisions/DECISION-LOG.md` — condensed index of all
   ADRs with one-line summaries. Start here.
2. **Scope guard:** `docs/vision/anvil-scope-guard.md` — defines what anvil is
   and isn't. Check before proposing new scope.
3. **Architecture overview:** `docs/architecture/overview.md` — design
   philosophy (zero-config posture, deterministic, composable, safety by
   default).
4. **Full ADRs:** `plans/decisions/NNN-*.md` — read the specific ADR when you
   need trade-off context beyond the one-line summary.

When introducing a new architectural decision, follow
`docs/guides/adr-process.md` and add the entry to the decision log.

## Documentation Governance

Documentation is executable context for humans and agents. Treat documentation
changes as operational changes, not prose cleanup.

### Authority model

The canonical documentation authority model lives in
`docs/guides/documentation-governance.md`. Agent-facing summary:

- APS files authorise and track work.
- ADRs explain durable architectural decisions and trade-offs.
- Source code, schemas, tests, and generated artefacts are implementation truth.
- As-built docs map what is currently shipping and must cite source references.
- Runbooks define operational procedures and must stay executable.
- Guides explain development practice and must not duplicate source truth.
- Public docs describe user-facing behaviour and must match release state.
- Archived docs are historical unless an active doc explicitly cites them.

### Mandatory docs closeout

When changing `docs/**`, `plans/**`, `README.md`, `CONTRIBUTING.md`,
`AGENTS.md`, or package/crate READMEs, agents MUST complete documentation
closeout before the final response:

1. Classify each changed document by type and authority.
2. Check whether APS, ADRs, as-built docs, runbooks, guides, public docs, or
   READMEs need cross-link updates.
3. Update required indexes, especially `plans/index.aps.md`,
   `plans/decisions/DECISION-LOG.md`, and local README indexes.
4. Mark stale or superseded information inline, or track unresolved drift in
   APS.
5. Run the relevant validation command, or state why it was not run.
6. Include a short `Docs Closeout` note in the final response.

Use `docs/guides/documentation-governance.md` for the documentation workflow,
authority routing, and closeout checklist.

For docs/APS changes, prefer the narrow relevant checks before the full repo
gates: `pnpm run docs:check`, `pnpm run aps:index:check`,
`pnpm run aps:active-lint`, `pnpm run test:docs-check`, `pnpm run lint:md`, and
`pnpm run format:check`.

## Repository Operations — gx

Use `gx` for all repository management. Never use raw `git clone`.

| Task              | Command                  |
| ----------------- | ------------------------ |
| Clone a repo      | `gx clone <url-or-name>` |
| Jump to a project | `gx <name>`              |
| Scaffold configs  | `gx init`                |
| List projects     | `gx list`                |

All cloned repos land in `~/Projects/src/` automatically.

## Commit Format

Conventional commits with imperative mood, lowercase, no period:

```
<type>(<scope>): <subject>
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `chore`, `ci`

The `Authored-By:` trailer is added automatically — do not add it manually.

## Development Lifecycle

Every piece of work follows this sequence. Agents must not skip stages.

```
APS (Ready) → Worktrunk Branch → Code → Council → PR → Merged → cleanup offer → Released/Shipped → Complete
```

### 1. Start from APS

- Read `plans/index.aps.md` — pick the next **Ready** work item
- Mark module **In Progress** before touching any code
- Branch name must reference the APS module: `fix/<module-slug>` or
  `feat/<module-slug>`
- Create the task branch and worktree from `main` with Worktrunk
  (`wt switch --create <branch>`). Hotfixes also branch from `main` (or the
  latest good tag if `main` is unreleasable). `dev` was retired by OPMODEL-012
  on 2026-05-11 — see `docs/guides/branching-strategy.md`.

### 2. Code

- Work in a Worktrunk-managed worktree — see `docs/guides/worktree-policy.md`
- Follow TDD: tests before implementation
- Run `pnpm format:check && pnpm lint:check && pnpm typecheck && pnpm test`
  before committing
- `.husky/pre-commit` runs `lint-staged` and re-checks staged `oxfmt`-managed
  files, but it does not replace the full repo checks above
- Commit with conventional format referencing APS ID where applicable

### 3. Council Review (before PR)

- Run `/council [quick|mini|full] <target>` before opening any non-trivial PR.
  Default to `quick`; escalate to `mini` for cross-boundary / CI / release /
  security / workflow risk, and to `full` for branch / release-operating-model
  changes or high-risk design — see `.claude/commands/council.md` for the tier
  table and Role Map.
- Address CRITICAL and MAJOR findings before opening PR.

### 4. Open PR

- Target `main`. Hotfixes also target `main` (or a release branch if one is
  active).
- After opening the PR, wait up to 10 minutes for Copilot and other automated
  reviewers to complete or time out, then run `addressing-pr-reviews`.
- Do not mention or tag Copilot or other bots to request review unless the user
  explicitly asks.
- Review remediation order is: failing CI first, automated review comments
  second, human review comments third. Re-run targeted validation after each
  meaningful fix batch.
- If the PR has post-merge verification steps:
  - Extract them to `plans/reviews/post-merge/<branch-slug>.md`
    (`!plans/reviews/post-merge/` is a gitignore exception, so the file is
    tracked)
  - Reference the APS module ID in the file
  - Do NOT leave test plans only in the PR description
- Mark the affected work item(s) **Merged** in the `.aps.md` file when the PR
  lands. Keep module schema status canonical; use **Done** only when all active
  work items are terminal and closeout evidence supports it.

### 5. Local cleanup offer

- At the end of a task, offer `wt remove` to remove the worktree and delete the
  local branch when safe.
- Do not offer cleanup while PR review fixes are still expected unless the user
  says local iteration is done.
- Only clean automatically safe state: clean worktree, branch pushed or merged,
  and no uncommitted work.
- Ask before deleting unmerged, unpushed, or still-needed branches. Never delete
  remote branches unless the user explicitly asks or the PR workflow already
  did.

### 6. Cleanup Agent (automated)

- Runs on schedule — checks all modules with **Merged** work items
- Verifies branch merged + CI green → advances work items to
  **Released/Shipped** when a release record proves inclusion, then closes out
  the module when no active work remains
- Works through `plans/reviews/post-merge/` — verifies agent-runnable steps,
  flags human-required steps
- Sends notification for anything needing attention
- Logs to `plans/reviews/cleanup-log.md`

### Quick Reference

| Stage          | Agent/Command           | Skill                               |
| -------------- | ----------------------- | ----------------------------------- |
| Plan           | `anvil-plan-spec` agent | `writing-plans`, `planning-council` |
| Branch         | —                       | `using-git-worktrees`               |
| Code           | `tdd-coach` agent       | `test-driven-development`           |
| Debug          | `debugger` agent        | `systematic-debugging`              |
| Review         | `/council` command      | `council`                           |
| Finish         | `/commit` command       | `finishing-a-branch`                |
| Address review | —                       | `addressing-pr-reviews`             |
| Verify         | cleanup agent (cron)    | `verification-before-completion`    |

Reference: `plans/aps-rules.md` · `plans/project-context.md` ·
`docs/guides/branching-strategy.md` · `docs/guides/worktree-policy.md` ·
[`docs/guides/agent-surface-inventory.md`](docs/guides/agent-surface-inventory.md)
— authoritative list of which skills and agents are repo-local vs global.

> Routing skill: `dev-workflow` is vendored repo-local at
> `.claude/skills/dev-workflow/SKILL.md` and
> `.opencode/skills/dev-workflow/SKILL.md`, tuned to anvil's main-first cutover
> and `quick|mini|full` Council tiers.

## Feature Flags

When introducing or modifying feature flags, follow the governance rules in
`docs/guides/feature-flag-governance.md` and `plans/project-context.md`. Key
points:

- Every flag needs `createdFor` linking to an APS work item
- `rollout` flags must have a sunset date (`expiryOrReviewDate`)
- Retirement follows: `active` → `retiring` → `retired` → delete
- Kill switch and entitlement flags fail closed on error

## Test Infrastructure

Tests are split across three stacks. All run in CI via
`.github/workflows/ci.yml` (TS) and `.github/workflows/rust.yml` (Rust).

### Where tests live

| Stack       | Location                            | Runner              | CI job                   |
| ----------- | ----------------------------------- | ------------------- | ------------------------ |
| Unit (TS)   | `packages/**/__tests__`, co-located | vitest (via nx)     | `ci.yml` → `test`        |
| Unit (Rust) | `crates/**/src/**/tests`            | `cargo test`        | `rust.yml` → `test`      |
| E2E         | `apps/e2e/src/**/*.e2e.test.ts`     | vitest (workspace)  | `ci.yml` → `e2e-harness` |
| Rego        | `policies/fixtures/*.rego`          | `opa test`, `regal` | `rust.yml` → `test`      |

### Running locally

```bash
# TS unit tests (all packages, via nx)
pnpm test

# TS unit tests (one package)
pnpm exec nx run @eddacraft/anvil-core:test

# TS E2E harness (smoke + all surfaces)
pnpm --filter @eddacraft/anvil-e2e test
pnpm --filter @eddacraft/anvil-e2e test:smoke     # surfaces only
pnpm --filter @eddacraft/anvil-e2e test:cli       # skipped unless Rust CLI is built

# Rust unit tests + Rego
cargo test --workspace
opa test --verbose policies/fixtures/

# Coverage (local)
pnpm test -- --run --coverage --coverage.reporter=html
cargo llvm-cov --workspace --html            # needs `cargo install cargo-llvm-cov`
```

### Coverage reports

Coverage is advisory only — no blocking threshold. PR and `main`-push runs do
not emit coverage; per APS CICD-006, coverage moved to the nightly
`ci-nightly.yml` workflow:

- **TypeScript**: the `coverage-typescript` nightly job emits a per-project
  line/branch/function/statement table in the job summary. Raw
  `coverage/coverage-summary.json` files are uploaded as the
  `coverage-report-22.x` artifact (14-day retention).
- **Rust**: the `coverage-rust` nightly job emits the per-file summary from
  `cargo llvm-cov report --summary-only`. Raw JSON, summary text, and HTML
  report are uploaded as `coverage-report-rust` (14-day retention).

### E2E conventions

- Every E2E test file ends in `.e2e.test.ts` and is under `apps/e2e/src/`.
- Fixtures live in `apps/e2e/src/helpers/` and must match current adapter and
  schema contracts — broken fixtures fail detection/parse tests first.
- The CLI is now a Rust binary (ADR-011). CLI suites use `cliBinaryAvailable()`
  from `apps/e2e/src/helpers/cli-runner.ts` and skip gracefully when
  `target/{debug,release}/anvil` is absent, so a pure TS run does not require
  `cargo build`.
- Flaky tests retry once (`retry: 1` in `apps/e2e/vitest.config.ts`). Real
  failures still fail the job; two retries were considered excessive.

### OPA + Regal

Both CI workflows install OPA `v1.16.1` (pinned to `DEFAULT_OPA_VERSION` in
`packages/anvil/policy/src/opa-binary-manager.ts`) via
`open-policy-agent/setup-opa`. Regal lints the fixture policies in `rust.yml`.
Locally, the policy tests fall back to the host `opa` if available; the TS
policy tests skip when absent.

## Code Quality

- UK English spelling in plan text and documentation
- Clean, modular, functional code following the project's conventions
- Validate at system boundaries; trust internal code
- No secrets in code — use env vars or secret managers
