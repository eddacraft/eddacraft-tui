<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# CLI Command Truth

| ID    | Owner | Priority | Status      | Progress |
| ----- | ----- | -------- | ----------- | -------- |
| CLICT | —     | high     | In Progress | 0/5      |

**Last reviewed:** 2026-07-06

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

- **Intent:** Audit and reconcile `anvil policy` documentation against runtime
  subcommands and the regorus-backed policy engine path.
- **Expected Outcome:** Slice 2 in `cli-command-truth-review.md`; guides and
  public docs match `anvil policy --help`; false-complete or legacy OPA claims
  corrected.
- **Scope:** `docs/guides/`, `docs/public/anvil/`, `docs/runbooks/cli-surface.md`
  (policy section), review doc slice 2
- **Non-scope:** Policy engine implementation (OPAE, POLENG, etc.)
- **Dependencies:** —
- **Validation:** `pnpm run docs:check`; `pnpm run lint:md`; manual compare
  against `cargo run --bin anvil -- policy --help`
- **Confidence:** medium
- **Status:** Proposed

### CLICT-003: Drift docs reconciliation

- **Intent:** Audit and reconcile `anvil drift` documentation against runtime
  subcommands (`snapshot`, `compare`, `report`, `list`, etc.).
- **Expected Outcome:** Slice 3 in review doc; tutorial and ops docs match
  `anvil drift --help`.
- **Scope:** `docs/public/anvil/tutorials/drift.md` (if present), drift
  surfaces in guides/runbooks, review doc slice 3
- **Non-scope:** Drift engine or baseline implementation
- **Dependencies:** —
- **Validation:** `pnpm run docs:check`; `pnpm run lint:md`; manual compare
  against `cargo run --bin anvil -- drift --help`
- **Confidence:** medium
- **Status:** Proposed

### CLICT-004: Watch docs reconciliation

- **Intent:** Audit and reconcile `anvil watch` documentation against the
  daemon-backed watch surface; resolve conflation with documented-but-absent
  `anvil architecture watch`.
- **Expected Outcome:** Slice 4 in review doc; watch lifecycle/NDJSON docs match
  runtime; architecture-watch redirects explicit where needed.
- **Scope:** `docs/public/anvil/`, watch sections in runbooks/guides, review
  doc slice 4
- **Non-scope:** Watch daemon implementation (DSV modules)
- **Dependencies:** CLICT-001 (architecture-watch wording may overlap)
- **Validation:** `pnpm run docs:check`; `pnpm run lint:md`; manual compare
  against `cargo run --bin anvil -- watch --help`
- **Confidence:** medium
- **Status:** Proposed

### CLICT-005: Gate docs reconciliation

- **Intent:** Audit and reconcile `anvil gate` / `gate-config` documentation
  against check aliases, profiles, and quality-model vocabulary.
- **Expected Outcome:** Slice 5 in review doc; check-name aliases (`architecture`
  vs `import-boundaries`) and profile docs match `gate.rs` / `check_catalog.rs`.
- **Scope:** `docs/architecture/quality-model.md`, gate surfaces in public docs
  and runbooks, review doc slice 5
- **Non-scope:** Gate engine or check implementation
- **Dependencies:** CLICT-001 (import-boundaries / architecture alias)
- **Validation:** `pnpm run docs:check`; `pnpm run lint:md`; manual compare
  against `cargo run --bin anvil -- gate --help`
- **Confidence:** medium
- **Status:** Proposed