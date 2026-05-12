# Multi-Layer Protection

| ID  | Owner  | Status | Progress      |
| --- | ------ | ------ | ------------- |
| MLP | @aneki | Ready  | 0/17 complete |

**Last reviewed:** 2026-05-13 (Wave 0 readiness review — MLP-009 confirmed as
hard release gate for `v0.7.0-beta`; ADRs 036–039 now Accepted; ready for Wave 1
implementation)

> **Scope.** MLP is the v1 module that ships the multi-layer
> protection backbone: witness chain, hooks, L4 policy framework,
> baseline, multi-agent coordination, rule distribution + Rego support.
> Together with DLIFE (daemon lifecycle) it makes the dream claim
> ("Anvil protects this project") defensible. INTD remains the
> daemon-internal module; DRVR remains the surface-driver module; MLP
> sits on top of both.

> **MLP-009 is the hard release gate** for the module. Protection-
> claim contract test suite + air-gapped operation guarantee + no-
> noise discipline tests must all be green before any MLP item is
> marked Complete in `plans/index.aps.md`. Reason: the public claim
> only exists if it's tested.

## Purpose

Anvil's defense-in-depth model (L0–L5) needs a coordinated set of
mechanisms that work together to make "Anvil protects this project"
a defensible claim, not a slogan. This module owns:

- The **witness chain** primitive (per-commit tamper-evident proof
  of which layers fired, in-tree, travels via git).
- The **hook surface** (pre-commit, pre-push, post-commit, post-merge,
  post-rewrite) under noise discipline.
- The **L4 policy framework** (per-branch rules,
  `validate_at_l4` server-side fallback, `cutoff_commit` for
  baselined repos, integration via pre-push hook + CI action).
- The **`anvil baseline`** mechanism for adopting Anvil into existing
  repos.
- The **multi-agent coordination** primitives needed for sub-agent-wave
  workflows (per-task fence isolation, attribution chain, rule version
  pinning, DAG-aware verification).
- The **rule distribution / versioning** model (`rules_sha`,
  hard-pinned classes, Rego custom-rule support).
- The **L5 audit** command + nightly CI workflow.

## In Scope

- Per-commit witness lines (`anvil/witnessed.ndjson`) with hash chain
- Active + archive + manifest with rollover at 1000 lines / 1 MB
- `flock`-protected chain integrity
- Pre-commit / pre-push / post-commit / post-merge / post-rewrite hooks
- Husky / lefthook / pre-commit-framework / plain integration
- L4 policy framework (`anvil/policy.yml`)
- `validate_at_l4` server-side validation fallback
- `refs/notes/anvil-l4` for L4-generated witnesses
- `anvil baseline` mechanism + `anvil/baseline.json` shape
- Per-rule-class baseline behaviour (hard-pinned secrets / command-safety)
- `anvil hook bootstrap` recovery command
- Worktree silent self-heal (daemon-driven)
- Multi-session-per-worktree (`AgentTag` composite key) + per-task
  fence isolation (promoted from DLIFE-008)
- `ANVIL_TASK_ID` env propagation + process-tree attribution fallback
- DAG-aware witness verification at L4
- Rule version pinning (`rules_sha` per witness)
- Hard-pinned rule classes (`secrets`, `command-safety`)
- Multi-format config (yaml / json / toml) auto-detection
- `eddacraft/anvil-action` GitHub Marketplace publishing
- Closed-set protection-claim contract test suite (extends DLIFE-009)
- L1 editor driver → Kindling mid-edit observation emission
- Air-gapped operation guarantee + tests
- Org-shared rules via git submodule pattern (documented convention)
- `anvil audit` command + nightly CI workflow

## Out of Scope

- GitHub App (v2 amplifier — separate module / future ADR)
- GitLab / Bitbucket native integrations (vNext)
- Pre-receive hook script for self-hosted git (vNext universal v2)
- Anvil cloud sidecar / hosted services (vNext, opt-in only)
- Rule pack distribution channel beyond git-tracked (vNext)
- Cross-Windows ↔ WSL surface bridging (vNext, separate ADR)
- macOS App Sandbox detection observability (DLIFE-010, v1.5)
- `prepare-commit-msg` / `commit-msg` hooks (v1.5)
- Migration tooling for `project_uuid` changes (deferred per direction)

## Interfaces

- **Depends on:**
  - INTD (daemon enforcement pipeline; `scan_buffer` RPC; fence state;
    watcher events for new worktrees)
  - DLIFE (daemon lifecycle, discovery via `info.json`,
    `os_locality_token`, `anvil intercept ensure`)
  - DRVR (driver framework; editor-driver protocol)
  - RMCP / RMCPF (MCP shim; `validation.backend` honesty rule)
  - RTAI (mid-edit validation semantics; midEdit envelope; debouncer)
  - kindling-integration (write `gate_evaluated`, `action_executed`,
    `constraint_applied`, `error` observations; per-(machine, project)
    DB)
  - anvil-checks (built-in Rust rules: secrets, AI-001, command-safety,
    antipattern)
  - opa-agent-orchestration (Rego runtime; pack architecture stub for
    vNext)
  - anvil-cli `commands/start.rs` (extend the activation orchestrator
    with MLP steps)
  - anvil-cli `commands/init.rs` (multi-format config support)
  - LAUNCH (extends `ProtectionState` enum with MLP states)
- **Exposes:**
  - `anvil hook pre-commit | post-commit | pre-push | post-merge | post-rewrite`
    binary subcommands
  - `anvil hook bootstrap [--witness-recent]` recovery command
  - `anvil baseline [--refresh] [--scope <path>] [--verify]` command
  - `anvil baseline verify` re-scan command
  - `anvil audit` on-demand audit command
  - `anvil project status [--json]` extended status surface
  - `anvil/witnessed.ndjson` + `anvil/witness/` tree as in-repo
    artefact
  - `anvil/baseline.json` + `anvil/policy.yml` (or `.json`/`.toml`) +
    `anvil/project-id` as tracked repo files
  - `eddacraft/anvil-action` GitHub Marketplace action
  - `.github/workflows/anvil.yml` (PR validation) + `.github/workflows/anvil-audit.yml`
    (nightly cron) templates

## ADRs cited

- **ADR-036** (rewritten 2026-05-07) — Daemon scope, discovery, OS
  boundary. Defines per-execution-scope model that MLP rests on.
- **ADR-037** (new) — Witness chain + L4 policy framework. The
  load-bearing decision for cross-machine determinism.
- **ADR-038** (new) — Hook surface + noise discipline (the Serena
  rule). Behavioural governance for everything MLP ships.
- **ADR-039** (new) — Baseline policy + hard-pinned rule classes.
  Adoption mechanism for existing repos.

## Tasks

### MLP-001: Project identity (`anvil/project-id`)

- **Intent:** Establish stable cross-machine project identity at
  adoption time.
- **Expected Outcome:** `anvil start` writes `anvil/project-id` (UUID
  + optional `forked_from`); `anvil baseline` does the same. Composite
  identity (`project_uuid`, `first_commit`, `origin_canonical`) checked
  at daemon attach time. Forks inherit by default; `--new-identity`
  flag opts out.
- **Files:** `crates/anvil-cli/src/activation/identity.rs` (new),
  edits in `commands/start.rs`, `commands/init.rs`,
  `commands/baseline.rs` (new — see MLP-007).
- **Validation:** Idempotent on re-run; cross-check warns on origin
  mismatch; tests cover greenfield, baseline, fork-inherit, fork-opt-out.
- **Confidence:** high
- **Priority:** Critical (load-bearing)
- **Dependencies:** none

### MLP-002: Witness chain (active + archive + manifest + hash chain)

- **Intent:** Implement the in-tree witness primitive that every other
  MLP layer reads/writes.
- **Expected Outcome:**
  - `anvil/witnessed.ndjson` (active) + `anvil/witness/manifest/chain.ndjson`
    + `anvil/witness/archive/<scope>-<seq>-<merkle>.ndjson`
  - Hash chain via `prev_line_hash` (sha256 of preceding line); anchors
    `GENESIS-FRESH` / `GENESIS-BASELINED`.
  - `flock(LOCK_EX)` on `anvil/witness/.lock` for chain integrity.
  - Rollover at 1000 lines OR 1 MB (whichever first), atomic inside
    the lock, content-addressed archive naming, manifest event
    appended.
  - `merge=union -text` on active + manifest via `.gitattributes`.
  - DAG-aware verification (merge commits carry `parent_commits[]` +
    `prev_line_hashes[]`).
- **Files:** `crates/anvil-witness/` (new crate), `Cargo.toml`,
  `crates/anvil-cli/src/witness/` (CLI integration).
- **Validation:**
  - Concurrent-write tests (80+ parallel hooks)
  - Rollover-during-burst tests
  - Tamper detection (modify historical line → chain break detected)
  - DAG verification across merge commits
  - `merge=union` integration test (two branches, merge produces both
    lines)
  - `anvil/witnessed.ndjson` survives `git worktree add` (tracked in
    `anvil/`, not `.anvil/`)
- **Confidence:** medium
- **Priority:** Critical (load-bearing)
- **Dependencies:** MLP-001

### MLP-003: Pre-commit hook (L3 validation + witness append)

- **Intent:** Self-contained binary subcommand that fires from any
  hook framework or none.
- **Expected Outcome:**
  - `anvil hook pre-commit` subcommand
  - 3-line shell wrapper with `command -v anvil` guard
  - Reads `anvil/project-id`; resolves Kindling DB path;
    validates diff via daemon RPC or embedded fallback;
    appends witness via MLP-002.
  - Noise-disciplined output per ADR-038.
  - Panic catcher demotes panics to exit-0 + log + witness-error line.
- **Files:** `crates/anvil-cli/src/commands/hook.rs` (new),
  `crates/anvil-hook/` (new crate for shared hook logic),
  shell template at `crates/anvil-cli/src/hook/templates/`.
- **Validation:**
  - Husky / lefthook / pcf / no-framework integration tests
  - Exit codes: 0 on pass/warn/internal-error; 1 on block; never panic
  - Noise-discipline tests (silent on success; one line on failure)
  - Embedded fallback path when daemon unreachable
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** MLP-001, MLP-002

### MLP-004: Pre-push hook (L4 client-side validation)

- **Intent:** Walk pushed commit range; verify chain integrity;
  validate any unwitnessed commits per `anvil/policy.yml`.
- **Expected Outcome:**
  - `anvil hook pre-push` subcommand
  - Reads stdin (git pre-push contract: `<local-ref> <local-sha> <remote-ref> <remote-sha>` per line)
  - For each ref: walk `<remote-sha>..<local-sha>`; verify each
    commit's witness; `validate_at_l4` for unwitnessed commits per
    policy
  - Time budget <2s p95 for typical push; hard cap with `partial: true`
    for very large pushes
- **Files:** `crates/anvil-cli/src/commands/hook.rs` (extend),
  `crates/anvil-l4/` (new crate for L4 policy logic).
- **Validation:**
  - Range-walk tests (single commit, many commits, force-push,
    rebase-replay)
  - Chain integrity break detection
  - `validate_at_l4` integration
  - Per-branch policy resolution tests
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** MLP-001, MLP-002, MLP-003, MLP-006

### MLP-005: post-commit / post-merge / post-rewrite handlers

- **Intent:** Record commit-time state changes in Kindling; handle
  witness chain edge cases (merges, rebases, amends).
- **Expected Outcome:**
  - `anvil hook post-commit` — emit Kindling `action_executed`;
    update daemon's chain head cache.
  - `anvil hook post-merge` — record merge join in witness chain
    (DAG-aware); Kindling `action_executed`.
  - `anvil hook post-rewrite` — regenerate witnesses for amended /
    rebased commits.
- **Files:** `crates/anvil-cli/src/commands/hook.rs` (extend).
- **Validation:**
  - Merge-commit witness has `parent_commits[]` array
  - Amend/rebase regenerates witness for new commit object
  - Time budget <100ms p95 per handler
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** MLP-002, MLP-003

### MLP-006: L4 policy framework (`anvil/policy.yml`)

- **Intent:** Per-branch policy rules + `validate_at_l4` server-side
  fallback.
- **Expected Outcome:**
  - `anvil/policy.yml` (or `.json` / `.toml`) parser
  - Per-branch pattern matching for `require:` / `on_no_witness:` /
    `on_block:` / `on_warn:` rules
  - `cutoff_commit` legacy acceptance
  - `validate_at_l4` runs the same rule pipeline server-side; writes
    L4 witness to `refs/notes/anvil-l4`
- **Files:** `crates/anvil-l4/` (new crate; covers MLP-004 + MLP-006),
  `crates/anvil-cli/src/policy/` (config parser).
- **Validation:**
  - Branch-pattern matching tests
  - Legacy cutoff acceptance
  - `validate_at_l4` round-trip tests
  - Multi-format config parsing
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** MLP-002, MLP-007 (for cutoff_commit)

### MLP-007: `anvil baseline` command

- **Intent:** Adopt Anvil into an existing repo with deep history.
- **Expected Outcome:**
  - `anvil baseline` scans current tree; records findings in
    `anvil/baseline.json`; pins `cutoff_commit` in policy file;
    writes witness genesis line; installs hooks; stages everything.
  - `anvil baseline --refresh` re-scans at HEAD; updates baseline;
    writes `baseline-refreshed` line to chain.
  - `anvil baseline --verify` re-scans without writing; confirms
    recorded findings still exist.
  - Per-rule-class default baseline behaviour per ADR-039.
  - Hard-pinned `secrets` and `command-safety` classes refused at
    config-parse time.
  - Adversarial-refresh detection (`degraded:baseline-suspicious`).
  - Time budget: <60s for 12k files; bounded scan with async
    continuation for >100k files.
- **Files:** `crates/anvil-cli/src/commands/baseline.rs` (new),
  `crates/anvil-baseline/` (new crate for fingerprint + scan logic).
- **Validation:**
  - Greenfield baseline (no existing findings)
  - Existing-repo baseline (with findings across multiple classes)
  - Refresh: resolved + new + remaining counted correctly
  - Hard-pinned class rejection at parse time
  - Fingerprint stability across line moves
  - `--verify` detects falsified baseline metadata
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** MLP-001, MLP-002

### MLP-008: `anvil hook bootstrap` recovery command

- **Intent:** Recover from worktree-bootstrap failure (hooks didn't
  fire; missing witnesses on local commits).
- **Expected Outcome:**
  - Detects framework (husky / lefthook / pcf / none); regenerates
    runtime files (e.g., `.husky/_/` from `package.json`).
  - `--witness-recent` mode: walks `<remote>..HEAD`; runs validation
    against each unwitnessed commit; writes retroactive witnesses
    tagged `validation_at: bootstrap-recovery`.
  - One-line success output: `anvil: bootstrapped (N commits witnessed retroactively)`.
- **Files:** `crates/anvil-cli/src/commands/hook.rs` (extend).
- **Validation:**
  - Husky `_/` regeneration without `pnpm install`
  - Retroactive witness only for `<remote>..HEAD` range
  - Idempotent on re-run
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** MLP-002, MLP-003

### MLP-009: Protection-claim contract test suite (HARD GATE)

- **Intent:** Pin the closed-set protection-claim states as testable
  contract. **Hard release gate for the module.**
- **Expected Outcome:**
  - For each state in spec §14.2 (per-worktree) and §14.1 (per-surface),
    drive system into that state and assert rendered claim matches
    pinned string.
  - JSON schema fixtures for `anvil status --json`; field-additions
    forward-compat tested; field-removals fail.
  - Tests cover: `unprotected`, `warming`, `pre-write-embedded`,
    `pre-write-daemon`, `save-time-only`, `full`, `degraded-protection`,
    `cross-boundary-mixed`, `multi-daemon-detected`, `path-uncertain`.
  - **No MLP item marked Complete in `plans/index.aps.md` until this
    is green.**
- **Files:** `crates/anvil-cli/tests/protection_claim_states.rs` (new),
  `apps/e2e/src/protection_claim_states.spec.ts` (new),
  `crates/anvil-cli/tests/fixtures/status_v1/` (JSON snapshots).
- **Validation:** All ten worktree states reachable in fixture; all
  eight surface states reachable in fixture; rendered claim matches;
  JSON schema stable.
- **Confidence:** high (extension of existing DLIFE-009 pattern)
- **Priority:** Critical (RELEASE GATE)
- **Dependencies:** MLP-002 through MLP-008

### MLP-010: GitHub Action publishing

- **Intent:** `eddacraft/anvil-action` available on GitHub Marketplace
  for `uses: eddacraft/anvil-action@v1`.
- **Expected Outcome:**
  - Separate publishing repo at `github.com/eddacraft/anvil-action`
  - Action runs the same `anvil l4-validate` binary
  - `.github/workflows/anvil.yml` template generated by `anvil start`
    references `eddacraft/anvil-action@v1`
  - Versioned with semver; major-version tags (`v1`) auto-update to
    latest minor/patch
  - Documented inputs (policy file path, fail-on-warning, etc.)
- **Files:** new repo `eddacraft/anvil-action` (separate publishing
  pipeline), template at
  `crates/anvil-cli/src/templates/anvil-workflow.yml`.
- **Validation:**
  - PR check fires; verdict surfaced as GitHub check status
  - Branch protection integration (require check before merge) tested
  - Action installs anvil binary in runner; same rule pipeline
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** MLP-006 (L4 policy framework drives the action's
  decisions)

### MLP-011: Multi-format config (yaml / json / toml)

- **Intent:** Support `.anvil.yaml`, `.anvil.yml`, `.anvil.json`,
  `.anvil.toml`. Detection in that order; first match wins.
- **Expected Outcome:**
  - Parser dispatches on extension; canonical-JSON serialisation for
    `rules_sha` computation (format-independent hash).
  - `anvil start --format json|toml` overrides default YAML.
  - `anvil/policy.*` matches `.anvil.*` choice (consistency).
- **Files:** `crates/anvil-config/` (new crate for unified parsing),
  edits in `crates/anvil-cli/src/commands/init.rs`.
- **Validation:**
  - Equivalent yaml + json + toml configs produce same `rules_sha`
  - Detection precedence
  - Round-trip parse + serialise for each format
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** none

### MLP-012: `rules_sha` computation in witness lines

- **Intent:** Every witness line records the deterministic hash of
  the rule set used.
- **Expected Outcome:**
  - `rules_sha = sha256(anvil_version + opa_runtime_version + sorted_rules + config_sha)`
  - Daemon caches `(worktree_key, rules_sha) → ResolvedRuleSet`;
    invalidation on `.anvil.*` watcher event.
  - In-flight evaluations finish with their pinned `rules_sha`.
  - `required_anvil_version` floor in policy file; hook checks at fire time.
  - L4 verifies witness `rules_sha` is from a recognised version;
    falls back to revalidation if outside policy.
- **Files:** `crates/anvil-rules/src/sha.rs` (new),
  edits in `crates/anvil-witness/`, `crates/anvil-l4/`.
- **Validation:**
  - Cross-machine determinism (same anvil_version + same config →
    same sha)
  - Format-independent (yaml + toml equivalent → same sha)
  - In-flight evaluation pinning under config-update burst
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** MLP-011

### MLP-013: Hard-pinned rule classes (`secrets`, `command-safety`)

- **Intent:** Config parser refuses configs that disable hard-pinned
  classes.
- **Expected Outcome:**
  - Parser-level enforcement (not runtime check). Same pattern as
    ADR-015 ambiguous-ownership hard-cap.
  - Hard-pinned classes documented in code; rule registration carries
    `hard_pinned: bool` field.
  - Per-finding `@anvil-ignore` bypass remains available (ADR-004).
- **Files:** `crates/anvil-config/src/validation.rs` (new),
  `crates/anvil-checks/` (rule registration metadata).
- **Validation:**
  - Config attempting `enforcement.rules.secrets.enabled: false` →
    parse error
  - Config tuning rule params (severity, mode) → accepted
  - Per-finding suppression still works
- **Confidence:** high
- **Priority:** Critical (security)
- **Dependencies:** MLP-011

### MLP-014: Multi-session-per-worktree + per-task fence isolation

- **Intent:** Promoted from DLIFE-008 v1.5 to MLP v1 per user direction.
  Sub-agent waves require per-task fence scope so one bad sub-agent
  doesn't cascade-fence a whole worktree.
- **Expected Outcome:**
  - Session key becomes `(WorktreeKey, AgentTag)` rather than just
    `WorktreeKey`.
  - `AgentTag` minted by daemon from `(driver_id, claimed_agent_id, pid_starttime)`.
  - Fence keys: `(WorktreeKey, AgentTag)` for per-task fences;
    worktree-level fence still triggered for unattributable writers.
  - Per-worktree session cap (default 16, configurable
    `enforcement.session.per_worktree_max`).
  - `degraded:fence-cascade` mode triggered if >5 fences in 60s;
    operator-clear required.
  - `ANVIL_TASK_ID` env var inherited through process spawns;
    process-tree walk fallback when env missing.
  - **Trust model.** `ANVIL_TASK_ID` and `ANVIL_AGENT_TAG` are
    advisory hints, not authenticated identity — any same-UID
    process can spoof or unset them. The daemon MUST cross-check an
    env-supplied `AgentTag` against the `AgentTag` it issued for this
    pid lineage at INTL-003; mismatches are treated as missing, not
    honoured. The process-tree walk fallback finds a registered
    ancestor on env miss; a walk that finds none downgrades to
    worktree-level fence (ADR-038 noise discipline applies). Witness
    chain (ADR-037 §D-2) and `validate_at_l4` (§D-5) are the
    authentication backstop.
- **Files:** `crates/anvil-intercept/src/registry.rs` (extend session
  key), `crates/anvil-intercept-proto/src/session.rs` (`AgentTag`
  stub landed 2026-05-13; extend in MLP-014),
  `packages/anvil-driver-client/src/session.ts` mirror,
  `crates/anvil-attribution/` (new — env propagation + process-tree
  walk).
- **Validation:**
  - Two sessions same worktree distinguished by AgentTag
  - Per-task fence does not affect other sessions in same worktree
  - Worktree-level fence on unattributable still applies to all
  - Cascade detection at 5 in 60s
  - Process-tree walk finds registered ancestor on env var miss
  - Spoofed env (`ANVIL_AGENT_TAG` set to a tag that was never
    registered for this pid lineage) is rejected — not silently
    honoured — and the session is treated as unattributable
- **Confidence:** medium (substantial extension of existing INTD-003)
- **Priority:** Critical
- **Dependencies:** MLP-002 (witness chain encodes AgentTag), MLP-003
  (hook reads AgentTag for witness)

### MLP-015: L5 audit (on-demand + nightly CI)

- **Intent:** Periodic re-scan of mainline for drift detection.
  Catches commits that bypassed all earlier layers (admin overrides,
  force-push manipulation).
- **Expected Outcome:**
  - `anvil audit` command — on-demand re-scan of current branch;
    reports drift since last audit.
  - `.github/workflows/anvil-audit.yml` — nightly cron template
    written by `anvil start` / `anvil baseline`. Active by default
    (per user direction); user can comment-out to disable.
  - Audit emits Kindling `gate_evaluated` with `mode: audit`.
  - Audit drift triggers `degraded:audit-drift` mode if findings
    exceed configured threshold.
- **Files:** `crates/anvil-cli/src/commands/audit.rs` (new),
  template `anvil-audit-workflow.yml`.
- **Validation:**
  - Cron-driven nightly audit produces reproducible output
  - `anvil audit` on-demand matches scheduled run
  - Drift threshold detection
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** MLP-006 (uses same rule pipeline as L4)

### MLP-016: L1 editor driver → Kindling integration

- **Intent:** Editor driver emits Kindling `gate_evaluated` with
  `mode: midEdit` for findings, preserving forensic detail of "what
  the AI tried to write mid-edit."
- **Expected Outcome:**
  - DRVR mid-edit findings (warn / block decisions) emit a Kindling
    `gate_evaluated` observation with `mode: midEdit`.
  - Pass-no-finding mid-edit calls remain silent (volume control).
  - No witness file write at L1 (no commit yet); witness happens at
    L3 if the edit is saved + committed.
  - Coordinates with RTAI-007 telemetry contract.
- **Files:** edits in `crates/anvil-intercept/src/midedit.rs`,
  `packages/anvil-driver-client/src/`,
  `crates/anvil-cli/src/mcp/validation.rs` (mirror for MCP shim).
- **Validation:**
  - Mid-edit warn/block produces Kindling row
  - Pass-no-finding produces no Kindling row
  - Volume bounded under burst
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** RTAI-007 telemetry contract; existing DRVR-002
  protocol

### MLP-017: Air-gapped operation guarantee + tests

- **Intent:** Per user direction, v1 must work fully without internet.
  Test suite asserts no network calls in core operation.
- **Expected Outcome:**
  - `anvil start`, `anvil baseline`, `anvil intercept ensure`, all
    `anvil hook` subcommands, `anvil audit` make zero network calls
    in normal operation.
  - Test suite uses a sandboxed network-blocking harness; assert no
    DNS / TCP / HTTP attempts from the relevant code paths.
  - Pack distribution (vNext) constrained to git-based fetch (no
    Anvil cloud HTTPS).
  - Documentation: `docs/runbooks/anvil-air-gapped.md` describing
    the guarantee and how it's tested.
- **Files:** `crates/anvil-cli/tests/air_gapped.rs` (new),
  `tools/test-harness/network-blocked/` (new sandbox).
- **Validation:**
  - Run all v1 commands under network-blocked harness; all succeed
  - Telemetry / usage analytics opt-in only (off by default)
  - CI ensures no new network call regressions
- **Confidence:** medium
- **Priority:** Critical (doctrine commitment)
- **Dependencies:** none — instrumentation across the codebase

## Stats

| Phase | Items | Status |
| ----- | ----- | ------ |
| Foundations (identity, witness, hooks) | 5 (MLP-001..-005) | 0/5 |
| Policy + adoption | 3 (MLP-006, -007, -008) | 0/3 |
| Hard release gate | 1 (MLP-009) | 0/1 |
| CI + config | 2 (MLP-010, -011) | 0/2 |
| Rule distribution | 2 (MLP-012, -013) | 0/2 |
| Coordination + audit | 3 (MLP-014, -015, -016) | 0/3 |
| Doctrine | 1 (MLP-017) | 0/1 |
| **Total** | **17** | **0/17** |

## Recommended landing order

Implementation should land in this order:

1. **MLP-001** project identity (no deps; unblocks rest)
2. **MLP-011** multi-format config (no deps)
3. **MLP-002** witness chain (depends on MLP-001)
4. **MLP-013** hard-pinned classes (depends on MLP-011)
5. **MLP-012** rules_sha (depends on MLP-011)
6. **MLP-003** pre-commit hook (depends on MLP-001, -002, -012, -013)
7. **MLP-014** multi-session-per-worktree + fence isolation
8. **MLP-005** post-* hook handlers (depends on MLP-002, -003)
9. **MLP-007** `anvil baseline` (depends on MLP-001, -002)
10. **MLP-006** L4 policy framework (depends on MLP-002, -007)
11. **MLP-004** pre-push hook (depends on MLP-006)
12. **MLP-008** `anvil hook bootstrap` (depends on MLP-003)
13. **MLP-016** L1 → Kindling
14. **MLP-015** L5 audit (depends on MLP-006)
15. **MLP-017** air-gapped tests (instrumentation across all)
16. **MLP-010** CI action publishing (depends on MLP-006)
17. **MLP-009** contract test suite — **HARD RELEASE GATE**

## Risks

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| Lock contention under 82-commit burst | Medium | Medium | Crude alternative (lazy rollover) is filed simplification path; benchmark MLP-002 under realistic burst |
| Husky `_/` regeneration brittle across husky versions | Medium | High | Vendor known-good runtime files per husky major version; document fallback to direct `.git/hooks` install |
| `merge=union` corner cases on the witness file | Low | High | Custom merge driver as escape hatch; tested against parallel-branch scenarios |
| User config ignores hard-pinned classes via novel encoding | Low | Critical | Parser-level rejection covers known patterns; defense in depth via runtime checks too |
| Air-gapped guarantee broken by future contribution | Medium | High | MLP-017 tests catch regressions in CI |
| Rule version drift between machines causes spurious L4 revalidation | Medium | Medium | `required_anvil_version` floor; documented mixed-version expected behaviour |
| `anvil baseline` performance on huge monorepos | Medium | Medium | Bounded scan budget + async continuation; partial baseline marker |

## Decisions

1. **Witness chain in-tree, not in notes ref or sidecar.** Air-gapped
   doctrine demands self-contained per-repo verification (ADR-037).
2. **Hooks integrate, never replace.** Husky / lefthook / pcf / plain
   all work the same way (ADR-038).
3. **Per-task fence isolation in v1, not v1.5.** Multi-agent stress
   profile demands it (user direction 2026-05-07).
4. **Hard-pinned `secrets` and `command-safety` cannot be config-
   disabled.** Defense at parser level (ADR-039).
5. **Forks inherit `project_uuid` by default.** Rules travel with
   the repo (ADR-036 §D-2).
6. **L4 v1 = pre-push + CI action.** GitHub App is v2 amplifier;
   not required for wow-start (user direction 2026-05-07).
7. **Air-gapped operation guaranteed at v1.** No cloud calls in
   normal operation (user direction 2026-05-07).
8. **Org-shared rules via git submodule (documented convention).**
   No new pack mechanism in v1 (user direction 2026-05-07).

## Coordinates with

- **INTD** — daemon enforcement pipeline; watcher events; fence state
  primitives extended to per-task scope (MLP-014).
- **DLIFE** — daemon discovery (`info.json`), `os_locality_token`,
  `anvil intercept ensure`. Some DLIFE work items consolidated here
  per spec §17.2.
- **DRVR** — driver framework provides the L1 channel; MLP-016 wires
  Kindling emission.
- **RMCP / RMCPF** — `validation.backend` honesty rule on MCP responses.
- **RTAI** — mid-edit validation backbone; MLP-016 builds on RTAI-007
  telemetry.
- **LAUNCH** — `anvil start` orchestrator extends with MLP-001 / -002 /
  -003 / -006 / -010 / -011 / -015 steps; `ProtectionState` enum
  gains MLP states.
- **kindling-integration** — receives observations from L1/L2/L3/L4/L5;
  per-(machine, project) DB path.
- **opa-agent-orchestration** — Rego runtime for custom rules; pack
  architecture stub for vNext.
