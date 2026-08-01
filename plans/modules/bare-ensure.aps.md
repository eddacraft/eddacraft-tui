<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Bare Ensure — Daily On-Switch

| ID   | Owner | Priority | Status  | Progress |
| ---- | ----- | -------- | ------- | -------- |
| ONSW | Josh  | high     | Merged | 6/6     |

**Last reviewed:** 2026-08-01 — **Merged** via PR
[#3474](https://github.com/eddacraft/anvil-001/pull/3474) (`0388a432a` on
`main`, ancestor-checked). ADR-114 Accepted; ONSW-001..006 all Merged.
Public first-class docs (quickstart day-2, beta brief, CHANGELOG) landed with
the v0.10 reframe. Spec:
[`plans/specs/2026-08-01-bare-anvil-ensure.md`](../specs/2026-08-01-bare-anvil-ensure.md).
Conductor JOURNEY-011 closed with the same merge. Pending `v0.10.0-beta`
Released/Shipped evidence.

## Purpose

Make bare `anvil` the discoverable daily on-switch that ensures the per-user
daemon and already-owned MCP are live **without** reinstalling or re-offering
declined optional installs. Keep `anvil start` as the full activation and
reconfigure path.

## In Scope

- Root bare-invocation dispatch (supersede CIB-177 exit-2-always for the ensure
  path)
- Idempotent daemon ensure and worktree spine ensure without first-time project
  writes
- MCP ensure-only for present anvil-owned entries (ADR-044 SafeDrift / UpToDate)
- Honest first-run and NotPresent MCP recovery copy pointing at `anvil start`
- Non-interactive / CI deterministic behaviour
- Help, runbook, and CLICT documentation of the split
- Fixture-pinned exit codes and no-install guarantees

## Out of Scope

- Durable "user declined forever" preference store (v1 uses disk state only)
- Changing ACTTUI consent chrome or unticked defaults on `start`
- Welcome / WOW first-win behaviour
- JOURNEY-009 always-on indicator / ACTMO-021 local control app
- New MCP clients (MCPX)

## Interfaces

**Depends on:**

- [ADR-114](../decisions/114-bare-anvil-ensure-surface.md) — product decision
- [ADR-082](../decisions/082-daemon-lifecycle-user-startup.md) — daemon ensure
- [ADR-092](../decisions/092-mcp-optional-activation-spine.md) — MCP-optional spine
- [ADR-044](../decisions/044-mcp-entry-activation-owned.md) — MCP entry ownership
- [ADR-094](../decisions/094-worktree-registration-ux.md) — worktree registration
- [activation-mcp-optional](./activation-mcp-optional.aps.md) — ensure/register
  primitives
- DLIFE ensure primitive (archived module; code lives under intercept ensure)

**Exposes:**

- Bare `anvil` ensure behaviour and contracts
- Recovery copy that routes reconfigure to `anvil start`

**Coordinates with:**

- [release-user-journeys](./release-user-journeys.aps.md) — JOURNEY-011 gate
- [cli-command-truth](./cli-command-truth.aps.md) — root command truth note
- ACTTUI (done) — start remains the consent owner

## Ready Checklist

Change module status to **Ready** when:

- [x] ADR-114 is **Accepted** (or operator authorises implementation behind
      Proposed with explicit risk acceptance)
- [x] Open questions in the design spec §Open questions are answered or
      deferred with owners
- [x] ONSW-001..006 have enough detail to execute

## Work Items

### ONSW-001: Accept ADR-114 and pin open questions

- **Status:** Merged via [#3474](https://github.com/eddacraft/anvil-001/pull/3474) 2026-08-01 — ADR-114 Accepted; open questions pinned
  (config-Absent predicate, default-on, global `--json`, exit 0/1).
- **Intent:** Land the product decision so implementation does not invent
  first-run or exit-code policy mid-PR.
- **Expected Outcome:** ADR-114 Accepted (or rejected with rationale). Open
  questions resolved: never-activated predicate, default-on vs feature gate,
  bare `--json` in v1, exit codes for not-activated vs daemon-fail. Spec
  amended if needed.
- **Files:** `plans/decisions/114-bare-anvil-ensure-surface.md`,
  `plans/decisions/DECISION-LOG.md`,
  `plans/specs/2026-08-01-bare-anvil-ensure.md`
- **Dependencies:** none
- **Validation:** `pnpm adr:check`; ADR status Accepted; open questions
  closed or explicitly deferred in the ADR.
- **Confidence:** high

### ONSW-002: Bare ensure spine (daemon + worktree)

- **Status:** Merged via [#3474](https://github.com/eddacraft/anvil-001/pull/3474) 2026-08-01 — root `Option<Commands>` dispatch, daemon ensure,
  early registerable-worktree gate (exit 1), durable registration; covered by
  `bare_invocation` + live worktree smoke on #3474.
- **Intent:** Bare root invocation runs idempotent daemon ensure and spine
  attestation without first-time project writes.
- **Expected Outcome:** New thin command path (e.g. `commands/ensure.rs`)
  dispatched from root when no subcommand is present (except `--help`). Reuses
  DLIFE/ACTMO ensure primitives. Outside registerable worktree: honest short
  message + pinned exit. Never runs init, hooks install, or workflow install.
  First-run / never-activated predicate emits recovery pointing at
  `anvil start`.
- **Files:** `crates/anvil-cli/src/main.rs`,
  `crates/anvil-cli/src/commands/ensure.rs` (or equivalent),
  `crates/anvil-cli/src/commands/mod.rs`,
  intercept/activation ensure call sites as needed
- **Dependencies:** ONSW-001
- **Validation:** `cargo test -p eddacraft-anvil ensure`; unit/integration
  fixtures for healthy ensure, outside-repo, never-activated honesty.
- **Confidence:** medium

### ONSW-003: MCP ensure-only (no NotPresent install)

- **Status:** Merged via [#3474](https://github.com/eddacraft/anvil-001/pull/3474) 2026-08-01 — `ensure_existing_mcp_entries`; unit tests
  prove NotPresent never writes and SafeDrift repairs.
- **Intent:** Bare may verify/repair already-owned MCP entries; must not install
  or re-offer NotPresent clients.
- **Expected Outcome:** For UpToDate / SafeDrift anvil entries: verify and
  SafeDrift rewrite per ADR-044. For NotPresent: skip with one recovery line
  naming `anvil start`. Honour `ANVIL_NO_MCP`. No consent plan, no picker.
  Fixture proves no file write when NotPresent after a prior decline.
- **Files:** `crates/anvil-cli/src/commands/ensure.rs`,
  `crates/anvil-cli/src/activation/orchestrator/install.rs` (reuse only;
  prefer shared ensure-only helper over forking start)
- **Dependencies:** ONSW-002
- **Validation:** `cargo test -p eddacraft-anvil ensure`; MCP path fixtures
  NotPresent / UpToDate / SafeDrift / UnsafeDrift / no-mcp.
- **Confidence:** medium

### ONSW-004: Exit codes, non-interactive contract, supersede CIB-177 bare test

- **Status:** Merged via [#3474](https://github.com/eddacraft/anvil-001/pull/3474) 2026-08-01 — exit 0/1/3; `--json` compact; CIB-177
  superseded; `tests/bare_invocation.rs` rewritten.
- **Intent:** Replace the bare-always-exit-2 contract with pinned ensure
  contracts; keep help path correct.
- **Expected Outcome:** Documented exit codes for success, not-activated,
  daemon failure, and help. Non-TTY / CI never prompts. `tests/bare_invocation.rs`
  (or successor) asserts ensure behaviour and help-on-`--help`. CIB-177 pointer
  intent retained in `before_help` where still useful.
- **Files:** `crates/anvil-cli/tests/bare_invocation.rs`,
  `crates/anvil-cli/src/help_layout.rs`, ensure command tests
- **Dependencies:** ONSW-002, ONSW-003
- **Validation:** `cargo test -p eddacraft-anvil --test bare_invocation`;
  `cargo test -p eddacraft-anvil ensure`; non-TTY fixture.
- **Confidence:** high

### ONSW-005: Docs, help copy, CLICT note

- **Status:** Merged via [#3474](https://github.com/eddacraft/anvil-001/pull/3474) 2026-08-01 — `cli-surface.md` bare section, help pointer,
  `flags/surfaces.json` ensure key, CLICT review note.
- **Intent:** Make the split discoverable in help and runbooks.
- **Expected Outcome:** `cli-surface.md` documents bare ensure vs `start`
  reconfigure. Root help blurb names both roles. Public quickstart updated if it
  implies daily `start`. CLICT review log notes root behaviour change. No
  internal IDs in user-visible help (CLIC-010).
- **Files:** `docs/runbooks/cli-surface.md`,
  `crates/anvil-cli/src/help_layout.rs`,
  `docs/reviews/cli-command-truth-review.md`,
  public docs as needed
- **Dependencies:** ONSW-004
- **Validation:** `pnpm docs:check` (or narrow docs lint); help layout unit
  tests; CLICT inventory mentions bare.
- **Confidence:** high

### ONSW-006: Cross-path regression — start still reconfigures

- **Status:** Merged via [#3474](https://github.com/eddacraft/anvil-001/pull/3474) 2026-08-01 — unit tests: ensure never writes NotPresent; SafeDrift repairs; start auto-install path unchanged (`fresh_repo_auto_installs`).
- **Intent:** Prove the split does not break activation or re-offer-on-start.
- **Expected Outcome:** After a decline-on-start fixture, bare does not install
  MCP/workflows; a subsequent interactive (or plain) `start` still offers
  NotPresent clients. Existing `cargo test -p eddacraft-anvil start` and
  activation e2e remain green. JOURNEY-011 consumes this evidence.
- **Files:** start + ensure integration tests; optional e2e path
- **Dependencies:** ONSW-003, ONSW-004
- **Validation:** `cargo test -p eddacraft-anvil start`;
  `cargo test -p eddacraft-anvil ensure`; activation e2e subset if present.
- **Confidence:** medium

## Sequencing

```text
ONSW-001 (ADR accept)
  -> ONSW-002 (spine)
  -> ONSW-003 (MCP ensure-only)
  -> ONSW-004 (contracts) || ONSW-005 (docs can trail slightly)
  -> ONSW-006 (cross-path)
  -> JOURNEY-011 (conductor acceptance)
```

## Risks

| Risk | Mitigation |
| ---- | ---------- |
| Silent first-run writes | §2.3 honesty; fixtures forbid MCP/workflow writes on never-activated |
| Script relies on bare exit 2 | Document in release notes; pin new codes; help still via `--help` |
| Over-claim MCP "on" | Honesty pins; config present ≠ editor attached |
| Scope creep into decline prefs | Explicit non-goal; only start re-offers |

## Acceptance Criteria (module)

- [x] Bare ensure turns daemon spine on for an activated worktree without
      reinstall prompts
- [x] Declined MCP stays uninstalled under bare; `start` can still install
- [x] Help and runbook state the split clearly
- [x] JOURNEY-011 Merged with rehearsal evidence (PR #3474 tests + worktree smoke)
