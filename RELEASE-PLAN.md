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

## Next Release Window — Daemon-Working Slate

**Candidate tag:** `v0.7.0-beta`

**Claim:** _Daemon working end-to-end._ `anvil start` lands a real, testable
protection claim; hooks fire deterministically; the witness chain records every
commit; baseline adoption works; and `anvil-run` wraps agent processes.

**Primary APS modules:**

| Pick | Module                                                      | Status      | Progress | Role                                                                                                                                                                                                                                                                            |
| ---- | ----------------------------------------------------------- | ----------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| N1   | [`MLP`](./plans/modules/multilayer-protection.aps.md)       | Complete    | 18/18    | Multi-layer protection v1 primitives: project identity, witness chain, hooks, L4 policy, baseline, audit, attribution, workflow template, and protection-claim vocabulary. MLP-018 closed by splitting follow-up integration work into MLP2.                                    |
| N1b  | [`MLP2`](./plans/modules/multilayer-protection-v2.aps.md)   | Draft       | 0/56     | Follow-up integration module created from the MLP-018 split; holds v1 deferrals and broader integration debt. It is not part of the current `v0.7.0-beta` protection claim unless promoted separately.                                                                          |
| N2   | [`INTL`](./plans/modules/intercept-launcher.aps.md)         | Ready       | 0/9      | `anvil-run` launcher, session registration, process-group control, shell wrappers, side-channel register.                                                                                                                                                                       |
| N3   | Carry-forward gates                                         | Confirmed   | 6/6      | ADR-036..039 Accepted (2026-05-13); project-id, noise-discipline policy, AIGUARD envelope, INTR-004 promoted, DRVR forward-compat — all hold. G5 closed 2026-05-13 when INTR-004 (path-deny rule) was promoted Draft → Ready in `intercept-rules.aps.md`.                       |
| N4   | Documentation lanes                                         | —           | 0/6      | Adoption, air-gap, witness-chain, hooks-integration runbooks, migration note, INTL manpage. Owner: @aneki (lands in Wave 4). Status column is `—` because these are not APS modules — see Wave 0 outcome row for the actual ownership scope.                                    |
| N5   | [`WATCHUX`](./plans/modules/watch-ux-advisory-rules.aps.md) | In Progress | 0/8      | Beta incident remediation and first-run/watch UX: Homebrew installer detection, local-noise ignore policy, initial watch scan baseline semantics, immediate watch startup feedback; later items cover warning/failing language, warm-up TUI, rule modes, and config visibility. |

**Hard release gate:** `MLP-009`. The protection-claim contract suite, air-gap
guarantee, and noise-discipline tests must be green before the release can claim
"Anvil protects this project." No partial slice should be marketed as full
protection.

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

### Wave 4: Release-Gate Closure

Do not tag until these are green and reflected in release evidence.

| Work                                  | Status | Notes                                                                                                             |
| ------------------------------------- | ------ | ----------------------------------------------------------------------------------------------------------------- |
| `MLP-004` pre-push hook               | Done   | Shipped via PR #1499; walks pushed ranges, verifies witnesses, and applies L4 fallback.                           |
| `MLP-009` protection-claim contract   | Done   | Closed-set protection vocabulary and contract evidence shipped with MLP v1.                                       |
| `MLP-010` GitHub Action publishing    | Done   | Shipped via PR #1504; workflow template/accessor exposes the L4/CI surface.                                       |
| `MLP-015` L5 audit                    | Done   | Shipped via PR #1500; audit-chain lane covers bypassed-layer detection.                                           |
| `MLP-016` L1 editor driver → Kindling | Done   | Shipped via PR #1503; mid-edit Kindling observation builder completed.                                            |
| Documentation lanes                   | Gate   | Adoption, air-gap, witness-chain, hooks-integration, migration, and `anvil-run` docs still gate release evidence. |

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
