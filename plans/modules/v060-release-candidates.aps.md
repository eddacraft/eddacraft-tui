<!--
APS Module: v0.6.0-beta Release Candidates
==========================================
Capture-as-you-find module for items targeted at the next release window
after v0.5.0-beta (which shipped 2026-05-01). Holds two kinds of entries:

  1. Deferrals from the v0.5.0-beta council / post-tag findings that
     were judged non-blocking for the v0.5.0 cut but should ride the
     next release rather than rot as silent debt.

  2. Forward-looking nominations — work items from other modules that
     are being earmarked for the next release window because they're
     small, low-risk, high-leverage, or obviously slot-fitting.

This is a *capture* surface, not a *commitment* surface. Anything in
here is a candidate, not a guarantee. Sequencing is owed against
plans/next-steps.md (the strategic frame) at cherry-pick time.

Naming: file is `v060` because items here ride the *next* release after
v0.5.0; if the next release tags as v0.5.1 (patch) instead of v0.6.0
(minor), rename the file then — the prefix V060F stays stable on items
already filed, but the module title and file should reflect the actual
target tag once chosen.

See: plans/aps-rules.md
-->

# v0.6.0-beta Release Candidates

| ID    | Owner | Status      | Progress |
| ----- | ----- | ----------- | -------- |
| V060F | —     | In Progress | 1/11     |

**Last reviewed:** 2026-05-07 (added V060F-002..V060F-011 from the v0.6.0-beta as-built sweep — discrepancies surfaced while the as-built docs for intercept, activation, MCP shim, checks, and kernel were being written against HEAD `97b61fd0`)
**Predecessor:** [v050-release-followups](./v050-release-followups.aps.md)
**Sequencing context:** [plans/next-steps.md](../next-steps.md)

## Purpose

Hold the running list of items targeted at the next release after
v0.5.0-beta. The previous module (`v050-release-followups`) is now
historical — its target shipped — so new follow-ups and candidates land
here.

Two intake paths:

- **Deferrals** — anything the v0.5.0 council / external review flagged
  but that was consciously deferred so the tag could ship; anything the
  release run itself surfaced (workflow failures, post-deploy gaps,
  publisher bugs) that was patched manually but needs a permanent fix.
- **Nominations** — small or high-leverage items from other modules
  that the team wants to slot into the next release. These are
  pointers, not duplicates: the canonical tracking stays in the source
  module, V060F just records the nomination + rationale.

Each entry should carry enough context that a future reader can decide
whether to keep, drop, or reschedule it without rerunning the
discovery.

## In Scope

- Deferrals from v0.5.0-beta release prep (council rounds, external
  reviews, post-tag workflow / deploy / publisher findings)
- Hardening items born from v0.5.0 production runtime that should ride
  the next tag rather than wait
- Forward nominations from active modules where a specific work item
  is earmarked for the next release (recorded as a pointer, not a
  re-spec)

## Out of Scope

- Items already tracked in `v050-release-followups` that didn't ride
  v0.5.0 — those need a status reconciliation in that module first
  (mark Complete if they shipped, roll forward to V060F only if they
  remain open and the rationale still applies)
- Net-new feature work — features belong in their own module; V060F
  only nominates work items, not feature concepts
- Items gated on un-staffed dependencies (e.g. blocked on OPAE) —
  parking them here just creates noise

## Intake Conventions

- **Deferral entry:**
  - **Surface:** file/line or commit/PR
  - **Flagged by:** council reviewer, external review, or release run
  - **Intent:** what's broken / what hardening is needed
  - **Expected outcome:** the resolution shape
  - **Confidence:** high / medium / low
  - **Status:** Open by default; flip to Complete when shipped
- **Nomination entry:**
  - **Source module + work item:** e.g. `RCLI2 / RCLI2-009`
  - **Why earmark:** one line on why this fits the next-release window
    (size / risk / leverage)
  - **Status:** Nominated until the source item flips Complete

---

## Tasks

### V060F-001: admin command parity for `anvil admin` (nomination)

- **Source:** [RCLI2-009](./rust-cli-tier2.aps.md#rcli2-009-admin-command-parity-listshowrevokeauditsend-migrationemail-update)
- **Status:** Complete
- **Intent:** Track RCLI2-009 as the nominated admin parity candidate for the
  next release window until the source work item is complete.
- **Why earmark:** Operator-experience papercut — RCLI-016 only ported
  `approve` and `invite`, leaving the other admin surfaces on the historical
  Node binary `anvil-admin` (`apps/admin-cli/`). RCLI2-009 closes that parity
  gap with a 1:1 port over a well-tested API surface, plus one new CLI surface
  for the existing `POST /admin/user/email-update` endpoint. High confidence,
  medium priority, no policy/OPAE dependency.
- **Cuts:** unblocks retiring `apps/admin-cli/` for a single operator
  surface; closes the `anvil admin list` ergonomic gap that prompted
  this nomination.
- **Filed:** 2026-05-01

---

### V060F-002: `anvil intercept stop` CLI subcommand

- **Surface:** `crates/anvil-cli/src/commands/intercept.rs:22-30` (clap enum)
  vs `crates/anvil-intercept/src/lib.rs` (`Shutdown` channel +
  `wait_for_shutdown_signal`)
- **Flagged by:** intercept-as-built §16 gap 1 (2026-05-07); discovered
  while reconciling the v0.6.0-beta release runbook against HEAD
- **Intent:** The daemon-side shutdown signals are wired (the foreground
  `run_foreground` path responds to SIGINT/SIGTERM cleanly, flushing
  fence state and unbinding the IPC listener). The CLI surface only
  declares `Start` and `Status` — there is no `anvil intercept stop`.
  Operators today stop the daemon via Ctrl-C in its foreground terminal
  or `kill <PID>` externally. The runbook now reflects this honestly,
  but a small CLI wrapper closes the operator-surface gap.
- **Expected outcome:** Add `Stop` variant to `InterceptCommand`. On
  Unix, send SIGTERM to the per-user PID file's PID; on Windows, signal
  the named-pipe equivalent. Foreground-only daemon means the command
  is a thin lookup-and-signal wrapper. Update the runbook §1 and §3
  framing once shipped.
- **Confidence:** high
- **Status:** Open

---

### V060F-003: `anvil intercept unblock` CLI subcommand

- **Surface:** `crates/anvil-intercept/src/fence.rs`
  (`FenceStore::unblock_worktree` data path) vs
  `crates/anvil-cli/src/commands/intercept.rs:22-30` (CLI gap)
- **Flagged by:** intercept-as-built §16 gap 1 (2026-05-07); the runbook
  originally documented `unblock` as the canonical fence recovery, but
  the CLI surface was never wired
- **Intent:** Daemon-side `FenceStore::unblock_worktree` ships and
  `--all` is straightforward. v1 recovery is `rm -rf` of
  `${XDG_DATA_HOME:-$HOME/.local/share}/anvil`, which destroys all
  fence state for the user. A worktree-scoped CLI recovery is the
  intended operator UX per `plans/specs/2026-04-26-rtai-demo-runbook.md`
  §3.1.
- **Expected outcome:** Add `Unblock { worktree: Option<PathBuf>, all:
  bool }` subcommand routing through the existing daemon data path
  over IPC. Reframe runbook §3 / §4 recovery instructions back to
  `anvil intercept unblock` once shipped.
- **Confidence:** high
- **Status:** Open

---

### V060F-004: macOS `current_process_start_time` helper branch

- **Surface:** `crates/anvil-intercept/src/interrupt.rs:419-431`
- **Flagged by:** intercept-as-built §16 gap 3, security note H4
  (2026-05-07)
- **Intent:** The Linux helper reads `/proc/PID/stat` field 22; the
  macOS branch returns `None` unconditionally. AD-7's
  fence-on-failure invariant forces a fence on every interrupt
  decision against a session with a recorded `started_at_unix`, so
  the macOS interrupt ladder is fence-first instead of running the
  SIGINT → SIGTERM → SIGKILL ladder. This shows up disproportionately
  in macOS fence telemetry as `FenceReason::SignalDeliveryFailed`.
- **Expected outcome:** Add a macOS branch using `proc_pidinfo` to
  read `pbi_start_tvsec`. Once present, macOS interrupt decisions can
  run the same signal ladder as Linux. The PID-reuse defence becomes
  symmetric; macOS-vs-Linux fence telemetry skew goes away.
- **Confidence:** medium (libproc binding choice; needs to match the
  existing Linux call shape so AD-7's logic doesn't diverge)
- **Status:** Open

---

### V060F-005: Windows MCP `daemonStatus` wiring (`chore/windows-status`)

- **Surface:** `crates/anvil-cli/src/mcp/validation.rs:142-148` (cfg gate)
  and `:371-382` (mapping); compare to the working Win32 client at
  `crates/anvil-cli/src/commands/intercept.rs:143-148`, `:170+`
- **Flagged by:** intercept-as-built §16 gap 9, mcp-shim-as-built G-01
  (2026-05-07); rides the `chore/windows-status` workstream
- **Intent:** The MCP `LocalDaemonValidationClient::validate_pre_write`
  is `cfg(unix)`-gated; the `cfg(not(unix))` arm returns
  `DaemonValidationOutcome::Unavailable`, which maps to
  `DaemonStatus::NotWired`. The `correlation.daemonStatus` envelope
  cannot distinguish daemon-up from daemon-down on Windows in v1.
  Notably, the `anvil intercept status` CLI itself works fine on
  Windows via `query_daemon_status_windows_at` and
  `connect_owner_only_pipe_client` — only the MCP-side validation
  client is gated.
- **Expected outcome:** Route the MCP validation client through the
  same Win32 named-pipe path the status CLI already uses. No new
  transport — reuse the synchronous helpers in `anvil-intercept-win32`.
  Remove the `cfg(unix)` gate on `LocalDaemonValidationClient`.
- **Confidence:** high (the path is proven by the status CLI; the
  validation client is one more consumer of the same primitives)
- **Status:** Open

---

### V060F-006: LAUNCH-016 `extensions:` user-config opt-in across CLI seams

- **Surface:** `crates/anvil-cli/src/commands/check.rs`,
  `commands/watch.rs`, `commands/audit.rs`; partition seam at
  `crates/anvil-cli/src/activation/language_profile.rs`
- **Flagged by:** activation-as-built §"Known gaps" — LAUNCH-016
  acceptance criterion (d) marked hand-off (2026-05-07)
- **Intent:** The language profile gate
  `partition_for_language_specific_checks` is wired in the activation /
  init path (the skipped-languages line prints when non-empty), but
  `commands::check`, `commands::watch`, and `commands::audit` do not
  yet honour the user-config-aware `extensions:` opt-in. A user with
  Python files who explicitly opts in via `.anvilrc`'s `extensions:`
  cannot today get language-specific antipattern checks against those
  files via these CLI surfaces.
- **Expected outcome:** Compose the user-config decision before
  invoking `partition_for_language_specific_checks` at each CLI seam.
  Test that explicit `extensions:` opt-in reverses the
  unsupported-language skip without affecting the cross-language
  secret scan (which already runs on all files).
- **Confidence:** medium
- **Status:** Open

---

### V060F-007: Watch-liveness probing (LAUNCH-011 follow-up)

- **Surface:** `crates/anvil-cli/src/activation/diagnostic.rs` (WatchTier
  probe) and `commands/start.rs --verify` path
- **Flagged by:** activation-as-built §"Known gaps" (2026-05-07);
  acknowledged in the LAUNCH-014 protection-loop tutorial copy
- **Intent:** `anvil start --verify` enumerates what it actually probes
  today (config, MCP entries, baseline, language profile) but does not
  probe watch-liveness. The diagnostic cannot honestly assert
  `WatchTier::Running` without a prior watch spawn in the same
  process. The LAUNCH-014 tutorial step 5 documents this as a known
  gap.
- **Expected outcome:** Add a watch-liveness probe to the verify path
  — either a kernel watcher health endpoint, or a fast cross-check
  against a known-recent file event in the foreground process. Once
  wired, the protection-loop tutorial copy can drop the watch caveat.
- **Confidence:** medium
- **Status:** Open

---

### V060F-008: `rust-kernel-spec.md` status header refresh

- **Surface:** `docs/architecture/rust-kernel-spec.md` line 3
- **Flagged by:** kernel-as-built §2 reconciliation (2026-05-07)
- **Intent:** The spec's status header still says "Proposed — H1
  Implementation Target". The kernel has shipped through several beta
  tags; the spec is intent-of-record and the new
  `kernel-as-built.md` is the as-built record. The header drift makes
  it easy to misread the spec as still-aspirational.
- **Expected outcome:** Update the status header to something like
  "Spec — H1 Design Intent (kernel as-built supersedes for current
  state)". Add a one-line cross-link to `kernel-as-built.md` near the
  top so readers land on the as-built first if they want shipping
  state.
- **Confidence:** high (small doc edit)
- **Status:** Open

---

### V060F-009: `quality-model.md` watch → checks dispatch framing

- **Surface:** `docs/architecture/quality-model.md`
- **Flagged by:** checks-as-built §9.4 (2026-05-07)
- **Intent:** The quality-model doc implies `anvil watch` directly
  dispatches into checks. At HEAD the kernel watcher emits change
  events; check evaluation is deferred to `anvil-intercept-rules`
  (when the daemon is wired) or to a manual `anvil check` re-run.
  The framing is misleading for a contributor reading the model
  first.
- **Expected outcome:** One-paragraph clarification distinguishing
  the intended end-state (direct dispatch) from the v0.6.0-beta
  reality (event emission + deferred evaluation via daemon or
  re-run). Cross-link `checks-as-built.md` §9.4 for the live
  state.
- **Confidence:** high (small doc edit)
- **Status:** Open

---

### V060F-010: APS module-ownership alignment for the checks pipeline

- **Surface:** `plans/index.aps.md`, `plans/modules/`,
  `docs/architecture/checks-as-built.md` header pointer
- **Flagged by:** checks-as-built (2026-05-07)
- **Intent:** Writing `checks-as-built.md` surfaced that no single
  canonical APS module owns the v0.6.0-beta-era checks pipeline.
  RENG (engine ports) is the historical owner but has progressed;
  ownership today is split across `scan-performance` (SCAN-NNN),
  `surface-env-files`, `realtime-ai-validation`,
  `ai-guardrail-profile`, plus archived modules
  (`anvil-rust-scanner`, `anvil-scanner-parity-gaps`,
  `anvil-ts-scanner-retirement`). The as-built points at the split,
  but `plans/index.aps.md` doesn't make the decision discoverable.
- **Expected outcome:** Triage decision: either consolidate
  ownership under a `checks-pipeline` module, or document the
  multi-module split inside the index so future readers can find
  the right home for new check work without rerunning the
  discovery.
- **Confidence:** medium
- **Status:** Open

---

### V060F-011: Kernel spec/code divergences (parser, AST snapshot, Heartbeat, declarative invariants)

- **Surface:** `crates/anvil-kernel/src/parser/languages.rs:5-22`,
  `crates/anvil-kernel/src/embedded.rs`,
  `crates/anvil-kernel/src/protocol/emitter.rs`,
  `crates/anvil-kernel/src/policy/`; spec at
  `docs/architecture/rust-kernel-spec.md` §§5.2 / 6.4 / 7.2 / 8.2 /
  9.3
- **Flagged by:** kernel-as-built §2 + §15 (2026-05-07)
- **Intent:** Five spec items are not built (or are built differently
  from the spec):
  - **Rust language parser (§5.2):** named as a dogfooding target;
    the registry has TS / TSX / JS / JSX only.
  - **AST snapshot to disk (§6.4):** named as fast-follow; cold
    rebuild on every start is the only path.
  - **`Heartbeat` event payload (§8.2):** named as future;
    `EventPayload` is `Progress | Snapshot | Violation | Error`.
  - **Declarative invariant DSL (§7.2):** invariants are Rust-only;
    declarative work moved to the separate `anvil-policy` crate,
    which the kernel does not call into.
  - **Daemon-mode kernel transport (§9.3):** superseded into INTD per
    ADR-030; KERN-050..052 are tracked in `plans/index.aps.md:238`.
- **Expected outcome:** Per-item triage. Each is independent. Either
  (a) implement the spec item, (b) update the spec to mark it
  moved/dropped/deferred, or (c) acknowledge as a known gap in the
  kernel as-built and leave the spec dormant. Pick per item.
- **Confidence:** low (mixed scope)
- **Status:** Open

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Module accumulates aspirational nominations and stops being a real release-window list | High | Medium | Re-run a triage pass at cherry-pick time; demote nominations that no longer fit |
| `v050-release-followups` open items not reconciled before V060F starts collecting | Medium | Low | Add a one-line status for each open V050F item in the next reconciliation pass; only roll forward to V060F if still applicable |
| Release version target shifts (v0.5.1 patch vs v0.6.0 minor) | Medium | Low | File/title rename is cheap; existing V060F prefix stays stable on already-filed items |

## Stats

| Phase                     | Items | Status                                                 |
| ------------------------- | ----- | ------------------------------------------------------ |
| Deferrals (v0.5.0-beta)   | 0     | —                                                      |
| Nominations               | 1     | Complete (V060F-001)                                   |
| As-built sweep follow-ups | 10    | Open (V060F-002..V060F-011, filed 2026-05-07)          |
| **Total**                 | **11** | 1 Complete / 10 Open                                  |

The 10 as-built sweep follow-ups split roughly:

- **CLI gaps** (3): V060F-002 stop, V060F-003 unblock, V060F-005 Windows MCP daemonStatus
- **Cross-platform behaviour** (1): V060F-004 macOS interrupt-ladder PID-reuse branch
- **Activation hand-offs** (2): V060F-006 LAUNCH-016 user-config, V060F-007 watch-liveness probing
- **Doc alignment** (3): V060F-008 kernel-spec status, V060F-009 quality-model dispatch framing, V060F-010 checks ownership
- **Spec/code reconciliation** (1, multi-item): V060F-011 kernel spec divergences
