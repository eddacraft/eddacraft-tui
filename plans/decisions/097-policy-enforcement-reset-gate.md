# Architecture: Policy Enforcement Reset Gate

| Field | Value |
|-------|-------|
| Status | Accepted 2026-07-04 (operator, planning council plan-18c47503) |
| Planning Council | plan-18c47503 |
| Input Brief | [POLRESET-001 ADR Reconciliation Brief](../brainstorms/2026-07-03-polreset-001-adr-reconciliation-brief.md) |
| Date | 2026-07-04 |
| Participants | architect, pragmatic-lead, adversarial-reviewer |

## Problem Statement

POLRESET-001 is the design gate for Anvil's policy-value reset
([`policy-value-enforcement-reset.aps.md`](../modules/policy-value-enforcement-reset.aps.md)).
It must reconcile four standing decisions — ADR-002 (warnings-first), ADR-015
(warn / fence / interrupt intercept enforcement), ADR-037 (witness / L4
acceptance policy), and ADR-040 (regorus engine) — into one accepted record for
user-authored policy admission, validation-before-load, exceptions, the
pre-write boundary, and enforcement routing. The operator's steer is explicit:
the legacy Go OPA components are to be deprecated, not preserved as a parallel
production runtime.

The code reality the gate must resolve (surveyed 2026-07-03, see the input
brief): the regorus facade (`crates/anvil-policy-engine`) is real and used, but
the crate named `anvil-policy` is the legacy OPA-binary-subprocess path; the
default `anvil gate` policy check still routes to the Go OPA subprocess
(`gate.rs:1736`, live via `GATE_INTERNAL_CHECKS`, `check_catalog.rs:234`) with
**no** regorus implementation behind it; two parallel enforcement-mode
vocabularies exist (daemon `Mode` vs MCP `EnforcementMode`); exceptions are
verified but not enforcement-wired (EXCEPT-006 gap); and pack admission on the
regorus path does not exist.

The operator personally ratified every gate question in session
plan-18c47503; the architecture decisions below record those ratifications.

## Constraints

Stated as non-negotiable during the operator interrogation:

- **Full OPA → regorus replacement is the intent.** Zero production users is the
  change window; there is no frozen reference crate to preserve.
- **Deprecation is decoupled** — it does **not** block POLRESET-002..006.
- **Enforcement vocabulary unifies on kernel-types `ControlDecision` now**
  (slice 1), not via a mapping table deferred to a follow-up.
- **A mandatory out-of-band kill switch** must exist: a daemon-level env-var
  override that bypasses `.anvil.yaml`, so recovery from a broken pack never
  routes through the interrupt gate itself.
- **The pre-write eval budget is tight and fail-open** — on timeout, policy eval
  degrades to warn + log and never blocks the write.
- **ADR-002's default posture is upheld** — policy breaches default to `warn`;
  `observe_only` is honoured.
- **The daemon dependency boundary must forbid policy crates** as part of
  POLRESET-001's done-criteria, not as an assumption.

## Architecture Decisions

### AD-1: OPA Deprecation and Replace-Then-Delete Sequencing

- **Context:** `anvil-policy` is the OPA-binary-subprocess legacy crate
  (`opa.rs`, `evaluator.rs`, vestigial `loader.rs`/`bundle.rs`, a hardcoded
  `builtin_policies()` catalogue), while regorus lives in `anvil-policy-engine`.
  The default `anvil gate` policy check (`run_check_policy`, `gate.rs:1736`) is
  the live Go OPA subprocess path with no regorus implementation behind it. Zero
  production users gives a clean change window for a full replacement rather than
  a dual-runtime migration.
- **Decision:** Full OPA → regorus replacement (no frozen reference crate),
  delivered as a **sequenced replace-then-delete** in three PRs:
  - **PR-A (ships with this ADR):** delete
    `crates/anvil-policy/tests/opa_capabilities.rs` and
    `crates/anvil-policy/tests/opa_real_binary.rs` **only**; fix POLRESET-007's
    stale citation (`policies/eval/` does not exist → `policies/fixtures/`).
  - **PR-B (= the existing OPAE-003 Regorus-backed user policy load path):**
    repoint `run_check_policy` (`gate.rs:1736`, live in the default gate flow via
    `GATE_INTERNAL_CHECKS`, `check_catalog.rs:234`, currently the Go OPA
    subprocess path), `anvil policy list` / `explain`, and `builtin_policies()`
    onto the `anvil-policy-engine` facade. Ships as its own PR **gated on
    eval-regression parity evidence** (this is the real engine swap). It **may
    start before** OPAE-002 / POLVAL-004 — this is a sequencing note, not a hard
    dependency.
  - **PR-C:** mechanical deletion of `opa.rs` / `evaluator.rs` / `loader.rs` /
    `bundle.rs` / `library.rs`, and move the `Violation` struct into
    `exceptions.rs`. Gated on PR-B.
  - `opa-executor.ts` is deferred to the JS/TS retirement track.
- **Rationale:** Replace-then-delete keeps the gate flow working at every step
  and lets the risky engine swap (PR-B) carry its own parity gate independently
  of the mechanical cleanup. **What stays in CI:** `opa test policies/fixtures/`,
  regal lint, and the poleng-parity bench remain — these are live ADR-040 D-1/D-5
  infrastructure, not the deprecated runtime. The regorus port also closes a
  live UX gap: the gate policy check is live wiring with dormant config — it
  silently returns `passed:true` when `.anvil/policies/` is absent
  (`gate.rs:1745`) or the opa binary is missing (`OpaNotAvailable`,
  `evaluator.rs:74`); the port removes that silent-skip. EVALCI / eval-regression
  impact is **zero** — `adapter.rs:140-154` shells out to the `anvil` binary,
  never to `opa`.
- **Alternatives Considered:**
  - Keep a frozen OPA reference crate — rejected; zero production users means the
    change window is now and a parallel runtime is the exact "second policy
    runtime" POLRESET exists to prevent.
  - Delete OPA in a single PR — rejected; the engine swap needs its own
    parity-gated PR distinct from the mechanical deletion.
- **Status:** Accepted

### AD-2: Crate Topology End-State

- **Context:** The naming is inverted — `anvil-policy` is the OPA crate and
  `anvil-policy-engine` is the regorus facade — which invites new code onto the
  wrong path. The exceptions store, the eval-regression harness, and the
  `list`/`explain` catalogue all currently live in the OPA crate.
- **Decision:** End-state topology:
  - `anvil-policy-engine` is the product eval path.
  - A **new graph-free crate `anvil-exceptions`** (speaks kernel-types
    `ControlDecision`) is the eventual exceptions home. Its extraction trigger is
    `min(EXCEPT-006 landing, the anvil-policy disposition PR)`; the estimate
    includes decoupling `evaluator::Violation`.
  - The eval-regression harness and the `list`/`explain` catalogue **fold into
    `anvil-cli`**; `list`/`explain` repoints onto the canonical `anvil-checks`
    AP registry, retiring the hardcoded 10-entry `builtin_policies()` mirror.
  - `profiles.rs` and `config_view.rs` (no external users) die with
    `library.rs`.
  - Under the full-replacement posture, `crates/anvil-policy` is ultimately
    deleted once the exceptions extraction completes.
- **Rationale:** Naming and ownership follow the product path so new work cannot
  drift onto the OPA crate; exceptions get a graph-free home that speaks the
  canonical vocabulary rather than a policy-crate-local `Violation` type.
- **Alternatives Considered:**
  - `anvil-kernel-types` as the exceptions home — rejected; would turn the
    minimal type crate into a logic crate.
  - `anvil-config` as the exceptions home — rejected; exceptions are governance
    evidence, not configuration.
- **Status:** Accepted

### AD-3: Enforcement Vocabulary Unification

- **Context:** Two parallel vocabularies parse `.anvil.yaml` `enforcement.mode`
  with different, lossy alias tables — daemon `Mode::{Warn, Fence, Interrupt}`
  (`config.rs:84`) and MCP `EnforcementMode::{Block, Warn, Off}`
  (`mcp/enforcement.rs:47`, where `fence`/`interrupt` collapse to `Block` at
  parse time and `off` exists only MCP-side). The canonical `ControlDecision`
  lives in kernel-types (`diagnostics.rs:56-63`).
- **Decision:** Adopt a **two-axis model** now (slice 1): outcome =
  `ControlDecision`, posture = `enforcement.mode`.
  - `ControlDecision` gains `Fence` → `Allow | Warn | Block | Fence | Interrupt`
    **plus** a `#[serde(other)] Unknown` variant in the **same change** (its
    siblings `Severity`/`Category`/`Mode` already have fallbacks — this makes it
    the last breaking extension of the enum). `Off` stays posture-only (it
    projects to always-`Allow`).
  - Daemon `Mode` and MCP `EnforcementMode` **both die** into **one shared
    posture type + a single alias table**, homed in **kernel-types**
    (`anvil-intercept-proto` was rejected — its `enforcement_config.rs` doc pins
    resolution consumer-side).
  - The lossy `fence → Block` collapse **moves from parse-time to action-time
    projection**: MCP records the true decision and projects
    `Block | Fence | Interrupt → write-veto`; the daemon projects
    `Fence → fence-worktree` and `Interrupt → signal ladder`.
  - Stricter-wins stays on the posture `Ord` (`off < warn < fence < interrupt`).
    Alias: `block → interrupt`. `Unknown → Warn` as the safe default, **plus
    mandatory per-occurrence structured telemetry** (anvil-intercept and anvil
    are separate binaries; version skew is real — reuse the ADR-036 daemon-stale
    precedent).
  - **Three slice-1 amendments that MUST ship in the same change:**
    1. `apply_patch.rs:165` and `validate_write.rs:354` `isError` gating is a
       JSON string comparison (`payload["decision"] == "block"`) — it must become
       an `is_veto()` check, or a fence-vetoed write reports `isError:false` (a
       silent bypass).
    2. Audit **every** `matches!()` over `ControlDecision` (e.g. the
       `ack_required` path, `telemetry.rs:496`, is `matches!`-based and **not**
       compiler-forced), not just `match` statements.
    3. The load-time divergence warning is scoped **up** to per-occurrence
       telemetry.
- **Rationale:** One canonical outcome type and one posture type remove the
  implicit, lossy mapping the two enums encode today. Action-time projection
  keeps the true decision auditable while still honouring the daemon/MCP response
  shapes. `Unknown` + telemetry makes cross-binary version skew observable rather
  than silently mis-handled.
- **Alternatives Considered:**
  - A pinned mapping table between the two enums — rejected by operator steer;
    unify now rather than defer.
  - Home the shared posture type in `anvil-intercept-proto` — rejected; its
    `enforcement_config.rs` doc pins resolution consumer-side.
  - `Unknown(String)` payload variant — the unit `#[serde(other)]` form is
    consistent with the enum's siblings.
- **Deferred:** `RuleMode` normalisation, EXCEPT-006 wiring, and the
  posture → policy-outcome ladder.
- **Status:** Accepted

### AD-4: Pre-Write Boundary and Hot-Path Deferral

- **Context:** Two independent walls keep policy off the save-time path today —
  ADR-061 §6 (`validate_paths` runs the antipattern family only; the response
  labels the claim via `check_families`) and the daemon dependency boundary
  (ADR-064/071 pattern), which currently does not name policy crates, so regorus
  could enter the resident daemon by accident.
- **Decision:**
  - The pre-write boundary is **existing surfaces only**: MCP
    `anvil_validate_write` + `anvil gate` + CI. **No** new tool-call interception
    layer — that needs its own ADR (per POLRESET out-of-scope).
  - **ADR-061 §6 family scoping is upheld** — no policy family on
    `validate_paths` in slice 1. This is an explicit deferral; revisiting it
    requires a bench against the ADR-031 rubric, a new ADR, **and** a
    dep-boundary edit made via the reserved injected-trait pattern.
  - `daemon_dep_boundary.rs` is **extended in the gate PR** to forbid `regorus`
    and `anvil-policy*` on the resident daemon. This is part of POLRESET-001's
    done-criteria.
  - The ADR-067-style **injected-trait pattern is reserved as the only
    sanctioned future on-ramp** for daemon save-time policy.
- **Rationale:** Slice 1 gets policy value on surfaces that already link the
  facade (`anvil-cli`, gate, CI) without paying eval cost on the always-on
  daemon hot path. Encoding the boundary in the dep-boundary test converts a
  silent default into an enforced one.
- **Alternatives Considered:**
  - Add a `policy` family to `validate_paths` now — rejected; widens the work
    ADR-061 exists to keep narrow, absent a bench and a widened claim contract.
  - Admit regorus into `anvil-intercept` directly — rejected; makes the policy
    engine resident in the always-on daemon.
- **Status:** Accepted

### AD-5: Policy Enforcement Routing and Safety Rails

- **Context:** Policy outcomes must reach the enforcement vocabulary without
  reopening ADR-015's shipped rule contract, while ADR-002's warnings-first
  default and a safe recovery path from a broken pack are preserved.
- **Decision:**
  - The `Rule` trait **stays binary `Allow | Interrupt`**; policy routing happens
    at the **adapter** (ADR-015 AD-6 upheld; the four-level model stays
    deferred).
  - **`warn` is the default** (ADR-002 upheld) — policy breaches default to warn;
    `observe_only` is honoured. Stricter-wins applies to policy modes, with the
    escalation visibility documented.
  - **Mandatory out-of-band kill switch:** a daemon-level env-var override
    (`ANVIL_POLICY_ENFORCEMENT=off` style) that **bypasses `.anvil.yaml`**, so
    recovery from a broken pack never routes through the interrupt gate itself.
  - **Tight pre-write eval budget, fail-open:** on timeout, policy eval degrades
    to warn + log and **never** blocks the write. (The 10 s facade timeout is
    CLI-only.)
  - **Warn-mode exception labelling before EXCEPT-006:** findings covered by a
    valid-but-unenforced exception are labelled ("exception exists, not yet
    enforced") via a read-only check.
- **Rationale:** Keeping the rule trait binary avoids reopening a shipped
  contract; the adapter is the correct seam for policy severity. The kill switch
  and fail-open budget ensure enforcement can never trap the operator or block a
  write on engine latency.
- **Alternatives Considered:**
  - Widen the rule trait to a four-level output now — rejected; deferred to v2
    per ADR-015 AD-6.
  - Fail-closed eval budget — rejected; a slow eval must not block a save.
- **Status:** Accepted

### AD-6: Exceptions Prerequisite Ordering

- **Context:** Exception verification (`verify_exception_at`) is implemented, but
  EXCEPT-006 — the wiring that makes enforcement **consult** verdicts — is
  Proposed, and the in-code doc confirms the legacy suppression path ignores
  verdicts. Without it, an expired or unattributed exception could silently
  suppress an interrupt.
- **Decision:** EXCEPT-006 verdict-aware exception wiring is a **hard
  prerequisite** for any `fence` / `interrupt` policy mode. `warn`-mode policy
  value ships before it (via the read-only labelling check in AD-5). The
  exceptions store **stays in `crates/anvil-policy` for slice 1**; the move to
  `anvil-exceptions` is deferred to the AD-2 trigger.
- **Rationale:** Blocking modes must not be reachable while an invalid exception
  can silently suppress a finding; warn-mode carries no such risk and can ship
  first.
- **Alternatives Considered:**
  - Allow blocking modes before verdict-aware wiring — rejected; creates a silent
    suppression bypass.
  - Move the exceptions store in slice 1 — deferred; decoupling `Violation` is
    scoped to the AD-2 trigger.
- **Status:** Accepted

### AD-7: ADR Relationships and Bookkeeping

- **Context:** The gate reconciles four ADRs and depends normatively on ADR-015,
  whose header was still Proposed while every constituent AD was Accepted and
  shipped.
- **Decision:**
  - This ADR **amends nothing** in ADR-002, ADR-037, or ADR-040.
  - **ADR-015 is ratified** (header flipped to Accepted) as bookkeeping in this
    PR; all policy routing semantics live here rather than in a further ADR-015
    amendment.
  - **Two policy surfaces are named:** *acceptance policy* (ADR-037's
    `anvil/policy.yml`, a commit-acceptance vocabulary — its `on_block: reject`
    stays a commit-acceptance verb, not an enforcement mode) versus *code policy*
    (ADR-040's regorus packs). ADR-040-path outcomes route through ADR-015's
    vocabulary.
  - `validate_at_l4`-over-regorus is recorded as **future intake, not scope**.
  - **Validation-before-load** is fail-fast pack admission **before** evaluation;
    the detailed design is delegated to POLVAL / OPAE.
  - **Downstream unblock is minimal, deps-only:** POLRESET-002 / 003 / 004 / 010
    flip to Ready on this ADR landing; the others wait on their own
    prerequisites.
- **Rationale:** Keeps the reconciliation surgical — it consumes the four ADRs
  and closes ADR-061 §6's family-growth question by explicit deferral without
  reopening any of them.
- **Status:** Accepted

## Open Questions

1. **Daemon save-time policy on-ramp timing** — when should the reserved
   ADR-067-style injected-trait hook actually deliver policy eval on the daemon
   hot path? Deferred behind a bench against the ADR-031 rubric, a new ADR, and a
   `daemon_dep_boundary.rs` edit.
2. **Posture → policy-outcome ladder** — the full mapping from posture to graded
   policy outcomes is deferred (AD-3).
3. **`RuleMode` normalisation** — deferred (AD-3).
4. **`anvil-exceptions` extraction timing** — the exact trigger is
   `min(EXCEPT-006 landing, anvil-policy disposition PR)`; the precise ordering
   resolves when whichever lands first (AD-2/AD-6).
5. **`validate_at_l4`-over-regorus** — recorded as a future L4 consumer of the
   engine, not designed now (AD-7).

## Risks

| Risk | Severity | Source | Mitigation |
|------|----------|--------|------------|
| Fence-vetoed write reports `isError:false` (silent bypass) because gating is a JSON string compare (`decision == "block"`) | High | adversarial-reviewer | AD-3 amendment 1: replace with an `is_veto()` check at `apply_patch.rs:165` + `validate_write.rs:354` |
| New `Fence`/`Unknown` arms silently mis-handled by `matches!()` sites that are not compiler-forced (`telemetry.rs:496`) | High | adversarial-reviewer | AD-3 amendment 2: audit every `matches!()` over `ControlDecision`, not just `match` |
| Cross-binary version skew (anvil-intercept vs anvil) mis-handles an unknown decision | Medium | adversarial-reviewer | AD-3: `Unknown → Warn` + mandatory per-occurrence telemetry (ADR-036 precedent) |
| A broken policy pack locks recovery behind the interrupt gate | High | adversarial-reviewer | AD-5: out-of-band `ANVIL_POLICY_ENFORCEMENT` kill switch bypassing `.anvil.yaml` |
| Policy eval latency blocks the write | Medium | pragmatic-lead | AD-5: fail-open budget degrades to warn + log |
| Expired / unattributed exception silently suppresses an interrupt | High | adversarial-reviewer | AD-6: EXCEPT-006 verdict-aware wiring is a hard prerequisite for blocking modes |
| `regorus` / `anvil-policy*` accidentally admitted to the resident daemon | Medium | architect | AD-4: `daemon_dep_boundary.rs` extended to forbid them in the gate PR |
| Stricter-wins silently escalates a contributor into interrupt mode | Medium | adversarial-reviewer | AD-5: `warn` default + documented escalation visibility |
| Dormant gate config silently returns `passed:true` (missing `.anvil/policies/` or opa binary) | Medium | adversarial-reviewer | AD-1: the regorus port removes the silent-skip UX gap |

## References

- Reconciled ADRs: [002](002-warnings-over-blocks.md),
  [015](015-intercept-loop-enforcement.md),
  [037](037-witness-chain-and-l4-policy.md),
  [040](040-rust-policy-engine-regorus.md)
- Adjacent: [061](061-save-time-daemon-delta-validation.md),
  [064](064-intercept-graph-cache-crate-boundary.md),
  [067](067-daemon-symbol-feed-parse-hook.md),
  [071](071-ast-aware-antipattern-detection.md),
  [036](036-daemon-scope-discovery-and-boundaries.md),
  [096](096-diagnostic-severity-category-forward-compat.md)
- Input brief:
  [POLRESET-001 ADR Reconciliation Brief](../brainstorms/2026-07-03-polreset-001-adr-reconciliation-brief.md)
- Modules: [POLRESET](../modules/policy-value-enforcement-reset.aps.md),
  [EXCEPT](../modules/git-native-exceptions.aps.md),
  [POLVAL](../modules/policy-pack-validation.aps.md),
  [OPAE](../modules/opa-enhancements.aps.md),
  [EVALCI](../modules/eval-regression-ci-gate.aps.md)
