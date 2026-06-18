# v0.6.0-beta Release Candidates

| ID    | Owner | Status      |
| ----- | ----- | ----------- |
| V060F | —     | In Progress |

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
ROADMAP.md (the strategic frame) and RELEASE-PLAN.md (release menu)
at cherry-pick time.

Naming: file is `v060` because items here ride the *next* release after
v0.5.0; if the next release tags as v0.5.1 (patch) instead of v0.6.0
(minor), rename the file then — the prefix V060F stays stable on items
already filed, but the module title and file should reflect the actual
target tag once chosen.

See: plans/aps-rules.md
-->


| ID    | Owner | Status      | Progress |
| ----- | ----- | ----------- | -------- |
| V060F | —     | In Progress | 18/25    |

**Last reviewed:** 2026-06-19 (full triage pass + Wave 1 shipped — re-verified
all 21 open items against HEAD; 8 closed as resolved-elsewhere:
V060F-003/005/010/011/012/013/017/022; **Wave 1 (docs & hygiene) completed:
V060F-008/009/014/023/024**; remaining 8 sequenced into Waves 2–4 — see
"Triage 2026-06-19" below. Previously: V060F-021 + V060F-020 completed
2026-05-12; V060F-025 OPA runtime pin 2026-05-08; V060F-002..V060F-024 filed
2026-05-07)
**Predecessor:** [v050-release-followups](./v050-release-followups.aps.md)
**Sequencing context:** [`ROADMAP.md`](../../ROADMAP.md) +
[`RELEASE-PLAN.md`](../../RELEASE-PLAN.md)

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

## Work Items

### V060F-001: admin command parity for `anvil admin` (nomination)

- **Source:** [RCLI2-009](./rust-cli-tier2.aps.md#rcli2-009-admin-command-parity-listshowrevokeauditsend-migrationemail-update)
- **Status:** Done
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
- **Status:** Done — superseded by RCLI3-017b (`anvil intercept unblock
  --worktree` / `--all`), wired in `crates/anvil-cli/src/commands/intercept.rs`
  (`Unblock(UnblockArgs)` → `dispatch_unblock_worktree`) over the existing
  `FenceStore::unblock_worktree` data path. Closed in triage 2026-06-19.

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
- **Status:** Done — `daemonStatus` parity closed on Windows by MLP2-075
  (`query_protection_claim` Windows named-pipe arm in
  `crates/anvil-cli/src/mcp/validation.rs`, via
  `anvil_intercept_win32::pipe_name_for_current_user`). The remaining Windows
  *pre-write* IPC (`validate_pre_write` `cfg(unix)` arm) is intentionally out
  of scope and tracked under the MLP2 / intercept Windows workstream, not
  here. Closed in triage 2026-06-19.

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
- **Status:** Done (Wave 1, 2026-06-19) — `docs/architecture/rust-kernel-spec.md`
  status header reworded to "Spec — H1 design intent" with an authoritative
  cross-link to `kernel-as-built.md`, plus a folded-in V060F-011 spec-vs-code
  reconciliation block covering §5.2/§6.4/§7.2/§8.2/§9.3.

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
- **Status:** Done (Wave 1, 2026-06-19) — `docs/architecture/quality-model.md`
  `anvil watch` section now states the watcher emits change events and check
  evaluation is deferred (intercept daemon / manual `anvil check`), with a
  cross-link to `checks-as-built.md`.

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
- **Status:** Done — `docs/architecture/checks-as-built.md` now records the
  canonical owner split (`scan-performance` SCAN-NNN primary, with SURFENV /
  AI-001 / AIGUARD subsets and archived `command-safety-surfaces`); ownership
  is discoverable without rerunning the discovery, so no consolidated module
  is needed. Closed in triage 2026-06-19.

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
- **Status:** Done — per-item triage complete (2026-06-19): (a) Rust parser
  **built** (`crates/anvil-kernel/src/parser/languages.rs` registers `Rust`
  alongside the tail languages); (e) daemon-mode transport tracked as
  KERN-050..052 → INTD per ADR-030 (INTD Complete). The residual spec-doc
  reconciliation for (b) AST-snapshot-to-disk, (c) `Heartbeat` payload, and
  (d) declarative-DSL-now-in-`anvil-policy` is a single doc edit that rides
  **V060F-008** (same file). No standalone code work remains; item closed.

---

### V060F-012: `auth-as-built.md` completeness gap

- **Surface:** `apps/anvil-api/src/routes/auth-github.ts` (246 lines) and
  `apps/anvil-api/src/db/migrations/005-*.sql` (the
  `idx_audit_log_metadata_email_lower` case-insensitive expression
  index)
- **Flagged by:** api-as-built §"Routes/tables I couldn't classify"
  (2026-05-07)
- **Intent:** The auth as-built doc was the original reference impl
  for the as-built shape, but two surfaces shipped after it were
  never folded in. `auth-github.ts` is a complete GitHub OAuth flow
  (246 lines). Migration 005 adds an audit-log index that's part of
  the auth-system schema but isn't on auth-as-built's schema list.
- **Expected outcome:** Update `docs/architecture/auth-as-built.md`
  to include: (a) a "GitHub OAuth flow" subsection alongside the
  device-code and OTP flows, citing `auth-github.ts`, with the
  endpoint path and trust-boundary semantics; (b) the
  `idx_audit_log_metadata_email_lower` row in the schema list with
  the migration that introduced it. Refresh "Last reviewed" date.
- **Confidence:** high (small targeted doc edit)
- **Status:** Done — `docs/architecture/auth-as-built.md` now carries the
  `### GitHub OAuth Flow` subsection (cites `auth-github.ts`) and lists the
  `idx_audit_log_metadata_email_lower` index in the schema list. Closed in
  triage 2026-06-19.

---

### V060F-013: anvil-observability dead-code purge

- **Surface:** `crates/anvil-observability/src/redaction.rs`
  (`SENSITIVE_FIELDS: &[&str]` 16-entry deny-list and
  `is_sensitive_field()` helper)
- **Flagged by:** observability-as-built §"Known gaps" G-02
  (2026-05-07)
- **Intent:** The deny-list and helper are advisory infrastructure
  with **zero external consumers** in the Rust workspace as of HEAD.
  The installed tracing subscriber does not consult them; the
  `[REDACTED]` strings in `anvil-checks` and the §4.4 redaction
  filter in `validate_write` are local literals, not imports of
  the helper. The crate's own unit tests are the only callers.
- **Expected outcome:** Triage decision — either (a) wire the
  helper into the installed subscriber so the deny-list is actually
  enforced cross-crate, (b) wire it into `anvil-checks` /
  `validate_write` redaction call sites where the deny-list is
  conceptually correct, or (c) delete it. Status quo is the worst
  option because it implies coverage that doesn't exist.
- **Confidence:** medium (depends on triage decision)
- **Status:** Done — no longer dead code: `is_sensitive_field()` is now
  consumed via `redact_arg()` in `crates/anvil-cli/src/usage.rs` (USAGE-001,
  live 2026-06-14), so the deny-list backs a real redaction path. Closed in
  triage 2026-06-19.

---

### V060F-014: namespace registry partial wiring

- **Surface:** `docs/observability/namespace-registry.md` (registry
  document) vs Rust workspace consumption sites
- **Flagged by:** observability-as-built §"Known gaps" G-04
  (2026-05-07)
- **Intent:** The registry document records three rows
  (`anvil.flags.*`, `kindling.*`, `anvil.rtai.*`); only
  `anvil.flags.*` is wired in code (FLAGS module Complete).
  `kindling.*` is out-of-tree (Edda Stack), `anvil.rtai.*` is
  provisional pending RTAI promotion. There is no Rust-side
  registry validation hook — the contract is enforced by
  founder-reviewed PR, not code. New consumers can drift the
  registry silently.
- **Expected outcome:** Either (a) add a build-time validation hook
  that enforces registry membership for tracing-event
  field-prefixes, or (b) update the registry document to mark each
  row's wiring status explicitly so future readers know what's
  enforced and what's advisory. (a) is more work but kills the
  drift class permanently.
- **Confidence:** medium
- **Status:** Done (Wave 1, 2026-06-19) — scope reduced to the doc-side per-row
  wiring markers (option b) and shipped: every row in
  `docs/observability/namespace-registry.md` now opens with a **Wired** /
  **Not yet wired** / **Out-of-tree** marker plus a legend; the registry now
  reflects that `anvil.cli.*`/`anvil.intercept.*` (TRACE-004) and
  `anvil.usage.*` (USAGE-001) are wired, not just `anvil.flags.*`. The
  build-time validation hook (option a) was triaged out as gold-plating —
  defer unless a drift regression appears.

---

### V060F-015: driver framework — eight spec-only JSON-RPC methods

- **Surface:** `plans/specs/anvil-driver-framework/editor-and-mcp-driver-design.md`
  §3.2 (spec) vs `crates/anvil-intercept-proto/src/protocol.rs`
  (proto) and the daemon's request router
- **Flagged by:** driver-framework-as-built §12 reconciliation table
  (2026-05-07)
- **Intent:** The driver design spec names **fourteen**
  `anvil/`-namespaced JSON-RPC methods. The proto crate ships
  **six**. Eight are spec-only with no Rust constant or daemon
  route:
  - `anvil/driver/capabilities/update`
  - `anvil/capability/downgrade` (notification — exists today as a
    `tracing::warn` log only, no JSON-RPC notification)
  - `anvil/enforcement/decision`
  - `anvil/enforcement/refuse` (the constant exists in the TS
    client's enforcement-ack-class timeout list, not in the
    protocol vocabulary)
  - `anvil/suppression/state`
  - `anvil/gate/result`
  - `anvil/nudge/metadata`
  - `anvil/correlation`
- **Expected outcome:** Per-method triage. For each method,
  decide: (a) ship the proto constant + daemon route + driver-side
  consumer; (b) drop from the spec; (c) defer with an explicit
  ADR-style note in the spec. The eight methods aren't equally
  important — `enforcement/decision` and `suppression/state` are
  load-bearing for the editor-driver UX; `nudge/metadata` is more
  decorative. Triage at the method level, not the umbrella level.
- **Confidence:** low (mixed scope; needs per-method scoping)
- **Status:** Open

---

### V060F-016: reliability-budget on-disk persistence

- **Surface:** `crates/anvil-intercept-rules/src/lib.rs`
  (`QUARANTINE_PERSISTENCE_NOTE` documentation block)
- **Flagged by:** driver-framework-as-built §"Known gaps"
  (2026-05-07)
- **Intent:** The reliability-budget logic is in-process only; the
  `QUARANTINE_PERSISTENCE_NOTE` documents the on-disk schema and
  recovery semantics, but no implementation exists. A daemon
  restart loses the quarantine state, which means a poorly-behaved
  driver that triggered quarantine in the previous run is allowed
  back in immediately on restart.
- **Expected outcome:** Implement the persistence layer per the
  documented schema. Driver quarantine state survives daemon
  restart, mirroring the fence-persistence invariant from INTD-005
  for sessions.
- **Confidence:** medium (the schema is documented; it's straight
  implementation against a stable contract)
- **Status:** Open

---

### V060F-017: panic isolation defeated by `panic="abort"` in release builds

- **Surface:** workspace `Cargo.toml` (`panic = "abort"` profile
  setting) vs `crates/anvil-intercept-rules/src/lib.rs`
  (`RuleRegistry`'s `catch_unwind` panic isolation)
- **Flagged by:** driver-framework-as-built §"anvil-intercept-rules"
  (2026-05-07)
- **Intent:** `RuleRegistry` wraps individual rule evaluations in
  `catch_unwind` so a panicking rule can't bring down the daemon's
  hot path. But the workspace release profile uses
  `panic = "abort"`, which makes `catch_unwind` a no-op — a panic
  in any rule aborts the daemon process. Panic isolation is
  effectively debug/test-only.
- **Expected outcome:** Triage decision — either (a) switch the
  release profile to `panic = "unwind"` for the daemon binary
  (workspace-wide change with binary-size implications), (b) split
  the daemon into its own crate with its own profile override, or
  (c) document the limitation in the as-built and architecture
  docs and accept the constraint as a v1 trade-off.
- **Confidence:** medium (the trade-off is real; the choice is
  hard)
- **Status:** Done — resolved by ADR-051: the workspace `[profile.release]`
  is `panic = "unwind"` (`Cargo.toml`), so `RuleRegistry`'s `catch_unwind`
  isolation (`crates/anvil-intercept-rules/src/registry.rs`) holds in release
  as well as debug/test. Closed in triage 2026-06-19.

---

### V060F-018: TuiBackend default flip from Ink to Ratatui

- **Surface:** `crates/anvil-tui/src/shell.rs` (TuiBackend default)
- **Flagged by:** tui-as-built §"Known gaps" G-02 (2026-05-07)
- **Intent:** The TUI surface library supports both the legacy
  Ink-style backend and the new Ratatui backend. RATS module is
  Complete (7/7) and PORT module is Complete (15/15) — the
  Ratatui port has shipped — but the default backend in the shell
  is still Ink. There is no tracked work item to flip the default,
  which means the new Ratatui surfaces are not the default user
  experience even though they're production-ready.
- **Expected outcome:** Flip the default to Ratatui across the
  CLI surfaces. Confirm snapshot-pinned rendering tests still pass
  against the new default. Update `--backend` flag (if present) to
  expose Ink as the explicit fallback.
- **Confidence:** medium (the technical flip is straightforward;
  the user-facing change needs validation across terminals)
- **Status:** Done (Wave 2, 2026-06-19) — **premise was stale**: the
  shipped CLI already renders every surface through
  `anvil_tui::shell::render_shell` → `eddacraft_tui::shell::render_shell`
  (Ratatui). `TuiBackend` / `select_backend` / `TuiApp` have **zero**
  references in `crates/anvil-cli/src` — vestigial migration scaffolding —
  and the `Ink` arm is the retired Node TUI (`archive/anvil-cli-node/`).
  So Ratatui is already the default (and only) production UX; there was no
  live Ink default to flip. Aligned the vestigial stub to match reality
  (`TuiBackend` default Ink → Ratatui, doc + `select_backend` fallback +
  the `default_returns_ratatui` test) so it stops advertising a false Ink
  default. No production/snapshot change (678 anvil-tui tests green; no
  `--backend`/`--tui` selector is wired to the stub, so nothing to
  re-expose).

---

### V060F-019: `apps/admin-cli` retirement + `X-Admin-Actor` attribution drift

- **Surface:** `apps/admin-cli/src/index.ts` (Node CLI, 7
  subcommands) vs `apps/anvil-api/src/middleware/admin-auth.ts:88-108`
  (admin attribution derived from the API key, not the
  `X-Admin-Actor` header the Node CLI still sends)
- **Flagged by:** api-as-built §"apps/admin-cli historical Node
  CLI", G-04 (2026-05-07)
- **Intent:** RCLI2-009 ported all 7 Node CLI subcommands (`list`,
  `show`, `approve`, `invite`, `revoke`, `audit`, `send-migration`)
  to the Rust `anvil admin` surface and added one net-new
  (`email-update`). The Node CLI still ships and is functional, but
  it sends an `X-Admin-Actor` header that the current API ignores
  by design (attribution is now derived from the key itself). The
  audit-log entries from Node-CLI calls record the API-key actor,
  not the operator's intended actor name in the header. This is a
  silent attribution drift.
- **Expected outcome:** Either (a) execute the documented retirement
  plan — archive `apps/admin-cli/` to `archive/admin-cli-node/`
  once the Rust binary is release-grade — and call it done, or
  (b) update the Node CLI to drop the `X-Admin-Actor` header so
  audit logs accurately reflect the API surface. Picking (a) is
  cleaner; (b) is a stopgap.
- **Confidence:** high (the retirement path is documented; the
  attribution drift is a small Node-CLI fix)
- **Status:** Open (scope reduced 2026-06-19) — the `X-Admin-Actor`
  attribution drift is **resolved by design**: `admin-auth.ts` now derives
  the actor from the key material and ignores the header (ADMINCLIH-002).
  Remaining work is purely option (a): **retire** the Node `apps/admin-cli/`
  now that the Rust `anvil admin` surface is release-grade. Hygiene, not a
  correctness fix.

---

### V060F-020: CLI TUI runner panic-safety gap

- **Surface:** `crates/anvil-cli/src/tui.rs` (no `Drop` guard, no
  `panic::set_hook`)
- **Flagged by:** cli-tui-runner-as-built §"Known gaps" G-01
  (2026-05-07)
- **Intent:** Each `run_*` wrapper enables raw mode + alternate
  screen at the top of the function and disables them at the
  bottom in a flat sequence. A panic between those two calls
  skips the cleanup, leaving the user's terminal in raw mode
  with the alternate screen active — the user has to blindly
  type `reset` to recover. Twelve call sites across nine command
  modules are exposed. Probability of panic is low, but blast
  radius is medium and the fix is cheap.
- **Expected outcome:** Introduce a `TerminalGuard` newtype that
  enables raw mode + alternate screen on construction and
  restores both on `Drop`. Install a `panic::set_hook` that
  flushes the terminal restore before printing the panic message
  so the panic backtrace is readable. Apply at every `run_*`
  entry point.
- **Confidence:** high (the fix is a well-known Rust idiom)
- **Status:** Done — `TerminalGuard` (RAII enter / `Drop` /
  explicit `leave`) wired through `run_surface`, `run_tutorial`,
  `run_watch_demo`, `run_watch` in `crates/anvil-cli/src/tui.rs`.
  `setup_terminal` (welcome-hub path) installs the same panic
  hook. The hook chains the previous hook so panic backtraces
  still print, but only after `LeaveAlternateScreen` +
  `disable_raw_mode` have run. Idempotent via `OnceLock` so
  repeated TUI sessions don't stack restore copies.

---

### V060F-021: tutorial legacy-path content drift + invariant coverage gap

- **Surface:** `crates/anvil-tui/src/surfaces/tutorial/paths.rs`
  (`policy_steps` / `architecture_steps` / `drift_steps` /
  `ci_steps` at `:138-306`); test pins `policy_path_steps` etc.
  at `paths.rs:330-396` (count + title only)
- **Flagged by:** tutorial-as-built §"Known gaps" G-05 (2026-05-07)
- **Intent:** The LAUNCH-014 ProtectionLoop default path has
  test-pinned copy invariants
  (`protection_loop_copy_uses_activation_state_vocabulary`,
  `protection_loop_copy_does_not_claim_pre_write_protection`).
  The four legacy paths (Policy / Architecture / Drift / CI)
  carry v0.4-era language: declarative YAML policies, hexagonal
  template catalogs that don't ship in v0.6.0-beta, drift
  commands that run but don't verify output shape, hooks /
  exit-code copy that hasn't been validated against
  v0.6.0-beta. Their tests assert step counts and exact titles
  but never content — a future "you are now protected" line
  could land in `policy_steps` without CI noticing.
- **Expected outcome:** Either (a) refresh the four legacy paths
  to reflect v0.6.0-beta reality and extend the LAUNCH-014
  honesty pins to gate their bodies, or (b) drop the legacy
  paths entirely and route users through the deeper `docs/public/anvil/tutorials/` written guides.
  ProtectionLoop is the canonical first-touch; the legacy paths
  carry their weight only if they remain accurate.
- **Confidence:** high
- **Status:** Done — All four legacy paths (Policy,
  Architecture, Drift, CI) refreshed in `crates/anvil-tui/src/surfaces/tutorial/paths.rs`.
  YAML references changed to Rego; file extensions aligned
  to `.rego`; `anvil architecture show` and `anvil policy test`
  wired into the walk; Honest Vocabulary invariants maintained.
  Tests updated to match refreshed titles and commands.

---

### V060F-022: APS schema drift — public docs document legacy ModuleStatus enum

- **Surface:** `docs/public/aps/schemas/json-schema.md` (public
  docs) vs `packages/aps/src/types/index.ts:95`
  (`ModuleStatusSchema = z.enum(['Proposed', 'Ready', 'In Progress', 'Done', 'Blocked'])`)
- **Flagged by:** adapter-packages-as-built §"Known gaps" G-05
  (2026-05-07)
- **Intent:** The live Zod schema documents the canonical enum
  values (`'Proposed'`, `'Ready'`, `'In Progress'`, `'Done'`,
  `'Blocked'`). The parser includes a normalisation layer that
  silently maps legacy `'Draft' → 'Proposed'` and
  `'Complete' → 'Done'` so existing APS documents keep parsing.
  But the public schema reference doc (`docs/public/aps/schemas/json-schema.md`)
  documents the **legacy** enum (`'Draft' | 'Ready' | 'In Progress' | 'Complete' | 'Blocked'`).
  Agents reading the public docs verbatim produce
  non-canonical APS documents. Parser-tolerated, but the
  drift undermines the public schema as an authoritative
  reference.
- **Expected outcome:** Update `docs/public/aps/schemas/json-schema.md`
  to document the canonical enum, with a one-paragraph note
  on the parser's legacy normalisation for backward
  compatibility. Cross-link the `packages/aps/src/types/index.ts`
  schema as the source of truth.
- **Confidence:** high (small public-doc edit)
- **Status:** Done — `docs/public/aps/schemas/json-schema.md` now documents
  the canonical enum (`Proposed | Ready | In Progress | Done | Blocked`),
  matching `packages/aps/src/types/index.ts`. Closed in triage 2026-06-19.
- **Risk-level:** medium for downstream agent tooling — agents
  typing APS by-hand against the public spec will land
  non-canonical values that work but don't match the live
  schema

---

### V060F-023: small doc + version drift — kindling header, BMAD adapter version

- **Surface:**
  - `packages/kindling-integration/src/observation-contract.ts:4`
    (header comment says "9 observation kinds" — actually 11
    schemas defined in the file)
  - `packages/adapters/src/bmad/format-adapter.ts` vs
    `packages/adapters/README.md` (BMAD adapter version drift:
    `v0.1.2` in code, `v1.0.0` in README)
- **Flagged by:** adapter-packages-as-built §"Known gaps" G-04
  + (a) findings (2026-05-07)
- **Intent:** Two small consistency drifts surfaced during the
  TS-package as-built sweep. Neither is load-bearing in
  isolation; bundled here because each is a one-line fix.
- **Expected outcome:** (a) Update the
  `observation-contract.ts:4` header to `"11 observation kinds"`
  (test files, `CONTRACTS.md`, README, OpenAPI generator all
  already say 11). (b) Pick one of `v0.1.2` (code) or
  `v1.0.0` (README) for the BMAD adapter and align both —
  v6.0.3 + v5 legacy support is the actual feature surface,
  the version-string choice is purely communicative.
- **Confidence:** high (both are one-line edits)
- **Status:** Done (Wave 1, 2026-06-19) — (a) `observation-contract.ts:4`
  header now reads "11 observation kinds"; (b) BMAD version aligned to
  **`0.1.2`** — the code value is test-pinned (`bmad-format-adapter.test.ts:44`),
  so `packages/adapters/README.md` was corrected down to match (README was the
  drift). Both now-stale `adapter-packages-as-built.md` notes (G-03 + G-04) are
  marked resolved in place.

---

### V060F-024: clarify or retire `archive/eddacraft-tui-local/`

- **Surface:** `archive/eddacraft-tui-local/` (pre-publication
  fork of `eddacraft-tui` — diverges from the published
  `0.1.0` crate the workspace now depends on)
- **Flagged by:** widgets-as-built §"Crate resolution"
  (2026-05-07)
- **Intent:** The `eddacraft-tui` crate is now published on
  crates.io at `0.1.0` and consumed via the workspace
  dependency (`Cargo.toml:52` + `Cargo.lock:1167-1176`). The
  local archive at `archive/eddacraft-tui-local/` is a
  pre-publication fork that diverges from the published
  crate (notably, the `editor` widget — 1005 lines, consumed
  by `tutorial/fix.rs` — exists only in the published crate
  and is missing from the archive). Future readers
  investigating the widget vocabulary may land on the archive
  and produce wrong claims.
- **Expected outcome:** Either (a) add a top-level `README.md`
  to `archive/eddacraft-tui-local/` explicitly stating the
  archive is historical-only and pointing at the
  `crates.io` release as the source of truth, or (b) delete
  the archive entirely and rely on `git log` for
  pre-publication history. (a) is safer; (b) is cleaner.
- **Confidence:** high (small archive-management decision)
- **Status:** Done (Wave 1, 2026-06-19) — took option (a): prepended a
  "Historical archive — not the source of truth" banner to
  `archive/eddacraft-tui-local/README.md` pointing at the published
  crates.io `0.1.0` crate and calling out the missing `editor` widget.

---

### V060F-025: bump OPA runtime pin before `v0.6.0-beta`

- **Surface:** `packages/anvil/policy/src/opa-binary-manager.ts`, CI OPA setup
  steps, policy testing docs, real-binary test comments
- **Flagged by:** release-prep dependency review (2026-05-08)
- **Intent:** The repo still pinned OPA `0.60.0` even though policy fixtures use
  `import rego.v1` and the release candidate should not ship with an avoidably
  stale policy runtime.
- **Expected outcome:** Update the canonical OPA runtime pin to the current
  stable release, refresh binary checksums, align CI version verification, keep
  the duplicate-pin guard green, and re-run the OPA policy validation surface.
- **Confidence:** high (runtime bump is isolated to OPA binary management and
  real-binary policy tests)
- **Status:** Done — bumped to OPA `1.16.1`.

---

## Triage 2026-06-19 — disposition + parallel waves

Re-verified all 21 open items against HEAD (five weeks of churn since the
2026-05-07 filing). **8 closed** as resolved-elsewhere; **2 scope-reduced**;
**13 remain**, sequenced into waves below so concurrently-developed items
touch disjoint files (no shared-file collisions, dependency order respected).

### Closed this pass (resolved since filing)

| Item | Resolved by |
| ---- | ----------- |
| V060F-003 | RCLI3-017b — `anvil intercept unblock --worktree/--all` shipped |
| V060F-005 | MLP2-075 — Windows `query_protection_claim` pipe parity (pre-write IPC out of scope, tracked elsewhere) |
| V060F-010 | `checks-as-built.md` now documents the canonical owner split |
| V060F-011 | Rust parser built; daemon transport → INTD/ADR-030; residual spec-doc edit folded into V060F-008 |
| V060F-012 | `auth-as-built.md` GitHub-OAuth + audit-index rows added |
| V060F-013 | `is_sensitive_field()` wired via `redact_arg()` (USAGE-001) |
| V060F-017 | ADR-051 — release profile is `panic = "unwind"` |
| V060F-022 | public APS schema doc now lists the canonical enum |

### Wave 1 — docs & hygiene ✅ COMPLETE (2026-06-19)

Disjoint single-file edits; no code-build risk. All five shipped together.

- **V060F-008** ✅ — `rust-kernel-spec.md` status header reworded + as-built
  cross-link + folded-in V060F-011 §5.2/§6.4/§7.2/§8.2/§9.3 reconciliation
- **V060F-009** ✅ — `quality-model.md` watch→checks deferred-dispatch framing
- **V060F-014** ✅ — `namespace-registry.md` per-row wiring markers + legend
- **V060F-023** ✅ — kindling "11 observation kinds" header (+ G-04 note marked
  resolved); BMAD version aligned to test-pinned `0.1.2` in the README
- **V060F-024** ✅ — `archive/eddacraft-tui-local/README.md` historical banner

### Wave 2 — operator surface & UX correctness (4 items, parallel, disjoint crates)

- **V060F-002** — `anvil intercept stop` subcommand (`anvil-cli` commands)
- **V060F-004** — macOS `current_process_start_time` via `proc_pidinfo`
  (`anvil-intercept` interrupt ladder — removes the macOS fence-telemetry skew)
- **V060F-018** ✅ (2026-06-19) — stale premise: production already renders
  Ratatui via `render_shell`; aligned the vestigial `TuiBackend` stub default
- **V060F-019** — retire Node `apps/admin-cli/` (attribution drift already
  resolved; pure retirement remains)

### Wave 3 — activation seams (2 items, coordinate — shared `anvil-cli` dirs)

Both touch `crates/anvil-cli/src/{commands,activation}/`; assign to one owner
or sequence to avoid collisions on the command modules.

- **V060F-006** — honour `extensions:` opt-in in `check`/`watch`/`audit`
- **V060F-007** — real watch-liveness probe in `start --verify` (today
  `WatchTier::Running` is synthesised, not probed)

### Wave 4 — design-gated / blocked (2 items, NOT RC quick-wins)

- **V060F-015** — eight spec-only driver JSON-RPC methods; needs per-method
  ADR triage (`enforcement/decision` + `suppression/state` are load-bearing,
  the rest decorative). Spike before any code; do **not** treat as one PR.
- **V060F-016** — reliability-budget on-disk persistence; explicitly deferred
  to **DRVR-001** (driver-framework Wave 2). Blocked on that owner.

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Module accumulates aspirational nominations and stops being a real release-window list | High | Medium | Re-run a triage pass at cherry-pick time; demote nominations that no longer fit |
| `v050-release-followups` open items not reconciled before V060F starts collecting | Medium | Low | Add a one-line status for each open V050F item in the next reconciliation pass; only roll forward to V060F if still applicable |
| Release version target shifts (v0.5.1 patch vs v0.6.0 minor) | Medium | Low | File/title rename is cheap; existing V060F prefix stays stable on already-filed items |

## Stats

| Phase                                | Items | Status                                                 |
| ------------------------------------ | ----- | ------------------------------------------------------ |
| Deferrals (v0.5.0-beta)              | 0     | —                                                      |
| Nominations                          | 1     | Complete (V060F-001)                                   |
| As-built sweep follow-ups (batch 1)  | 10    | 6 Done / 4 Open (triage closed 003/005/010/011; Wave 1 closed 008/009) |
| As-built sweep follow-ups (batch 2)  | 8     | 5 Done / 3 Open (triage closed 012/013/017; Wave 1 closed 014; Wave 2 closed 018) |
| As-built sweep follow-ups (batch 3)  | 5     | 5 Done / 0 Open (020/021/022; Wave 1 closed 023/024) |
| OPA runtime refresh                  | 1     | Complete (V060F-025, 2026-05-08)                      |
| **Total**                            | **25** | 18 Done / 7 Open (Wave 2 V060F-018; counts reconcile with sibling Wave 2 PR V060F-002 #2781 at merge) |

Batch 1 (intercept / activation / MCP shim / checks / kernel as-builts) split:

- **CLI gaps** (3): V060F-002 stop, V060F-003 unblock, V060F-005 Windows MCP daemonStatus
- **Cross-platform behaviour** (1): V060F-004 macOS interrupt-ladder PID-reuse branch
- **Activation hand-offs** (2): V060F-006 LAUNCH-016 user-config, V060F-007 watch-liveness probing
- **Doc alignment** (3): V060F-008 kernel-spec status, V060F-009 quality-model dispatch framing, V060F-010 checks ownership
- **Spec/code reconciliation** (1, multi-item): V060F-011 kernel spec divergences

Batch 2 (TUI / driver framework / API / observability as-builts) split:

- **Doc completeness** (1): V060F-012 auth-as-built (auth-github.ts + audit_log index)
- **Dead code / drift** (2): V060F-013 observability deny-list, V060F-014 namespace registry partial wiring
- **Spec/code reconciliation** (1, multi-item): V060F-015 driver framework 8 spec-only JSON-RPC methods
- **Architectural unfinished** (2): V060F-016 reliability-budget persistence, V060F-017 panic isolation defeated by panic=abort
- **Surface defaults** (1): V060F-018 TuiBackend default flip Ink → Ratatui
- **Retirement / attribution** (1): V060F-019 admin-cli retirement + X-Admin-Actor drift

Batch 3 (tutorial / widgets / CLI TUI runner / adapter packages as-builts) split:

- **Architectural / runtime correctness** (1): V060F-020 CLI TUI runner panic-safety gap
- **Test-pin coverage gaps** (1): V060F-021 Complete (refreshed Policy/Architecture/Drift/CI paths)
- **Public-doc spec drift** (1): V060F-022 APS public schema documents legacy ModuleStatus enum
- **Small doc / version drift** (1): V060F-023 kindling header comment + BMAD adapter version
- **Repo hygiene** (1): V060F-024 archive/eddacraft-tui-local/ clarification or retirement
