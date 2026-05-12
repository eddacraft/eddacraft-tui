# Anvil Release Plan

| Type         | Authority | Owner       | Status | Freshness                                                                                      |
| ------------ | --------- | ----------- | ------ | ---------------------------------------------------------------------------------------------- |
| Release plan | Derived   | APS modules | Live   | Last reviewed 2026-05-13 (Wave 0 Promote Contracts complete); base `v0.6.2-beta` + APS modules |

| Upstream                                                                                                            | Downstream                                                        |
| ------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| [`plans/index.aps.md`](./plans/index.aps.md), `git tag v0.6.2-beta`, [`ROADMAP.md`](./ROADMAP.md), MLP/INTL modules | Release runbooks, PR planning, [`ROADMAP.md`](./ROADMAP.md) links |

**Last updated:** 2026-05-13 (Wave 1 _Build The Load-Bearing Backbone_ started;
MLP-001 reconciled Done against shipped `activation/identity.rs` (22 tests
green; v1 scope narrowed per module footnotes); MLP module advanced to In
Progress with 1/17 done. Next candidate remains the daemon-working `v0.7.0-beta`
slate.)

> Companion: [ROADMAP.md](./ROADMAP.md) for thematic horizons. Execution source
> of truth: [`plans/index.aps.md`](./plans/index.aps.md) and the linked APS
> modules. This file selects the release slate and shows what can run in
> parallel; it does not duplicate every APS work item.

---

## Current State

**Latest tag in repo:** `v0.6.2-beta`

`v0.6.2-beta` is now the shipped operational-substrate release. It closed the
release-operating-model window by landing the main-first branch model, targeted
CI/readiness checks, and deterministic release commands on `main`.

| Area                              | Status  | Evidence                                                                                                                                       |
| --------------------------------- | ------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `OPMODEL` main-first cutover      | Shipped | 12/12, archived 2026-05-11. Cutover SHA `b6f236e9`; `main` ruleset 16217152; `dev-retired-2026-05-11` tag.                                     |
| `RELORCH` release orchestration   | Shipped | 12/12, archived. Deterministic `assess`, `preflight`, `prepare`, `promote`, `tag`, `monitor`, `verify`, and `closeout` command surface exists. |
| `CICD` targeting + drift controls | Shipped | 12/12, archived 2026-05-12. Fast PR targeting, integration SHA readiness, workflow contract map, and APS/repo/release drift checks are live.   |
| Release tag                       | Shipped | `HEAD` is tagged `v0.6.2-beta`; changelog preparation entries exist in `CHANGELOG.md` and public release notes.                                |

The next release should be a **product-surface** release, not another operating
model release. Its claim moves from "the operating model is executable" to
"Anvil protects this project end-to-end through the daemon, hooks, witness
chain, baseline, and wrapped agent launch surfaces."

---

<a id="next-release-window-proposed--post-v060-beta-daemon-working-slate"></a>

## Next Release Window — Daemon-Working Slate

**Candidate tag:** `v0.7.0-beta`

**Claim:** _Daemon working end-to-end._ `anvil start` lands a real, testable
protection claim; hooks fire deterministically; the witness chain records every
commit; baseline adoption works; and `anvil-run` wraps agent processes.

**Primary APS modules:**

| Pick | Module                                                | Status      | Progress | Role                                                                                                                                                                                                                                                      |
| ---- | ----------------------------------------------------- | ----------- | -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| N1   | [`MLP`](./plans/modules/multilayer-protection.aps.md) | In Progress | 1/17     | Multi-layer protection backbone: project identity, witness chain, hooks, L4 policy, baseline, audit. MLP-001 Done 2026-05-13 (v1-narrowed scope; see module footnotes).                                                                                   |
| N2   | [`INTL`](./plans/modules/intercept-launcher.aps.md)   | Ready       | 0/9      | `anvil-run` launcher, session registration, process-group control, shell wrappers, side-channel register.                                                                                                                                                 |
| N3   | Carry-forward gates                                   | Confirmed   | 6/6      | ADR-036..039 Accepted (2026-05-13); project-id, noise-discipline policy, AIGUARD envelope, INTR-004 promoted, DRVR forward-compat — all hold. G5 closed 2026-05-13 when INTR-004 (path-deny rule) was promoted Draft → Ready in `intercept-rules.aps.md`. |
| N4   | Documentation lanes                                   | —           | 0/6      | Adoption, air-gap, witness-chain, hooks-integration runbooks, migration note, INTL manpage. Owner: @aneki (lands in Wave 4). Status column is `—` because these are not APS modules — see Wave 0 outcome row for the actual ownership scope.              |

**Hard release gate:** `MLP-009`. The protection-claim contract suite, air-gap
guarantee, and noise-discipline tests must be green before the release can claim
"Anvil protects this project." No partial slice should be marketed as full
protection.

---

## Parallel Delivery Shape

<a id="required-prerequisites-cross-cutting-glue"></a>

### Wave 0: Promote Contracts

**Status:** Complete (2026-05-13). These ran before broad implementation. Each
item removed ambiguity from the release claim so later lanes don't diverge.
Outcomes recorded inline.

| Work                                  | Parallel? | Outcome (2026-05-13)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| ------------------------------------- | --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| MLP readiness review                  | First     | **Done.** MLP-009 confirmed as the hard release gate (module §17–22, recommended landing order §17). MLP promoted **Proposed → Ready** in [`multilayer-protection.aps.md`](./plans/modules/multilayer-protection.aps.md) and `plans/index.aps.md`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| INTL readiness review                 | First     | **Done.** `AgentTag` stub landed in `crates/anvil-intercept-proto/src/session.rs` (struct + `ANVIL_AGENT_TAG_ENV` / `ANVIL_TASK_ID_ENV` constants + 3 tests, all green via `cargo test -p eddacraft-anvil-intercept-proto`). INTL-003 / INTL-004 reference the real type; planning text now has a backing type definition. INTL promoted **Draft → Ready** at module level (`Draft` normalises to `Proposed` per `plans/aps-rules.md`, so the canonical lifecycle is `Proposed → Ready`). Module-level Ready means ready-to-start-Wave-3; INTL-003 / INTL-004 promoted to task-Ready, the other seven INTL tasks remain Draft pending their direct prerequisites.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| Carry-forward gate reconciliation     | First     | **6/6 confirmed.** G1 ADR-036/-037/-038/-039 promoted Proposed → **Accepted (2026-05-13)**; `DECISION-LOG.md` updated; `pnpm adr:check` green (`43 ADR files; 43 indexed; no duplicates, no orphans`). G2 `anvil/project-id` schema reaffirmed (ADR-036 §D-2 + MLP-001) — no code yet, but the schema is pinned. G3 **policy** confirmed — ADR-038 codifies the Serena rule + hook surface table; a **behavioural** audit is deferred to Wave 2 when MLP-003 ships shippable hook output to audit. G4 AIGUARD envelope re-run: `cargo test -p eddacraft-anvil-kernel-types` green; the `diagnostic_schema_version_constant_matches_spec` test pins `anvil.diagnostic.v1`, which ADR-037 §D-1 reuses inside the witness line envelope. G5 **closed 2026-05-13:** INTR-004 (path-deny rule) promoted **Draft → Ready** in `intercept-rules.aps.md`. G6 DRVR forward-compat: `crates/anvil-intercept-proto/src/protocol.rs` already owns the editor-driver method names (DRVR-002 / DRVR-008); the new `session.rs` lives in the same crate without touching the existing `IpcCommand` / `IpcEnvelope` types — compatibility confirmed by full proto suite (`cargo test -p eddacraft-anvil-intercept-proto`, 28 passed, 0 failed). |
| Release-doc/runbook ownership refresh | First     | **Done.** All 16 files in `docs/runbooks/` enumerated. Not changed by this slate (general operations; no MLP/INTL surface): `admin-cli`, `branch-reconciliation`, `db-migrations`, `emergency-hotfix`, `intd-012-windows-evidence`, `main-first-cutover`, `neon-db-operations`, `observability-triage`, `post-deploy-smoke-check`, `release-token-scope`, `rollback-bad-candidate-artefact`, `rollback-bad-main`, `rollback-bad-published-release`, `v0.6.0-beta-release-runbook`, `v0.6.0-beta-security-note`, `waitlist-email-operations`. **Net-new for `v0.7.0-beta` (six N4 lanes, owner @aneki, deliver in Wave 4):** adoption, air-gap, witness-chain operator, hooks-integration, `v0.6.x → v0.7.0-beta` migration note, `anvil-run` (INTL) manpage. **One additional runbook required before tag:** `v0.7.0-beta-release-runbook.md` (cut from the `v0.6.0-beta` template). `intd-012-windows-evidence.md` flagged for re-read when MLP-014 lands (multi-agent Windows scope).                                                                                                                                                                                                                                         |

**Wave 0 follow-ups (next-window scope, not blocking Wave 1):** (1) ADR-039
`@anvil-ignore` hardening — forbid wildcards and same-diff
suppress-on-introduction for hard-pinned classes — filed against MLP-013. (2)
Formal council session for ADR-037 before the MLP-002 spike (recommended; the
witness-chain primitive is the single load-bearing point of failure).

### Wave 1: Build The Load-Bearing Backbone

These items should stay small and reviewable. `MLP-002` is the single point of
failure for most downstream work.

| Work                                    | Parallel? | Status            | Notes                                                                                                                                                                                                                                                                                                                                                                                                             |
| --------------------------------------- | --------- | ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `MLP-001` project identity              | First     | Done (2026-05-13) | Establishes `anvil/project-id`; unblocks witness, baseline, and policy. Shipped via `activation/identity.rs` (22 tests green). v1 scope: idempotent UUID v7 write at `anvil start`; concurrent-rename convergence; symlink refusal. Deferred to follow-ups (see MLP-001 footnotes): composite-identity attach-check (with MLP-014), `--new-identity` flag (with MLP-007), `anvil baseline` integration (MLP-007). |
| `MLP-002` witness chain                 | After 001 | (separate PR)     | Witness-chain spike tracked on `feat/mlp-002-witness-chain`.                                                                                                                                                                                                                                                                                                                                                      |
| `MLP-011` multi-format config           | Parallel  | Done (2026-05-13) | New `crates/anvil-config/` library: extension-based dispatch into `serde_json::Value`, canonical-JSON serialisation, detection precedence yaml > yml > json > toml. 44 tests green; cross-format equivalence pinned. CLI flag wiring deferred (see footnote 1).                                                                                                                                                  |
| `MLP-013` hard-pinned rule classes      | Parallel  | Done (2026-05-13) | New `validation` module in `crates/anvil-config/`: rejects five disable-attempt shapes for `secrets` / `command-safety` (canonical + legacy locations + mode-disabled); tuning passes through; error messages cite ADR-039 and the `@anvil-ignore` bypass. 19 new tests green; 5 cross-format hard-pinned integration tests. `anvil-checks` rule-registration mirror deferred (see footnote 1).                  |
| `MLP-017` air-gapped guarantee scaffold | Parallel  | Pending           | Build the test harness early; it is part of the release gate.                                                                                                                                                                                                                                                                                                                                                     |

### Wave 2: Hook, Policy, And Baseline Surfaces

Start once the witness primitive is stable enough for dependent lanes to write
against it.

| Work                                          | Parallel? | Notes                                                               |
| --------------------------------------------- | --------- | ------------------------------------------------------------------- |
| `MLP-003` pre-commit hook                     | After 002 | First noisy surface; ADR-038 noise discipline applies from day one. |
| `MLP-005` post-commit/post-merge/post-rewrite | After 002 | Records commit-time state and handles merges/rebases/amends.        |
| `MLP-006` L4 policy framework                 | After 002 | Provides per-branch fallback policy for unwitnessed commits.        |
| `MLP-007` baseline command                    | After 001 | Enables adoption in existing repos without a warning storm.         |
| `MLP-008` hook bootstrap recovery             | After 003 | Recovery UX depends on the first hook implementation.               |
| `MLP-012` `rules_sha` in witnesses            | After 002 | Locks rule-version evidence into the witness model.                 |

### Wave 3: Coordination And Launcher Ingress

These lanes turn protection from a git-only mechanism into an agent-aware
runtime loop.

| Work                                        | Parallel?      | Notes                                                                            |
| ------------------------------------------- | -------------- | -------------------------------------------------------------------------------- |
| `MLP-014` multi-session + task fences       | With INTL      | Coordinates directly with INTL session registration and `AgentTag` schema.       |
| `INTL-001` launcher scaffold                | First INTL     | Creates `crates/anvil-run/` and workspace wiring.                                |
| `INTL-002` daemon connectivity/fence check  | After INTL-001 | Refuses launch when daemon is unreachable or worktree is fenced.                 |
| `INTL-003` session registration             | After INTL-002 | Registers tool/worktree/cwd/tmux context before spawn.                           |
| `INTL-004` process-group launch             | After INTL-003 | Uses Unix PGIDs or Windows named Job Objects so daemon can target interruptions. |
| `INTL-005` cleanup and `INTL-009` heartbeat | After INTL-004 | Keeps daemon state accurate and reapable.                                        |
| `INTL-006` shell wrappers                   | After INTL-001 | Adds zsh/bash integrations for common tool commands.                             |
| `INTL-007` side-channel registration        | After INTL-003 | Supports sessions not launched through `anvil-run`, with downgraded enforcement. |
| `INTL-008` blocked-launch UX                | After INTL-002 | Makes refusal states actionable.                                                 |

### Wave 4: Release-Gate Closure

Do not tag until these are green and reflected in release evidence.

| Work                                  | Parallel?  | Notes                                                                                 |
| ------------------------------------- | ---------- | ------------------------------------------------------------------------------------- |
| `MLP-004` pre-push hook               | After 006  | Walks pushed ranges, verifies witnesses, and applies L4 fallback.                     |
| `MLP-009` protection-claim contract   | Gate       | Closed-set claim states must be reachable in fixtures and rendered honestly.          |
| `MLP-010` GitHub Action publishing    | After 004  | Marketplace action exposes the L4/CI surface.                                         |
| `MLP-015` L5 audit                    | After 006  | On-demand and nightly drift detection for bypassed layers.                            |
| `MLP-016` L1 editor driver → Kindling | After DRVR | Emits mid-edit observations into the protection evidence stream.                      |
| Documentation lanes                   | Gate       | Adoption, air-gap, witness-chain, hooks-integration, migration, and `anvil-run` docs. |

---

## Cut Criteria For `v0.7.0-beta`

`v0.7.0-beta` is cuttable only when the release evidence supports the public
claim without caveats.

| Criterion                 | Required Evidence                                                                                                    |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| Protection claim          | `MLP-009` green: every allowed state is reachable in fixtures and rendered honestly.                                 |
| Save-time and hook layers | Pre-commit, pre-push, post-commit/post-merge/post-rewrite hooks run with ADR-038 noise discipline.                   |
| Witness integrity         | Concurrent writes, rollover, tamper detection, DAG verification, and worktree survival tests pass.                   |
| Baseline adoption         | Existing repositories can adopt without broad warning noise; hard-pinned classes remain enforced.                    |
| Agent launch path         | `anvil-run` registers sessions, isolates process groups / Job Objects, heartbeats, cleans up, and reports refusals.  |
| Air-gapped guarantee      | Core commands pass under a network-blocked sandbox.                                                                  |
| Release machinery         | `scripts/release/*` can assess, prepare, tag, monitor, verify, and close out the exact `main` SHA being released.    |
| Docs/runbooks             | User-facing docs and runbooks match the shipped claim; no protection state is described more strongly than evidence. |

**Anti-goal:** do not ship a partial MLP/INTL slice under the full-protection
claim. If scope must be cut, rename the release claim before tagging.

---

## Later Windows

After the daemon-working release, promote the next slice based on real adoption
signals rather than pre-allocating a fixed version.

| Future slice                                 | Source             | Gate before promotion                                                           |
| -------------------------------------------- | ------------------ | ------------------------------------------------------------------------------- |
| Team-lead browser surface                    | Dashboard/export   | Daemon-working evidence stream exists and can be exported reliably.             |
| Enterprise / compliance / language expansion | Queued APS modules | Demand-pulled by a design partner or customer; do not pre-build as speculation. |
| Wider language and rule-pack coverage        | Queued APS modules | Core protection loop stable enough that added breadth does not dilute signal.   |
