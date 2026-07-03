# POLRESET-001 — ADR Reconciliation Brief (council input)

**Date:** 2026-07-03
**Prepared for:** the POLRESET-001 design gate ("Policy value and enforcement
design gate", [`policy-value-enforcement-reset.aps.md`](../modules/policy-value-enforcement-reset.aps.md)).
**Status:** Briefing only. This document makes no decisions; it maps what
ADR-002, ADR-015, ADR-037, and ADR-040 currently pin, what the code actually
does today, the conflicts the gate ADR must resolve, and the open decisions
with suggested defaults. Intended as input to a planning council session.

POLRESET-001's expected outcome: a decision record that pins the Rego-first
path, validation before load, exception requirements, the pre-write boundary,
and the `warn` / `fence` / `interrupt` mapping.

---

## 1. What each ADR currently pins

### ADR-002 — Warnings Over Blocks (Accepted)

- Warnings never block by default; exit 0 for warnings, non-zero only for
  errors (schema failures, crashes).
- Enforcement is explicit opt-in (`fail-on-warnings: true` in CI).
- This is the product's default posture; every enforcement mode below is an
  opt-in deviation from it.

### ADR-015 — Intercept Loop Enforcement (header **Proposed**; all ADs Accepted)

- AD-3: project-level `.anvil.yaml` `enforcement` block with
  `mode: warn | fence | interrupt`, `on_ambiguous_ownership` hard-capped at
  fence, `observe_only` dry-run; project + user merge picks the **stricter**
  value.
- AD-6: rules output a **binary allow | interrupt** signal; the enforcement
  pipeline maps interrupt through the configured mode (warn → logged warning,
  fence → worktree fence, interrupt → signal ladder). The full four-level rule
  output (allow/warn/block/interrupt) was explicitly **deferred to v2**.
- AD-4 (amended 2026-05-06, INTD-016): DoS budgets on the daemon wire;
  plaintext-local-only stance.
- Anomaly: the header still says Proposed while every AD is Accepted and the
  daemon shipped. ADR-061's acceptance conditions said ADR-015 would be
  "ratified in parallel" — that ratification never landed.

### ADR-037 — Witness Chain and L4 Policy Framework (Accepted)

- D-5: `anvil/policy.yml` declares **per-branch acceptance rules**
  (`require: l4_or_l3 | l4_only | l3_only`, `on_no_witness`, `on_block`) —
  this is a *commit-acceptance* policy vocabulary, distinct from Rego code
  policy.
- `validate_at_l4` is the server-side revalidation backstop when no valid L3
  witness exists.
- Witness chain (in-tree NDJSON + hash chain) is the portable proof carrier.

### ADR-040 — regorus as the Anvil Policy Engine (Accepted)

- D-1: `crates/anvil-policy-engine` wraps `regorus` behind an Anvil-shaped
  facade; downstream crates depend on the facade, never on `regorus`.
- D-2 non-negotiables: closed `PolicyInput` data document (no file reach-out at
  eval time); explicit, audited, deterministic builtins (no clock / net / fs);
  coverage + trace first-class on every eval.
- D-5 explicitly deferred: tier model (OOB / YAML / Rego), OOB rule catalogue,
  bundle distribution/signing, watch-mode performance budget.
- Consequence noted at acceptance: the 10× headroom gives "watch-mode and
  intercept-loop (ADR-015) breathing room" — an aspiration, not a decision to
  put the engine on the hot path.

### Adjacent, load-bearing but not named by the gate

- **ADR-061 §6** (Accepted): the save-time `validate_paths` hot path runs
  `run_antipattern_check` **only**; the response labels the claim via
  `check_families: ["antipattern"]`. Structural policy checks stay on
  `anvil gate`. Doctrine: "narrow the *claim* rather than widen the *work*" —
  forcing more work onto the hot path reintroduces the CPU regression ADR-061
  exists to remove.
- **ADR-064 / ADR-071**: the resident daemon's dependency boundary is guarded
  by `crates/anvil-intercept/tests/daemon_dep_boundary.rs` (no `tree-sitter`,
  no `anvil-checks-ast`). The guard mechanism exists and is proven, but does
  **not** yet name `regorus` / `anvil-policy`.
- **ADR-042**: closeout/exit-code enforcement semantics (relevant to gate #3,
  CI blocking posture — out of scope for this brief).

---

## 2. Code reality (surveyed 2026-07-03)

| Area | Reality | Key citations |
| ---- | ------- | ------------- |
| regorus facade | Real and used: `regorus = "=0.10.1"`, full `Engine` with determinism fence (impure builtins dropped, `rand.intn` shadowed), panic-guard poisoning, 10 s eval timeout | `crates/anvil-policy-engine/src/lib.rs:159`, `:180`, `:236`; workspace `Cargo.toml:96` |
| `anvil-policy` crate | **OPA-binary-subprocess** legacy, not regorus: `OpaExecutor` shells out; `loader.rs` (`.anvil/policies` discovery) and `bundle.rs` are vestigial (test-only callers); hardcoded `builtin_policies()` catalogue; plus the eval-regression harness | `crates/anvil-policy/src/opa.rs:2`, `evaluator.rs:7`, `loader.rs:41`, `library.rs:3` |
| CLI surface | `anvil policy eval` routes through regorus (single `.rego` file + JSON input); `eval-regression` uses the subprocess-parity harness; `list`/`explain` read the hardcoded catalogue only | `crates/anvil-cli/src/commands/policy/eval.rs:17`, `eval_regression.rs:149`, `mod.rs:136` |
| User pack admission | Does not exist on the regorus path. No discovery, no manifest validation, no install UX; facade doc marks path-based discovery "provisional… lands with POLENG-007" | `anvil-policy-engine/src/lib.rs` (`add_policy` doc) |
| Rule contract | Still binary: `RuleDecision::{Allow, Interrupt}`; doc states severity-aware decisions are layered at the daemon's enforcement-mode adapter (INTD-008), not in the rule trait | `crates/anvil-intercept-rules/src/lib.rs:113` |
| Enforcement mode enums | **Two parallel vocabularies**: daemon `Mode::{Warn, Fence, Interrupt}` (stricter-wins, alias table) vs MCP `EnforcementMode::{Block, Warn, Off}` (`interrupt`/`fence` collapse to Block); canonical `ControlDecision` lives in kernel-types | `crates/anvil-intercept/src/config.rs:84`, `:108`, `:135`; `crates/anvil-cli/src/mcp/enforcement.rs:47`; `crates/anvil-kernel-types/src/diagnostics.rs:60` |
| Policy on the daemon path | Absent by construction: `anvil-intercept` has no dep on `anvil-policy`, `anvil-policy-engine`, `regorus`, or `anvil-l4`; no "policy" rule type exists (rule set: secret, path_deny, regex_content, antipattern, reasoning) | `crates/anvil-intercept/Cargo.toml`; `crates/anvil-intercept/src/enforcement.rs:99` |
| Dep-boundary guard | `daemon_dep_boundary.rs` asserts no `tree-sitter` / `anvil-checks-ast` on the daemon; forbidden list does not include policy crates | `crates/anvil-intercept/tests/daemon_dep_boundary.rs:31` |
| Pre-write surface | MCP `anvil_validate_write` → `validate_pre_write` → daemon `scan_buffer`; daemon `validate_paths` runs the antipattern family over openat2-guarded bytes. Nothing policy-shaped runs at pre-write | `crates/anvil-cli/src/mcp/tools/validate_write.rs:21`, `mcp/validation.rs:37`; `crates/anvil-intercept/src/validate_paths.rs` |
| Exceptions | `PolicyException` store + `verify_exception_at` fully implemented (precedence Revoked > Expired > InvalidScope > Unattributed > Active; expiry, scope glob, attribution). **Enforcement wiring is not**: the legacy `is_suppressed_at` path does not consult verdicts (EXCEPT-006 gap, stated in-code) | `crates/anvil-policy/src/exceptions.rs:687`, `:658`, `:557`; consumer `crates/anvil-capsule/src/verify.rs:312` |
| L4 | Policy schema, branch-rule resolver, and the ADR-037 §D-5 decision matrix are real; the `validate_at_l4` execution engine is a **no-op** (`NoOpValidationEngine` → `NotImplemented`; a `BinaryMissing` doc reserves a future regorus case). No exceptions module in `anvil-l4` | `crates/anvil-l4/src/policy.rs:13`, `decide.rs:60`, `validate.rs:105` |
| Witness chain | Shipped: hash chain, flock-serialised writer, rollover, DAG-aware verify; the one protection-stack crate the daemon links | `crates/anvil-witness/src/lib.rs:9`; `anvil-intercept/Cargo.toml:55` |

Coordinated-module status snapshot (2026-07-03): EXCEPT In Progress (001–003
Done, 007 shipped v0.8.0-beta, **005 In Progress, 006 Proposed**); POLVAL
Draft; OPAE Draft (reset 2026-07-02); CPACKS Draft 0/8; CPOL Ready; IORISK
Ready; EVALCI In Progress (001–004 landed, 005–008 Proposed).

---

## 3. Conflicts the gate ADR must resolve

### C1 — Default posture: ADR-002 vs enforcement routing

A policy breach must default to **warn** (ADR-002) with `fence` / `interrupt`
as explicit opt-in — POLRESET-006 already states this. But ADR-015's
"stricter-wins" project + user merge means a project can silently escalate a
contributor into interrupt mode. The gate ADR must say whether stricter-wins
applies to *policy* outcomes as-is, and how `observe_only` interacts with
policy packs (a dry-run rollout path is exactly what ADR-002's adoption logic
demands).

### C2 — Rule contract: binary allow|interrupt vs graded policy outcomes

ADR-015 AD-6 pins rules to binary output, with grading at the adapter; the
four-level model was deferred to v2. A Rego policy naturally expresses
severity/outcome per finding. Options: (a) keep the rule trait binary and do
all policy routing at the adapter layer (consistent with INTD-008; policy
severity becomes adapter input, not a rule output); (b) declare this the v2
trigger and widen the rule contract. Option (a) avoids reopening a shipped
contract; the gate ADR should pick explicitly rather than drift.

### C3 — Two enforcement-mode vocabularies

Daemon `Mode::{Warn, Fence, Interrupt}` and MCP
`EnforcementMode::{Block, Warn, Off}` both parse `.anvil.yaml`
`enforcement.mode` with different alias tables (`fence` collapses to Block on
the MCP side; `off` exists only on the MCP side). Routing policy outcomes
"through the existing intercept vocabulary" (POLRESET) requires either one
canonical enum (kernel-types `ControlDecision` is the natural home) or a
pinned, tested mapping table. Today the mapping is implicit and lossy.

### C4 — Hot-path boundary: where does regorus run?

Two independent walls currently keep policy off the save-time path:

1. **ADR-061 §6** — `validate_paths` runs the antipattern family only;
   `check_families` labels the claim. Adding a `policy` family widens the work
   the ADR refused to widen, unless policy eval is provably bounded (regorus
   eval over a closed input document, per-save, with the 10 s facade timeout
   replaced by a hot-path budget) **and** the family label + `certified`
   semantics are extended.
2. **Dep boundary (ADR-064/071 pattern)** — putting `regorus` into
   `anvil-intercept` makes the policy engine resident in the always-on daemon.
   The precedent for avoiding this is ADR-067's dependency-inverted
   `SymbolParser` hook: the daemon defines a trait, the binary injects the
   implementation, and the crate boundary stays clean.

Options: (a) first slice runs policy **off the daemon**: MCP pre-write
(`anvil-cli`, which already links the facade), `anvil gate`, and CI — daemon
save-time policy deferred behind a bench + a widened `daemon_dep_boundary`
decision; (b) inject policy evaluation into the daemon via an ADR-067-style
hook, keeping the crate graph clean but still paying eval cost on the hot
path; (c) admit regorus into the daemon directly. Whatever is chosen,
`daemon_dep_boundary.rs` should be extended to *encode* the decision (today it
is silent on policy crates, so option (c) could happen by accident).

### C5 — Two things called "policy"

ADR-037's `anvil/policy.yml` (branch-level commit acceptance) and ADR-040's
Rego packs (code-level policy) are different vocabularies with colliding
names, plus the `.anvil.yaml` `enforcement` block as a third config surface.
The gate ADR should name the surfaces (e.g. *acceptance policy* vs *code
policy*) and pin which one owns "policy breach → warn/fence/interrupt"
(ADR-040-path outcomes route through ADR-015's vocabulary; ADR-037's
`on_block: reject` stays a commit-acceptance verb, not an enforcement mode).
Note the latent join: `anvil-l4`'s no-op `validate_at_l4` engine already
reserves a regorus-shaped future (`EngineUnavailableReason::BinaryMissing`
doc) — worth acknowledging as a later consumer, not designing now.

### C6 — Exceptions before blocking

POLRESET-005 requires EXCEPT-005/006/007 before any fence/interrupt policy
mode. Reality: verification (`verify_exception_at`) is implemented, but
EXCEPT-006 — the wiring that makes enforcement *consult* verdicts — is
Proposed, and the in-code doc confirms the legacy suppression path ignores
verdicts. The gate ADR must make verdict-aware exception wiring a **hard
prerequisite** for blocking policy modes (otherwise an expired or unattributed
exception silently suppresses an interrupt). Also: POLRESET-005's validation
cites `cargo test -p eddacraft-anvil-l4 -- exceptions`, but `anvil-l4` has no
exceptions module or tests — the plan's validation commands need reconciling
with wherever EXCEPT-006 actually lands.

### C7 — Validation-before-load is undesigned

ADR-040 D-2 gives eval-time guarantees (closed input, determinism fence), but
pack *admission* — discovery, manifest shape, size caps, compile check,
determinism lint, failure UX — has no design (facade doc: provisional pending
POLENG-007; the only existing loader is the vestigial OPA-era
`.anvil/policies` scan). The gate ADR must pin at minimum: where packs live,
what "valid" means, and that validation failure is fail-fast **before**
evaluation (the POLRESET product outcome), with everything else delegated to
POLVAL/OPAE work items.

### C8 — "No second policy runtime" needs a structural expression

POLRESET's out-of-scope says Go OPA stays reference/parity tooling. But the
crate named `anvil-policy` *is* the OPA-subprocess crate, while the regorus
engine lives in `anvil-policy-engine` — inverted naming that invites new code
onto the wrong path (the eval-regression harness legitimately uses the parity
path; `loader.rs`/`bundle.rs` are dead weight). The gate ADR should
disposition the split: which crate is the product path, what is explicitly
reference-only, and whether the vestigial loader/bundle code is deleted or
retargeted.

### C9 — ADR-015's header status

The gate ADR depends normatively on ADR-015, which is still header-Proposed.
Ratify ADR-015 (flip to Accepted, sync the DECISION-LOG row — currently
"Proposed" at row 81) either before or in the same PR as the gate ADR, so the
new record does not build on a formally unaccepted dependency.

---

## 4. Open decisions for the council (with suggested defaults)

| # | Decision | Suggested default |
| - | -------- | ----------------- |
| D1 | Pre-write boundary (gate #2): does the first slice reuse existing surfaces? | **Yes** — MCP `anvil_validate_write` + `anvil gate` + CI only; no new tool-call interception layer (that needs its own ADR per POLRESET out-of-scope) |
| D2 | Where regorus runs in slice 1 | Off-daemon (C4 option a). Daemon save-time policy deferred behind a bench against the ADR-031 rubric and an explicit dep-boundary decision; extend `daemon_dep_boundary.rs` now to forbid `regorus`/`anvil-policy*` so the default is enforced, not assumed |
| D3 | Routing vocabulary | Rule trait stays binary (C2 option a); policy outcomes map to kernel-types `ControlDecision` at the adapter; pin one documented mapping between daemon `Mode` and MCP `EnforcementMode` (or unify the enums as a follow-up work item) |
| D4 | Exceptions ordering | EXCEPT-006 verdict-aware wiring is a hard prerequisite for `fence`/`interrupt` policy modes; `warn`-mode policy value can ship before it |
| D5 | ADR-015 disposition | Flip to Accepted as-is (C9); put all policy routing semantics in the new gate ADR rather than amending ADR-015 again |
| D6 | Default policy posture | `warn`, per ADR-002; `observe_only` honoured for policy packs; decide whether stricter-wins applies to policy mode (default: yes, consistent with AD-3, but document the escalation visibility) |
| D7 | ADR-037 relationship | Name the two policy surfaces; ADR-037 untouched in slice 1; record `validate_at_l4`-over-regorus as future intake, not scope |
| D8 | Crate disposition | `anvil-policy-engine` is the product path; `anvil-policy` is reference/parity + the exceptions store (or exceptions move); delete or quarantine the vestigial OPA loader/bundle code |

Gate #3 (CI blocking posture for EVALCI-008) is a separate ADR per the module
and is deliberately not covered here.

---

## 5. Suggested shape of the gate ADR

A single ADR (next number per `pnpm adr:check` at time of writing: 097) with
decision sections mirroring D1–D8, in the ADR-015 multi-decision style
(per-AD status), so partial acceptance is possible. It amends nothing in
ADR-002/037/040; it consumes ADR-015's vocabulary and closes ADR-061 §6's
question of family growth by explicit deferral. Companion bookkeeping in the
same PR: ADR-015 → Accepted, DECISION-LOG rows for both, and POLRESET-005's
validation-command fix.

## References

- ADRs: [002](../decisions/002-warnings-over-blocks.md),
  [015](../decisions/015-intercept-loop-enforcement.md),
  [037](../decisions/037-witness-chain-and-l4-policy.md),
  [040](../decisions/040-rust-policy-engine-regorus.md),
  [061](../decisions/061-save-time-daemon-delta-validation.md),
  [064](../decisions/064-intercept-graph-cache-crate-boundary.md),
  [067](../decisions/067-daemon-symbol-feed-parse-hook.md),
  [071](../decisions/071-ast-aware-antipattern-detection.md)
- Modules: [POLRESET](../modules/policy-value-enforcement-reset.aps.md),
  [EXCEPT](../modules/git-native-exceptions.aps.md),
  [POLVAL](../modules/policy-pack-validation.aps.md),
  [OPAE](../modules/opa-enhancements.aps.md),
  [EVALCI](../modules/eval-regression-ci-gate.aps.md)
