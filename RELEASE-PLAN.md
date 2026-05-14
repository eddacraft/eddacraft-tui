# Anvil Release Plan

| Type         | Authority | Owner       | Status | Freshness                                                                                     |
| ------------ | --------- | ----------- | ------ | --------------------------------------------------------------------------------------------- |
| Release plan | Derived   | APS modules | Live   | Last reviewed 2026-05-14 (RMCPF later-window phasing added); base `v0.6.2-beta` + APS modules |

| Upstream                                                                                                                          | Downstream                                                        |
| --------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| [`plans/index.aps.md`](./plans/index.aps.md), `git tag v0.6.2-beta`, [`ROADMAP.md`](./ROADMAP.md), MLP/INTL/WATCHUX/RMCPF modules | Release runbooks, PR planning, [`ROADMAP.md`](./ROADMAP.md) links |

**Last updated:** 2026-05-14 (RMCPF later-window phasing added. MLP v1 is now
Complete at 18/18, with MLP-018 split into the new MLP2 follow-up module for 56
integration items. WATCHUX remains the beta-incident lane, INTL remains the
launcher ingress lane, and RMCPF is sequenced after the daemon-working slate.)

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

## Next Release Window — `v0.7.0-beta` "Let's Use This"

**Candidate tag:** `v0.7.0-beta`

**Claim:** _Anvil is ready to live on a senior engineer's machine for a month
without being uninstalled._ Daemon working end-to-end is necessary but no longer
sufficient. The cut also requires the protection claim is legible during
sustained use (TRUST), first-week friction is removed (ADOPT), the update path
actually reaches users (DISTRIB), and at least three internal users have run the
candidate on real work for a full week without disabling, suppressing, or
bypassing anything (Wave 5 — Boring Week).

The reframing is documented in
[`plans/specs/2026-05-14-release-plan-v0.7.0-sit-on.md`](./plans/specs/2026-05-14-release-plan-v0.7.0-sit-on.md)
and was accepted on 2026-05-14.

**Primary APS modules:**

| Pick | Module                                                      | Status      | Progress | Role                                                                                                                                                                                                                                                                            |
| ---- | ----------------------------------------------------------- | ----------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| N1   | [`MLP`](./plans/modules/multilayer-protection.aps.md)       | Complete    | 18/18    | Multi-layer protection v1 primitives: project identity, witness chain, hooks, L4 policy, baseline, audit, attribution, workflow template, and protection-claim vocabulary. MLP-018 closed by splitting follow-up integration work into MLP2.                                    |
| N1b  | [`MLP2`](./plans/modules/multilayer-protection-v2.aps.md)   | Draft       | 0/56     | Follow-up integration module created from the MLP-018 split; holds v1 deferrals and broader integration debt. It is not part of the current `v0.7.0-beta` protection claim unless promoted separately.                                                                          |
| N2   | [`INTL`](./plans/modules/intercept-launcher.aps.md)         | Ready       | 0/9      | `anvil-run` launcher, session registration, process-group control, shell wrappers, side-channel register.                                                                                                                                                                       |
| N3   | Carry-forward gates                                         | Confirmed   | 6/6      | ADR-036..039 Accepted (2026-05-13); project-id, noise-discipline policy, AIGUARD envelope, INTR-004 promoted, DRVR forward-compat — all hold. G5 closed 2026-05-13 when INTR-004 (path-deny rule) was promoted Draft → Ready in `intercept-rules.aps.md`.                       |
| N4   | Documentation lanes                                         | —           | 0/6      | Adoption, air-gap, witness-chain, hooks-integration runbooks, migration note, INTL manpage. Owner: @aneki (lands in Wave 4). Status column is `—` because these are not APS modules — see Wave 0 outcome row for the actual ownership scope.                                    |
| N5   | [`WATCHUX`](./plans/modules/watch-ux-advisory-rules.aps.md) | In Progress | 0/8      | Beta incident remediation and first-run/watch UX: Homebrew installer detection, local-noise ignore policy, initial watch scan baseline semantics, immediate watch startup feedback; later items cover warning/failing language, warm-up TUI, rule modes, and config visibility. |
| N6   | [`TRUST`](./plans/modules/adoption-trust-surface.aps.md)    | Ready       | 0/6      | Adoption Trust Surface: `anvil status` legibility, degraded-state surfacing, `anvil doctor --fix`, daemon-down recovery, JSON schema pin, first-run verification recipe. Wave 3B.                                                                                               |
| N7   | [`ADOPT`](./plans/modules/adoption-friction.aps.md)         | Ready       | 1/6      | Adoption Friction Removal: hook coexistence, resource budget, AI auto-detect, complete ignore policy, **clean uninstall (ADOPT-005) shipped 2026-05-14 via PR #1521**, editor coexistence. Wave 3A.                                                                             |
| N8   | [`DISTRIB`](./plans/modules/distribution-and-update.aps.md) | Ready       | 0/5      | Distribution & Self-Update: signature verification, `anvil version --check` advisory surface, Homebrew formula automation, cadence policy, `anvil migrate`. ADR-044 §9 makes DISTRIB-001 / -002 load-bearing for the MCP-backend swap discovery gap. Wave 3A.                   |
| N9   | [`INSIGHTS`](./plans/modules/usage-insights.aps.md)         | Ready       | 0/4      | Usage Insights: local-only `anvil insights` weekly summary, suppression health, drift trend, first-week adoption hint. Wave 4.                                                                                                                                                  |

**Hard release gates:**

1. `MLP-009` protection-claim contract suite — Done.
2. `TRUST-001` and `TRUST-002` — a non-Anvil developer reads `anvil status` once
   and explains what it means, and degraded states surface within 60s of next
   save-time interaction.
3. `ADOPT-001` hook coexistence with lefthook/husky/pre-commit-framework.
4. `ADOPT-002` measured resource ceiling (CPU < 5% steady-state, RSS < 200MB on
   reference repo) green in CI.
5. `DISTRIB-001` signature-verified update path on all install methods.
6. **Wave 5 Boring Week** — three or more internal users finish the week with
   Anvil still enabled, the same config they started with, and at least one
   journal entry describing a real catch they would not have wanted to ship.

No partial slice should be marketed as the full "let's use this" claim. If scope
must be cut, rename the release claim before tagging.

The MLP wave rows below are retained as release evidence. Current integration
debt that did not belong in the v1 primitive module now lives in MLP2.

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

| Work                                    | Parallel? | Status            | Notes                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| --------------------------------------- | --------- | ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `MLP-001` project identity              | First     | Done (2026-05-13) | Establishes `anvil/project-id`; unblocks witness, baseline, and policy. Shipped via `activation/identity.rs` (22 tests green). v1 scope: idempotent UUID v7 write at `anvil start`; concurrent-rename convergence; symlink refusal. Deferred to follow-ups (see MLP-001 footnotes): composite-identity attach-check (with MLP-014), `--new-identity` flag (with MLP-007), `anvil baseline` integration (MLP-007).                                                         |
| `MLP-002` witness chain                 | After 001 | Done (2026-05-13) | Spike as a standalone PR with flock, rollover, DAG, and 80-writer tests. New `crates/anvil-witness/` crate: line + genesis + writer (`fs2` flock + rollover) + verifier (tamper / drop / stray-genesis detection); 25 tests green plus an `#[ignore]` 80-writer stress test. DAG-aware merge verification, manifest event stream, and `merge=union` integration test deferred to follow-ups (see module footnotes 1, 2, 3). CLI integration lands with MLP-003 hook lane. |
| `MLP-011` multi-format config           | Parallel  | Done (2026-05-13) | New `crates/anvil-config/` library: extension-based dispatch into `serde_json::Value`, canonical-JSON serialisation, detection precedence yaml > yml > json > toml. 44 tests green; cross-format equivalence pinned. CLI flag wiring deferred (see footnote 1).                                                                                                                                                                                                           |
| `MLP-013` hard-pinned rule classes      | Parallel  | Done (2026-05-13) | New `validation` module in `crates/anvil-config/`: rejects five disable-attempt shapes for `secrets` / `command-safety` (canonical + legacy locations + mode-disabled); tuning passes through; error messages cite ADR-039 and the `@anvil-ignore` bypass. 19 new tests green; 5 cross-format hard-pinned integration tests. `anvil-checks` rule-registration mirror deferred (see footnote 1).                                                                           |
| `MLP-017` air-gapped guarantee scaffold | Parallel  | Done (2026-05-13) | Linux network-namespace harness at `tools/test-harness/network-blocked/run.sh` (probes the kernel; exits 77 to skip on restricted hosts and non-Linux). Integration test suite at `crates/anvil-cli/tests/air_gapped.rs` (3 tests green covering `anvil version --offline`, `anvil status --verify --json`, and an executable-bit guard). Runbook at `docs/runbooks/anvil-air-gapped.md` documenting the extend-per-command protocol.                                     |

### Wave 1A: Beta Watch UX Hotfix

This lane is in the current release because beta feedback showed that first-run
watch can look hung, scan local agent worktrees, and render advisory baseline
findings as failures. The hotfix subset is scoped to WATCHUX-001 through
WATCHUX-004; the larger warm-up TUI, rule-mode, config-command, and cache work
remains sequenced after the hotfix unless it becomes required for the release
claim.

| Work                                   | Parallel? | Status      | Notes                                                                                                       |
| -------------------------------------- | --------- | ----------- | ----------------------------------------------------------------------------------------------------------- |
| `WATCHUX-001` Homebrew installer       | Parallel  | In Progress | Detect existing Homebrew Anvil before curl installer runs standalone install; guide user to `brew upgrade`. |
| `WATCHUX-002` shared ignore policy     | Parallel  | In Progress | Skip `.claude`, `.opencode`, `.gemini`, `.serena`, `.worktrees`, generated/cache dirs in audit/watch paths. |
| `WATCHUX-003` initial watch baseline   | Parallel  | In Progress | Initial scan builds graph/readiness state without emitting existing API surface as new violations.          |
| `WATCHUX-004` watch startup feedback   | Parallel  | In Progress | Print immediate startup feedback and avoid TUI mode when stdin/stdout are not both terminals.               |
| `WATCHUX-005` warning/failing language | Follow-up | Ready       | Required before broad beta if advisory findings still render as `Failing`; otherwise follow-up UX PR.       |

### Wave 2: Hook, Policy, And Baseline Surfaces

Start once the witness primitive is stable enough for dependent lanes to write
against it.

| Work                                          | Parallel?         | Status            | Notes                                                                                                                                                                                                                                                                                                                                                                                               |
| --------------------------------------------- | ----------------- | ----------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `MLP-012` `rules_sha` in witnesses            | After 002 (+ 011) | Done (2026-05-13) | Shipped as a new `crates/anvil-rules/` library: `RulesShaInput` + `rules_sha` over canonical JSON of `{anvil_version, config_sha, opa_runtime_version, rules}` + `RequiredAnvilVersion` semver-floor parser. 29 tests green incl. yaml/json/toml cross-format determinism. (Merged via PR #1489.)                                                                                                   |
| `MLP-007` baseline command                    | After 001 + 002   | Done (2026-05-13) | v1 library primitive shipped via `crates/anvil-baseline/`: `Baseline` schema + move-resistant `compute_fingerprint` + TOCTOU-hardened I/O (incl. broken-symlink + tmp-path refusal, atomic-replace for Windows) + diff partition; 44 tests green. CLI command, scanner integration, cutoff_commit policy pinning, witness genesis emission, hook install deferred to consumers (MLP-003 / MLP-006). |
| `MLP-003` pre-commit hook                     | After 002 + 012   | Done (2026-05-13) | v1 library primitive shipped via `crates/anvil-hook/`: ADR-038 §D-1/§D-4/§D-5/§D-6/§D-7 primitives (Verdict, SuppressionLog, detect_framework, shell_template, panic_catcher_hook); 47 tests green. CLI subcommands, framework install, witness append wiring, daemon RPC deferred to consumers.                                                                                                    |
| `MLP-005` post-commit/post-merge/post-rewrite | After 003         | Done (2026-05-13) | `anvil hook post-commit/post-merge/post-rewrite` CLI subcommands; `anvil-witness` extended with `parent_commits[]` / `prev_line_hashes[]` for DAG-aware merge writes (parent enumeration via `git rev-list --parents`).                                                                                                                                                                             |
| `MLP-006` L4 policy framework                 | After 002 + 007   | Done (2026-05-13) | v1 schema + resolver shipped via `crates/anvil-l4/`: `Policy` / `BranchRule` schema (yaml/json/toml via anvil-config) + globset first-match-wins resolver + `commit_is_before_cutoff` ancestry check + four boundary-rejection error variants. 24 tests green.                                                                                                                                      |
| `MLP-008` hook bootstrap recovery             | After 003         | Done (2026-05-13) | `anvil hook bootstrap [--dry-run]` executes `BootstrapPlan` (Husky regenerate / `.git/hooks/` install / NothingToDo) with the ADR-038 §D-5 3-line wrapper. `--witness-recent` walk deferred.                                                                                                                                                                                                        |

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

### Wave 3A: Adoption Friction

Parallel-safe with Wave 3 and 3B. No shared single-point-of-failure inside the
wave. ADOPT-004 waits for WATCHUX-002 (shared ignore helper, already in flight);
the rest are independent.

| Work          | Parallel?           | Status   | Notes                                                                                              |
| ------------- | ------------------- | -------- | -------------------------------------------------------------------------------------------------- |
| `ADOPT-001`   | Parallel            | Draft    | Hook coexistence with lefthook / husky / pre-commit-framework. Extends MLP-008 bootstrap recovery. |
| `ADOPT-002`   | Parallel            | Draft    | Resource budget — CPU/RSS measurement and CI ceiling in `crates/anvil-bench`.                      |
| `ADOPT-003`   | Parallel            | Draft    | AI tool auto-detect (Claude Code, Cursor, Aider, Windsurf, Codex).                                 |
| `ADOPT-004`   | After WATCHUX-002   | Draft    | Complete the local-noise ignore policy across watch, audit, hooks, `anvil-run`.                    |
| `ADOPT-005`   | —                   | **Done** | `anvil uninstall` shipped 2026-05-14 via PR #1521.                                                 |
| `ADOPT-006`   | Parallel            | Draft    | Editor surface coexistence matrix.                                                                 |
| `DISTRIB-001` | First (sig scheme)  | Draft    | Harden `anvil update` resolution chain + signature verification (cosign or minisign, new ADR).     |
| `DISTRIB-002` | After `DISTRIB-001` | Draft    | `anvil version --check` newer-version + advisory surface.                                          |
| `DISTRIB-003` | Parallel            | Draft    | Homebrew formula auto-bump on release.                                                             |
| `DISTRIB-004` | Parallel            | Draft    | `docs/policies/release-cadence.md` + EOL policy.                                                   |
| `DISTRIB-005` | After `DISTRIB-002` | Draft    | `anvil migrate` for cross-version config reconciliation.                                           |

### Wave 3B: Trust Surface

Parallel-safe with Wave 3 and 3A. TRUST-001 unblocks TRUST-002 and TRUST-005;
TRUST-003 / -004 / -006 are parallel after TRUST-001.

| Work        | Parallel?         | Status | Notes                                                                                                      |
| ----------- | ----------------- | ------ | ---------------------------------------------------------------------------------------------------------- |
| `TRUST-001` | First             | Draft  | `anvil status` plain-mode legibility — single screen, names the protection state, single next-action line. |
| `TRUST-002` | After `TRUST-001` | Draft  | Degraded-state surfacing within 60s of next save-time interaction; banner rate-limited.                    |
| `TRUST-003` | Parallel          | Draft  | `anvil doctor --fix` recovery for documented bad states.                                                   |
| `TRUST-004` | Parallel          | Draft  | Daemon-down auto-recovery — hooks detect and re-arm; `anvil start` is idempotent.                          |
| `TRUST-005` | After `TRUST-001` | Draft  | `anvil status --json` schema pinned at `anvil-status.v1`.                                                  |
| `TRUST-006` | Parallel          | Draft  | First-run claim summary + verification recipe.                                                             |

### Wave 4: Release-Gate Closure

Do not tag until these are green and reflected in release evidence.

| Work                                  | Status | Notes                                                                                                               |
| ------------------------------------- | ------ | ------------------------------------------------------------------------------------------------------------------- |
| `MLP-004` pre-push hook               | Done   | Shipped via PR #1499; walks pushed ranges, verifies witnesses, and applies L4 fallback.                             |
| `MLP-009` protection-claim contract   | Done   | Closed-set protection vocabulary and contract evidence shipped with MLP v1.                                         |
| `MLP-010` GitHub Action publishing    | Done   | Shipped via PR #1504; workflow template/accessor exposes the L4/CI surface.                                         |
| `MLP-015` L5 audit                    | Done   | Shipped via PR #1500; audit-chain lane covers bypassed-layer detection.                                             |
| `MLP-016` L1 editor driver → Kindling | Done   | Shipped via PR #1503; mid-edit Kindling observation builder completed.                                              |
| Documentation lanes                   | Gate   | Adoption, air-gap, witness-chain, hooks-integration, migration, and `anvil-run` docs still gate release evidence.   |
| `INSIGHTS-001`                        | Draft  | `anvil insights` weekly summary derived from witness chain + suppression log; schema pinned at `anvil-insights.v1`. |
| `INSIGHTS-002`                        | Draft  | Suppression health view; flags stale suppressions where the underlying violation is gone.                           |
| `INSIGHTS-003`                        | Draft  | Drift trend sparkline — 8 weeks of new cross-boundary edges; reports "insufficient data" honestly when applicable.  |
| `INSIGHTS-004`                        | Draft  | First-week adoption hint nudging new users at `anvil insights` once per week for 14 days.                           |

### Wave 5: Boring Week Validation Gate

**Pre-tag gate.** No amount of test fixture coverage proves "ready to use." The
only proof is using it.

**Entry criteria — which work must be green before Wave 5 can start:**

| Wave                          | Required for entry                                                                                            | Rationale                                                                                                                                                                                                                                                                                                                    |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Wave 3 (Coordination + INTL)  | `MLP-014` + `INTL-001..-005` only                                                                             | INTL-001..-005 deliver the wrapped-launch ingress that the protection-claim render path depends on. INTL-006..-009 (shell wrappers, side-channel registration, blocked-launch UX, heartbeat) are sequenced for the same release but can defer to `v0.7.1` per the Risks table — they refine UX rather than enable the claim. |
| Wave 3A (Adoption Friction)   | All six ADOPT items + DISTRIB-001..-003                                                                       | DISTRIB-004 (cadence doc) and DISTRIB-005 (`anvil migrate`) are not Boring-Week-blocking.                                                                                                                                                                                                                                    |
| Wave 3B (Trust Surface)       | All six TRUST items                                                                                           | All are load-bearing for the legibility gate.                                                                                                                                                                                                                                                                                |
| Wave 4 (Release-Gate Closure) | MLP-004 / -009 / -010 / -015 / -016 (already Done), INSIGHTS-001 + -004, documentation lanes, release runbook | INSIGHTS-002 / -003 (suppression health, drift sparkline) are not Boring-Week-blocking; they ship in the same release but can land late.                                                                                                                                                                                     |

If any Wave 3 INTL item from the deferrable set (-006..-009) is _not_ green at
freeze time, it must be explicitly listed as deferred-to-v0.7.1 in the release
notes so the cut claim still matches reality.

**Protocol:**

1. With the entry criteria above met, freeze the candidate SHA.
2. Three or more internal users install the candidate via the fresh-user path
   (Homebrew install or curl installer; no developer overrides) on their primary
   work machine.
3. For one calendar week, each user runs Anvil against their normal daily work.
4. Each user keeps a journal of every visible warning, every suppression, every
   bypass, every disabled check, every daemon failure, and every `anvil doctor`
   invocation.
5. End-of-week review: any disabled check or unresolved suppression is a cut
   blocker. Any "I gave up and turned it off" event is a cut blocker. Any daemon
   failure that did not auto-recover is a cut blocker.

**Exit criterion:** All three users finish the week with Anvil still enabled,
the same configuration they started with, and at least one journal entry
describing a real catch they would not have wanted to ship.

**Non-goal:** Wave 5 is not a perf test or a stress test. It is a sustained-use
trust test. The instrument is the journal, not a benchmark.

**Participants:** TBD by @aneki before tag. Journals land in
`plans/audits/2026-XX-XX-boring-week-v0.7.0.md` as the release record.

---

## Cut Criteria For `v0.7.0-beta`

`v0.7.0-beta` is cuttable only when the release evidence supports the public
claim ("Anvil is ready to live on a senior engineer's machine for a month
without being uninstalled") without caveats.

| Criterion                    | Required Evidence                                                                                                               |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Protection claim correctness | `MLP-009` green: every allowed state is reachable in fixtures and rendered honestly.                                            |
| Protection claim legibility  | `TRUST-001` and `TRUST-002` green: a non-Anvil developer reads `anvil status` once and explains what it means.                  |
| Save-time and hook layers    | Pre-commit, pre-push, post-commit/post-merge/post-rewrite hooks run with ADR-038 noise discipline.                              |
| Hook coexistence             | `ADOPT-001` green: install + run alongside lefthook / husky / pre-commit-framework on representative configs.                   |
| Witness integrity            | Concurrent writes, rollover, tamper detection, DAG verification, and worktree survival tests pass.                              |
| Baseline adoption            | Existing repositories can adopt without broad warning noise; hard-pinned classes remain enforced.                               |
| Agent launch path            | `anvil-run` registers sessions, isolates process groups / Job Objects, heartbeats, cleans up, and reports refusals.             |
| Resource budget              | `ADOPT-002` green: CPU < 5% steady-state, RSS < 200MB on the reference repo, measured in CI.                                    |
| Update path                  | `DISTRIB-001` green: signature-verified `anvil update` on Homebrew, curl-installer, and library paths.                          |
| Clean uninstall              | `ADOPT-005` green (shipped 2026-05-14): `anvil uninstall` returns a repo to byte-identical pre-install state for tracked files. |
| Air-gapped guarantee         | Core commands pass under a network-blocked sandbox.                                                                             |
| Release machinery            | `scripts/release/*` can assess, prepare, tag, monitor, verify, and close out the exact `main` SHA being released.               |
| Docs/runbooks                | User-facing docs and runbooks match the shipped claim; no protection state is described more strongly than evidence.            |
| Boring Week                  | Wave 5 protocol completed; no cut blockers raised.                                                                              |

**Anti-goal:** do not ship a partial MLP/INTL/TRUST/ADOPT slice under the full
"let's use this" claim. If scope must be cut, rename the release claim before
tagging.

**Anti-goal:** do not bypass Wave 5 because the candidate looks good. The
candidate always looks good. The Boring Week exists because looks-good and
gets-used diverge.

---

## Hotfix Iteration Plan (Post-Tag)

`v0.7.0-beta` is the release we sit on. "Sit on" means **no major release for
six weeks** unless a Boring-Week-tier regression appears. Hotfix iteration
shape:

| Cadence        | Channel                               | Scope                                                                 |
| -------------- | ------------------------------------- | --------------------------------------------------------------------- |
| `v0.7.x` patch | Weekly while user signal is non-empty | Bug fixes, false-positive reductions, doc corrections.                |
| `v0.7.x` patch | Within 48h of any P0 bug              | Crash, data loss, false-claim regression, daemon corruption.          |
| `v0.7.y` minor | Not before 6 weeks post-tag           | Feature additions; only if `v0.7.0` baseline retention is stable.     |
| `v0.8.0-beta`  | Demand-pulled                         | Driven by a real adopter requirement, not by completion of a backlog. |

The hotfix policy makes "we sit on this" trustworthy — without it, "the release
we sit on" becomes "the release we wait on until the next big thing." See also
DISTRIB-004 (cadence + EOL policy doc).

---

## Risks

| Risk                                                            | Mitigation                                                                                                                                |
| --------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Scope expansion makes ship date slip indefinitely.              | Each new module has explicit defer-to-`v0.7.x` items marked in its task body. Cut to the gate criteria, not below.                        |
| Boring Week protocol is performative, not real.                 | Require journal artefacts as a release record. Journals land in `plans/audits/2026-XX-XX-boring-week-v0.7.0.md`.                          |
| `anvil-run` (INTL) is not actually needed for "let's use this." | Defer cleanly: INTL-001..-005 ship in `v0.7.0`; INTL-006..-009 can land in `v0.7.1` if Wave 3 slips and they would block Boring Week.     |
| TRUST surface increases noise.                                  | TRUST-002 has an explicit noise budget (≤1 banner per 60s, never more than one concurrent). Tests pin this.                               |
| MCP-backend swap silently fails for existing users.             | ADR-044 §9 + DISTRIB-001 / -002. Until DISTRIB ships, the release notes carry the manual "run `anvil start` after upgrading" instruction. |
| New modules clash with existing in-flight work.                 | All Wave 3A / 3B work depends only on Done MLP surfaces and one WATCHUX item (WATCHUX-002, already in flight).                            |

---

## Later Windows

After the daemon-working release, promote the next slice based on real adoption
signals rather than pre-allocating a fixed version.

### RMCPF: Rust MCP Full-Port Phasing

RMCPF is the likely next MCP-focused lane after the daemon-working slate, but it
should not block `v0.7.0-beta`. The v0.7 claim is daemon/hooks/witness/launcher
protection; RMCPF replaces the archived TypeScript MCP server once parity is
defined and demand for each surface is clear.

| Phase                              | Scope                                                                                                                                       | Gate                                                                                                                           |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| 0 — Inventory lock                 | Complete RMCPF-001 against `archive/anvil-mcp-server/src/`; confirm client matrix and Streamable HTTP demand.                               | Inventory matrix reviewed; remaining module-level readiness blockers are explicit client, transport, and retirement decisions. |
| 1 — Core tool parity               | Implement RMCPF-010 and RMCPF-011 in Rust under `anvil mcp serve`, preserving DRVR-006 daemon/local classifications and DRVR-007 redaction. | Fixture parity for `anvil_check`, `anvil_gate`, `anvil_status`, `anvil_fix`, `anvil_suppress`, and `anvil_query_boundary`.     |
| 2 — Resources, prompts, transports | Implement or retire RMCPF-012, RMCPF-020, and RMCPF-021 using `docs/architecture/rust-mcp-server-spec.md` as authority.                     | Resource read/list tests pass; prompt and HTTP retain/retire decisions documented.                                             |
| 3 — Cutover and retirement         | Ship RMCPF-030 compatibility harness and RMCPF-031 TypeScript MCP retirement/archive decision.                                              | Generated configs and release-critical docs point at Rust MCP; migration doc names all intentional incompatibilities.          |

| Future slice                                 | Source             | Gate before promotion                                                           |
| -------------------------------------------- | ------------------ | ------------------------------------------------------------------------------- |
| Rust MCP full port                           | RMCPF              | Phase 0 inventory lock complete and supported-client demand confirmed.          |
| Team-lead browser surface                    | Dashboard/export   | Daemon-working evidence stream exists and can be exported reliably.             |
| Enterprise / compliance / language expansion | Queued APS modules | Demand-pulled by a design partner or customer; do not pre-build as speculation. |
| Wider language and rule-pack coverage        | Queued APS modules | Core protection loop stable enough that added breadth does not dilute signal.   |
