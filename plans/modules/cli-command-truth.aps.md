<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# CLI Command Truth

| ID    | Owner | Priority | Status      | Progress |
| ----- | ----- | -------- | ----------- | -------- |
| CLICT | —     | high     | In Progress | 0/7      |

**Last reviewed:** 2026-07-06 (runtime registry + slices 1–6 audited in
`docs/reviews/cli-command-truth-review.md`)

## Purpose

Keep documented CLI commands aligned with what `anvil` actually registers,
dispatches, and tests. Each command family gets a truth review entry in
`docs/reviews/cli-command-truth-review.md` before docs are reconciled or new
subcommands are planned.

This module owns **documentation and APS claim correction**. Build/implement
decisions for missing commands stay in the owning vertical modules (e.g.
ARCHCFG for `anvil architecture`).

## Operating model

One command family per cycle, repeated until the slice queue in
`docs/reviews/cli-command-truth-review.md` is drained:

1. **Audit** — add a slice to the review doc (architecture is slice 1).
2. **Plan** — add a CLICT-00N reconciliation work item (docs only).
3. **Reconcile** — align guides/public docs/APS claims to `anvil <cmd> --help`
   today; document redirects where behaviour already exists under another name.
4. **Build** (optional) — vertical module owns design gate + code (not CLICT).
5. **Re-audit** — refresh the slice and docs after each vertical merge that
   changes the command surface.

CLICT reconciliation is **phase 1 of N** when builds are planned: expect a
follow-up pass after vertical work lands. Do not block phase-1 doc fixes on
build verdicts when redirects are already known.

## In Scope

- Running command-truth audits (doc inventory → runtime → tests → verdict)
- Reconciling guides, runbooks, public docs, and APS/CHANGELOG false-complete claims
- Cross-linking substitute commands where behaviour already exists under another name
- Updating the living review log at `docs/reviews/cli-command-truth-review.md`

## Out of Scope

- Implementing missing CLI subcommands (deferred to vertical modules after design gates)
- Changing gate/check engine behaviour
- Public docs-site host wiring (DSITE) — content corrections that touch
  `docs/public/` may coordinate with DOCSYNC

## Interfaces

**Depends on:**

- `crates/anvil-cli/src/commands/` — runtime source of truth for registration
- `docs/runbooks/cli-surface.md` — authoritative CLI surface runbook
- `docs/reviews/cli-command-truth-review.md` — audit log

**Exposes:**

- Per-family truth tables and reconciliation work items
- Input to vertical design gates (e.g. ARCHCFG-006)

**Coordinates with:**

- [architecture-config-validation](./architecture-config-validation.aps.md) —
  ARCHCFG-006..014 for architecture *build* decisions; CLICT-001 for architecture
  *docs* reconciliation
- [documentation-sync](./documentation-sync.aps.md) — DOCSYNC-012 public policy
  tutorial (coordinate with CLICT-002)
- [policy-value-enforcement-reset](./policy-value-enforcement-reset.aps.md) —
  POLRESET product truth for CLICT-002 policy slice

## Acceptance Criteria

- [ ] Each audited command family has a slice in `cli-command-truth-review.md`
- [ ] Stale guides no longer list commands absent from `anvil <cmd> --help`
- [ ] APS completed-index / CHANGELOG false-complete claims corrected or superseded
- [ ] Substitute commands documented where behaviour already ships elsewhere

## Work Items

### CLICT-001: Architecture docs reconciliation

- **Intent:** Align all user-facing architecture command documentation with
  runtime (`validate` + `show` only under `anvil architecture`) and document
  redirects to existing surfaces (`gate --only-checks import-boundaries`,
  `anvil watch`, `anvil export`, `anvil dashboard architecture`) per the
  2026-07-06 audit in `docs/reviews/cli-command-truth-review.md`.
- **Expected Outcome:**
  - `docs/guides/custom-architecture-policies.md` no longer presents eight
    non-existent subcommands as live; quickstart and CLI section match runbook
    truth or explicit “redirect to …” wording.
  - False-complete APS rows (`OPA-004` init, `TUI-015` visualise) and CHANGELOG
    `visualise` claim corrected or marked superseded.
  - Public docs (`docs/public/anvil/tutorials/architecture.md`,
    `operations/config.md`, `beta-testing-guide.md`) stay consistent with
    `docs/runbooks/cli-surface.md`.
  - Review doc slice 1 updated with reconciliation checklist closed.
- **Scope:** `docs/guides/custom-architecture-policies.md`, `docs/public/anvil/`
  architecture surfaces, `docs/reviews/cli-command-truth-review.md`, APS
  completed records referencing architecture CLI commands
- **Non-scope:** Implementing ARCHCFG-007..014; ARCHCFG-006 verdict (may inform
  redirect wording but does not block doc fixes for known overlaps)
- **Files:**
  - `docs/guides/custom-architecture-policies.md`
  - `docs/public/anvil/tutorials/architecture.md`
  - `docs/public/anvil/operations/config.md`
  - `docs/public/anvil/beta-testing-guide.md`
  - `docs/reviews/cli-command-truth-review.md`
  - `plans/completed.aps.md` / `plans/completed-index.aps.md` (false-complete rows)
  - `CHANGELOG.md` (visualise claim, if still present without implementation)
- **Dependencies:** —
- **Validation:** `pnpm run docs:check`; `pnpm run lint:md`; manual compare
  against `cargo run --bin anvil -- architecture --help`
- **Confidence:** high
- **Status:** Ready

### CLICT-002: Policy docs reconciliation

- **Intent:** Reconcile `anvil policy` and `anvil exception` documentation with
  the post-POLRESET runtime (pack model, starter `install`, `validate`,
  eval-regression CI, exceptions) per slice 2 in
  `docs/reviews/cli-command-truth-review.md`.
- **Expected Outcome:**
  - `docs/runbooks/cli-surface.md` §policy lists all 11 subcommands; notes
    `test` is a discovery stub.
  - `docs/public/anvil/tutorials/policies.md` rewritten around
    `install` → `validate` → `gate --only-checks policy` (coordinate with
    DOCSYNC-012 — same PR or CLICT leads and DOCSYNC-012 closes on merge).
  - Beta guide, changelog, and `tutorial-as-built.md` aligned; archive
    `policy init` claims redirected to `install`.
  - Internal guides remain authoritative; public docs cite them.
  - Slice 2 reconciliation checklist closed.
- **Scope:** `docs/runbooks/cli-surface.md` (policy + exception), `docs/public/anvil/`
  policy surfaces, `docs/architecture/tutorial-as-built.md`, review doc slice 2,
  archive false-complete rows (`docs/archive/planning/TODO.md`)
- **Non-scope:** Policy engine implementation; adversarial probe user docs
  (`eval-regression`/`attack-regression` stay operator/CI focused)
- **Coordinates with:** [documentation-sync](./documentation-sync.aps.md)
  DOCSYNC-012
- **Dependencies:** —
- **Validation:** `pnpm run docs:check`; `pnpm run lint:md`; manual compare
  against `cargo run --bin anvil -- policy --help` and `exception --help`
- **Confidence:** high
- **Status:** Ready

### CLICT-003: Drift docs reconciliation

- **Intent:** Fix public drift tutorial drift: snapshot filename prefix
  (`snapshot-<name>.json`), remove non-existent `--overwrite` flag, and
  correct prerequisites per slice 3 in `cli-command-truth-review.md`.
- **Expected Outcome:**
  - `docs/public/anvil/tutorials/drift.md` matches `drift.rs` snapshot naming.
  - No documented flags absent from `--help`.
  - Slice 3 reconciliation checklist closed.
- **Scope:** `docs/public/anvil/tutorials/drift.md`, review doc slice 3
- **Non-scope:** Drift engine or baseline implementation
- **Dependencies:** —
- **Validation:** `pnpm run docs:check`; `pnpm run lint:md`; manual compare
  against `cargo run --bin anvil -- drift --help`
- **Confidence:** high
- **Status:** Ready

### CLICT-004: Watch docs reconciliation

- **Intent:** Resolve `anvil architecture watch` conflation; align public copy
  with default `--action check` and `--action none` for architecture-only watch
  per slice 4 in `cli-command-truth-review.md`.
- **Expected Outcome:**
  - No live doc teaches `anvil architecture watch`.
  - Public quickstart/ops cross-link `integrations/watch-output.md` where
    `--json` consumers are the audience.
  - Default `--action` change (v0.8+) consistently described.
  - Slice 4 reconciliation checklist closed.
- **Scope:** `docs/public/anvil/`, watch sections in guides/runbooks, review
  doc slice 4
- **Non-scope:** Watch daemon implementation (DSV modules)
- **Dependencies:** CLICT-001 (architecture-watch wording in
  `custom-architecture-policies.md`)
- **Validation:** `pnpm run docs:check`; `pnpm run lint:md`; manual compare
  against `cargo run --bin anvil -- watch --help`
- **Confidence:** high
- **Status:** Ready

### CLICT-005: Gate docs reconciliation

- **Intent:** Align quality-model and public gate vocabulary with
  `check_catalog.rs` canonical names (`import-boundaries`, `secret-detection`)
  while documenting legacy aliases (`architecture`, `secret`) per slice 5.
- **Expected Outcome:**
  - `docs/architecture/quality-model.md` teaches canonical check names + alias
    table sourced from `CHECK_DEFINITIONS`.
  - Public sample output (`sessions.md`, tutorials) uses canonical names with
    alias callouts where legacy names appear.
  - Runbook `gate-config` planned rename stays future tense.
  - Slice 5 reconciliation checklist closed.
- **Scope:** `docs/architecture/quality-model.md`, `docs/public/anvil/concepts/`,
  gate surfaces in runbooks, review doc slice 5
- **Non-scope:** Gate engine or check implementation; renaming `gate-config`
- **Dependencies:** CLICT-001 (architecture check redirect wording)
- **Validation:** `pnpm run docs:check`; `pnpm run lint:md`; manual compare
  against `cargo run --bin anvil -- gate --help` and `check_catalog.rs`
- **Confidence:** high
- **Status:** Ready

### CLICT-006: Intercept and workspace runbook reconciliation

- **Intent:** Close runbook gaps for daemon workspace registration surfaced in
  slice 6: `workspace register`, `unregister`, `install-hook` (ACTMO-015/020).
- **Expected Outcome:**
  - `docs/runbooks/cli-surface.md` §workspace documents all seven subcommands.
  - Public ops (`operations/config.md` or intercept docs) mention worktree
    registration where operators need it.
  - Slice 6 reconciliation checklist closed.
- **Scope:** `docs/runbooks/cli-surface.md` (workspace, intercept), relevant
  public ops surfaces, review doc slice 6
- **Non-scope:** ACTMO implementation; intercept daemon behaviour changes
- **Dependencies:** —
- **Validation:** `pnpm run docs:check`; `pnpm run lint:md`; manual compare
  against `workspace --help` and `intercept --help`
- **Confidence:** high
- **Status:** Ready

### CLICT-007: Tier 2 runbook alignment sweep

- **Intent:** Spot-check the remaining **38** command families in the runtime
  registry (tier 2) against `docs/runbooks/cli-surface.md` and fix runbook-only
  gaps without full slice write-ups unless drift is found.
- **Expected Outcome:**
  - Registry table in `cli-command-truth-review.md` updated: tier 2 families
    marked **aligned** or linked to a fix commit.
  - Any newly discovered false-complete APS/CHANGELOG claims filed as follow-up
    items (not inline TODOs).
  - Acceptance criterion "each audited command family has a slice" satisfied via
    tier 1/1½ slices + tier 2 sweep note.
- **Scope:** `docs/runbooks/cli-surface.md`, `docs/reviews/cli-command-truth-review.md`
- **Non-scope:** Public tutorial rewrites for low-traffic commands; code changes
- **Dependencies:** CLICT-001..006 (priority families first)
- **Validation:** `pnpm run docs:check`; scripted `--help` vs runbook synopsis diff
- **Confidence:** medium
- **Status:** Proposed