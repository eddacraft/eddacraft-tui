<!--
APS Module: Rust CLI Tier 2
=========================
Port Tier 2 (utility & operational) commands to crates/anvil-cli/.
Depends on RCLI (Tier 1) foundation being complete.

Scopes: RCLI2 (main)
-->

# Rust CLI — Tier 2

| ID    | Owner | Status      | Progress |
| ----- | ----- | ----------- | -------- |
| RCLI2 | —     | In Progress | 5/9      |

**Last reviewed:** 2026-05-01

> **Status correction 2026-04-26 (freshness audit):** Module flipped from
> `Proposed` to `In Progress`. RCLI2-001..-004 (`check`, `validate`, `drift`,
> `gate-config`) all shipped to `crates/anvil-cli/src/commands/` —
> see commits `1e44ef2d` (RCLI2-001), `c5679432` (RCLI2-002),
> `a2297dca` (RCLI2-003), `06d764d4` (RCLI2-004). Plan was never
> updated when the work landed. Index previously said `0/8 Proposed`;
> actual state was `4/8 In Progress` and the index has been corrected
> in this same change. RCLI2-005..-008 remain `Proposed`
> (still gated on OPAE).
>
> **Post-migration note (2026-04-26):** RCLI Tier 1 is complete (64/64) and
> the Node.js CLI at `apps/anvil-cli/` has been retired. References to the
> Node.js CLI in this module are historical — they describe the source we are
> reaching parity with, not a still-present runtime.

## Purpose

Port Tier 2 utility and operational commands from the historical Node.js CLI
(`apps/anvil-cli/`, retired) to the Rust binary (`crates/anvil-cli/`). These
commands extend the core workflow shipped in RCLI Tier 1 with single-file
checking, plan validation, drift tracking, gate configuration, and policy
tooling.

**Why:** Tier 1 covers the primary workflow loop (init → watch → gate → status)
but omits operational commands needed for CI pipelines (`check`, `pr-comment`),
incremental debugging (`policy-debug`, `drift compare`), and configuration
management (`gate-config`, `validate`). Without parity, parts of the
historical Node.js workflow remain unported.

**ADR:** [012-rust-cli-replacement](../decisions/012-rust-cli-replacement.md)
**Spec:** [2026-03-18-rust-cli-design](../specs/2026-03-18-rust-cli-design.md) §6 Tier 2

## Language Guardrails

This module extends Anvil's quality-facing CLI, so it must follow the canonical
language defined in
`plans/specs/2026-04-21-anvil-quality-language-design.md`.

- Use `check` for the smallest evaluative unit
- Use `gate` for workflow judgement over one or more checks
- Use `finding` as the generic result noun where the output spans warnings,
  violations, and similar result types
- Use `scan` for evidence-gathering actions, not as the primary user model
- Avoid introducing new command copy that treats `gate` as a generic synonym
  for any control, preflight, or config surface

## In Scope

- 8 commands (or command groups) classified as Tier 2 in the design spec
- Integration with existing Rust crates: anvil-kernel (gate/check), anvil-policy
  (policy-debug, policy-watch), anvil-architecture (drift)
- JSON and plain-text output modes for all commands
- Kindling provenance integration where applicable (check, gate-config)
- Admin command parity port: bring `anvil admin` to feature parity with the
  Node operator CLI (`apps/admin-cli/`) and add a CLI surface for
  `POST /admin/user/email-update`

## Out of Scope

- Tier 3 commands (subsystems — see rust-cli-tier3.aps.md)
- New features not present in the Node.js CLI
- TUI surfaces (Tier 2 commands are headless; no new Ratatui surfaces)
- OPA binary distribution (uses system-installed OPA or bundled Wasm — decided
  in OPAE module)

## Interfaces

**Depends on:**

- RCLI — Foundation crate structure, clap entry point, output formatters,
  auth middleware
- KERN — Kernel watcher and event emission (for policy-watch)
- OPAE — Policy commands policy-debug, policy-watch, pr-comment, exception
  require OPAE work items to be implemented in TypeScript first (or ported
  from spec directly). Items marked with ‡ below depend on OPAE completion

**Exposes:**

- 8 additional subcommands on the `anvil` binary
- Drift snapshot storage compatible with Node.js CLI format

## Constraints

- Output parity with the historical Node.js CLI JSON schema and exit codes
  (the canonical contract; the Node.js CLI itself is retired)
- Drift snapshots remain backwards-readable with the historical Node.js CLI
  format so older snapshots in `.anvil/snapshots/` keep working
- Commands that depend on unimplemented OPAE items are deferred until the
  domain logic exists (in Rust or TypeScript)
- Same error handling conventions as RCLI: `anyhow` for application code,
  `thiserror` for library errors
- User-facing help, docs, and task copy should describe results as findings
  unless a narrower subtype is genuinely required

## Ready Checklist

Change status to **Ready** when:

- [x] RCLI Tier 1 foundation complete (64/64)
- [ ] RCLI Phase 7 rework items resolved (gate checks, auth migration,
  command registration) — rework items identified by 2026-03-24 audit
- [ ] RCLI-017 (anvil-policy crate) has evaluator implemented (PR #640
  in progress)
- [ ] RCLI-019 (anvil-architecture crate) has validation logic implemented
- [ ] OPAE status reviewed — identify which Tier 2 commands can proceed
- [ ] Re-confirm which RCLI2 items remain outstanding versus already shipped
  in `crates/anvil-cli/src/commands/` (check, validate, drift, gate_config,
  policy.rs already present as of 2026-04-26)

---

## Work Items

#### Phase 1 — Check & Validate
### RCLI2-001: check command

- **Status:** Done (commit `1e44ef2d`; file present at `crates/anvil-cli/src/commands/check.rs`, ~1347 LOC)
- **Intent:** Port `anvil check` (planless file analysis). Supports file
  selection modes: explicit paths, `--all`, `--changed`, `--staged`,
  `--since <ref>`. Runs GateRunner in planless mode. Supports interactive
  review with nudge coaching, suppression filtering, and caching
- **Language Note:** `check` is the evaluative command; result summaries should
  use `findings` as the generic noun, with `warning` or `violation` reserved
  for specific severities/types
- **Expected Outcome:** `anvil check --changed` analyses modified files and
  reports findings with severity levels; `--json` produces machine-readable
  output; `--interactive` prompts per-warning
- **Validation:** Finding counts and severities match the historical Node.js
  CLI JSON contract for the same project state; exit code 0 on clean, 1 on
  warnings
- **Files:** `crates/anvil-cli/src/commands/check.rs` (file already exists —
  confirm whether interactive mode + selection flags are still outstanding)
- **Confidence:** medium (664 LOC in historical Node.js; interactive mode
  needs crossterm prompts)
- **Priority:** High
- **Dependencies:** RCLI (foundation), KERN (gate runner)

---

### RCLI2-002: validate command

- **Status:** Done (commit `c5679432`; file present at `crates/anvil-cli/src/commands/validate.rs`, ~890 LOC)
- **Intent:** Port `anvil validate <plan>`. Auto-detects plan format (APS,
  SpecKit, BMAD), validates structure, verifies content hash integrity.
  Supports `--format` override and `--no-validate-hash`
- **Expected Outcome:** `anvil validate plan.aps.md` reports validation issues
  with line numbers; shows detected format and confidence
- **Validation:** Validation results match the historical Node.js CLI for the
  same plan files; hash verification produces identical pass/fail
- **Files:** `crates/anvil-cli/src/commands/validate.rs` (file already exists —
  confirm scope gap)
- **Confidence:** high (192 LOC in historical Node.js; straightforward file
  parsing)
- **Priority:** Medium
- **Dependencies:** RCLI (foundation)

---

#### Phase 2 — Drift & Gate Config

### RCLI2-003: drift command

- **Status:** Done (commit `a2297dca`; file present at `crates/anvil-cli/src/commands/drift.rs`, ~1059 LOC)
- **Intent:** Port `anvil drift` with four subcommands: `snapshot` (capture
  baseline), `compare` (diff two snapshots), `report` (longitudinal analysis),
  `list` (enumerate snapshots). Reads/writes `.anvil/snapshots/` directory.
  Supports `--json` output for all subcommands
- **Language Note:** Drift is a reporting surface over findings and state
  changes; avoid letting it establish a separate result vocabulary from the
  core checks/findings/gates model
- **Expected Outcome:** `anvil drift snapshot --name release-1.0` captures
  current state; `anvil drift compare s1 s2` shows deltas in violations,
  antipatterns, and suppressions
- **Validation:** Snapshot JSON format is identical to the historical Node.js
  CLI; comparison metrics (net_change, trend, violation deltas) match for the
  same input
- **Files:** `crates/anvil-cli/src/commands/drift.rs` (file already exists —
  confirm subcommand coverage)
- **Confidence:** medium (324 LOC in historical Node.js; requires
  SnapshotCaptureService port or Rust equivalent)
- **Priority:** High
- **Dependencies:** RCLI (foundation), RCLI-019 (anvil-architecture crate)

---

### RCLI2-004: gate-config command

- **Status:** Done (commit `06d764d4`; file present at `crates/anvil-cli/src/commands/gate_config.rs`, ~432 LOC)
- **Intent:** Port `anvil gate-config`. List, enable, disable, and
  interactively configure gate checks and thresholds. Reads/writes
  `.anvil/gate-config.json`
- **Language Note:** Make clear in help and docs that this surface configures
  the checks used by gate evaluation; avoid implying that checks and gates are
  separate unrelated systems
- **Expected Outcome:** `anvil gate-config --list` shows current config;
  `--enable policy` enables a check; `--interactive` walks through all
  settings
- **Validation:** Config file format is identical to the historical Node.js
  CLI; enable/disable produces the same JSON mutations
- **Files:** `crates/anvil-cli/src/commands/gate_config.rs` (file already
  exists — confirm scope gap)
- **Confidence:** high (131 LOC in historical Node.js; straightforward JSON
  CRUD)
- **Priority:** Medium
- **Dependencies:** RCLI (foundation)

---

#### Phase 3 — Policy Utilities

### RCLI2-005: policy-debug command ‡

- **Status:** Proposed
- **Intent:** Port `anvil policy-debug`. Interactive policy debugging with
  step-through evaluation, variable inspection, and rule tracing. Requires
  OPA integration
- **Expected Outcome:** `anvil policy-debug <rule>` shows evaluation trace
  with variable bindings at each step
- **Validation:** Debug trace matches OPA's native `--explain` output format
- **Files:** `crates/anvil-cli/src/commands/policy_debug.rs`
- **Confidence:** low (deferred after the OPAE reset; no first-slice debugger)
- **Priority:** Medium
- **Dependencies:** RCLI-017 (anvil-policy crate), future policy-debugger module
- **Notes:** ‡ Blocked by the 2026-07-02 OPAE reset. The first policy slice does
  not include an interactive debugger.

---

### RCLI2-006: policy-watch command ‡

- **Status:** Proposed
- **Intent:** Port `anvil policy-watch`. Watches policy files (`.rego`,
  `.yaml`) for changes and re-evaluates on save. Uses kernel file watcher
  with policy-specific event handling
- **Expected Outcome:** `anvil policy-watch` monitors policy directory and
  reports validation errors in real time
- **Validation:** File change triggers re-evaluation within 500ms; errors
  reported correctly
- **Files:** `crates/anvil-cli/src/commands/policy_watch.rs`
- **Confidence:** low (deferred after the OPAE reset; first slice uses existing
  save-time/pre-write surfaces)
- **Priority:** Low
- **Dependencies:** RCLI-017, KERN (watcher), future policy-watch module

---

#### Phase 4 — CI Integration

### RCLI2-007: pr-comment command ‡

- **Status:** Proposed
- **Intent:** Port `anvil pr-comment`. Generates PR annotations from gate
  results. Formats findings as GitHub PR review comments with file/line
  context, severity badges, and summary table
- **Expected Outcome:** `anvil pr-comment --gate-output results.json` posts
  annotations to the current PR via GitHub API
- **Validation:** Annotations appear on correct lines in PR; summary table
  counts match gate results
- **Files:** `crates/anvil-cli/src/commands/pr_comment.rs`
- **Confidence:** low (deferred after the OPAE reset; first slice does not include
  broad PR auto-comments)
- **Priority:** Medium
- **Dependencies:** RCLI (foundation), future PR-comment policy surface

---

### RCLI2-008: exception command ‡

- **Status:** Proposed
- **Intent:** Port `anvil exception`. Manage policy exceptions — create,
  list, approve, revoke. Supports time-bounded exceptions with approval
  workflows
- **Expected Outcome:** `anvil exception create --rule ARCH-001 --reason "..."
  --expires 30d` creates a scoped exception; `anvil exception list` shows
  active exceptions
- **Validation:** Exception storage format matches the historical Node.js CLI
  contract; expired exceptions are correctly filtered
- **Files:** `crates/anvil-cli/src/commands/exception.rs`
- **Confidence:** medium (now depends on EXCEPT CLI work, not OPAE)
- **Priority:** Low
- **Dependencies:** RCLI (foundation), EXCEPT-004, EXCEPT-005

---

#### Phase 5 — Admin Parity
### RCLI2-009: admin command parity (list/show/revoke/audit/send-migration/email-update)

- **Status:** Done
- **Intent:** Bring `anvil admin` to feature parity with the historical Node
  operator CLI (`apps/admin-cli/`, binary `anvil-admin`) and add a CLI surface
  for `POST /admin/user/email-update`, which previously had no CLI. RCLI-016
  (Tier 1) ported only `approve` and `invite`; this item closes the remaining
  operator parity gap.
- **Expected Outcome:** Each subcommand below is callable on the Rust binary,
  authenticates via `ANVIL_ADMIN_KEY` (same env contract as
  `commands/admin.rs` today), supports `--json`, and surfaces
  `EXIT_AUTH_REQUIRED = 3` on missing/invalid credentials:
  - `anvil admin list [--status pending|approved|all] [--source manual|website|import|all] [--limit N] [--offset N]`
    → `GET /admin/waitlist`
  - `anvil admin show <email>` → `GET /admin/user/:email`
    (user, tokens, recent audit)
  - `anvil admin revoke [<email>] [--token <raw>] [-y]`
    → `POST /admin/revoke`
  - `anvil admin audit [--action <a>] [--filter-actor <e>] [--limit N] [--offset N]`
    → `GET /admin/audit`
  - `anvil admin send-migration [--source import|...] [--limit N] [--no-dry-run] [-y]`
    → `POST /admin/send-migration`; preserves the dry-run → `previewToken`
    → real-send flow (snapshot TTL 10 min, cohort-drift error handling per
    `apps/anvil-api/src/routes/admin.ts:539`)
  - `anvil admin email-update <current-email> <new-email>` (new — no Node
    equivalent) → `POST /admin/user/email-update`
- **Validation:**
  - JSON output schemas match the responses defined in
    `apps/anvil-api/src/routes/admin-schemas.ts` and the existing Node CLI
    expectations (`apps/admin-cli/src/__tests__/`)
  - `send-migration` round-trip honours `preview_token_required`,
    `preview_token_missing`, `preview_token_consumed`, `preview_token_expired`,
    and `cohort_drift` error codes with operator-friendly messages
  - Confirmation prompts (`-y` to skip) on destructive verbs (`revoke`,
    real-send `send-migration`); EOF on the prompt aborts cleanly
  - Integration tests against `wiremock` follow the existing
    `crates/anvil-cli/src/auth/client.rs` pattern; unit tests for arg parsing
    follow `crates/anvil-cli/src/commands/admin.rs` tests
- **Files:**
  - `crates/anvil-cli/src/commands/admin.rs` — extend `AdminCommand` enum
    and `run()` with the six subcommands
  - `crates/anvil-cli/src/auth/client.rs` — add `list_waitlist`,
    `get_user`, `revoke_*`, `list_audit`, `send_migration_dry_run`,
    `send_migration_commit`, `update_user_email` methods on `AnvilClient`
  - `crates/anvil-cli/src/output/` — table/JSON formatters for waitlist,
    audit, and user-with-tokens views (reuse the existing JSON printer)
  - Update `docs/cli/` admin reference once shipped
- **Confidence:** high — pure parity port over an established API surface;
  the Node CLI is the canonical contract and is well-tested
- **Priority:** Medium (operator quality-of-life; unblocks retiring
  `apps/admin-cli/` once parity lands)
- **Dependencies:** RCLI (foundation; already complete)
- **Notes:**
  - Once parity ships, `apps/admin-cli/` can be archived alongside
    `archive/anvil-cli-node/` to keep one operator surface
  - `email-update` is the only net-new CLI surface; the rest are 1:1 ports
  - Admin commands are already on the `requires_auth` bypass list
    (RCLI-016 precedent) — they auth on `ANVIL_ADMIN_KEY` directly, not
    user credentials

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| OPAE items not implemented (‡ commands) | High | Medium | Defer ‡ commands; ship Tier 2 without them; they join Tier 2 when OPAE lands |
| SnapshotCaptureService port complexity | Medium | Medium | Share analysis logic with gate command; avoid duplicating check infrastructure |
| Interactive mode (check --interactive) | Low | Low | Use crossterm raw mode with simple Y/N/S prompts; no full TUI needed |
| Drift snapshot format divergence | Low | Medium | Write format compatibility test against historical snapshots; snapshot header includes schema version |
| Module scope already partly delivered by RCLI | Medium | Low | Audit `crates/anvil-cli/src/commands/` against this list before starting; remove or downscale items already complete |

## Stats

| Phase | Items | Status |
| ----- | ----- | ------ |
| 1 — Check & Validate | 2 | Proposed |
| 2 — Drift & Gate Config | 2 | Proposed |
| 3 — Policy Utilities | 2 | Proposed (blocked on OPAE) |
| 4 — CI Integration | 2 | Proposed (blocked on OPAE) |
| 5 — Admin Parity | 1 | Complete |
| **Total** | **9** | — |
