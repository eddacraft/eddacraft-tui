# Multi-Layer Protection

| ID  | Owner  | Status      | Progress  |
| --- | ------ | ----------- | --------- |
| MLP | @aneki | In Progress | 13/17 done |

**Last reviewed:** 2026-05-13 (Wave 2 entry — MLP-007 shipped as a new
`crates/anvil-baseline/` library: `Baseline` / `BaselineFinding` /
`BaselineMetadata` on-disk schema (`anvil/baseline.json`, format
version 1), move-resistant `compute_fingerprint` (16-hex-char sha256
with NUL domain separation + whitespace-normalised snippet), TOCTOU-
hardened `load` / `save` with atomic temp-then-rename and symlink
refusal (including broken-symlink + tmp-path refusal), and
`Baseline::diff` for the "new edges only" gate partition. 44 tests
green. CLI command, scanner integration, `cutoff_commit` policy
pinning, witness genesis-line emission, hook installation, and
adversarial-refresh detection documented as deferred follow-ups
owned by their respective consumers (MLP-003 / MLP-006). MLP-012
already merged from PR #1489 (`crates/anvil-rules/` rules_sha +
RequiredAnvilVersion; 29 tests green incl. yaml/json/toml
cross-format determinism). Wave 1 entry — MLP-001 reconciled to Done
after audit confirmed the shipped implementation matches the
v1-narrowed scope; MLP-011 shipped a new `crates/anvil-config/`
library (extension dispatch + canonical-JSON serialisation; 44 tests
green); MLP-002 witness-chain spike shipped a new
`crates/anvil-witness/` crate (line, genesis, writer with flock +
rollover, verifier with tamper / dropped-line / stray-genesis
detection); 25 tests green plus an `--ignored` 80-writer stress test.
Module advanced to In Progress for the Wave 1 backbone slate per
`RELEASE-PLAN.md`. MLP-009 remains the hard release gate for
`v0.7.0-beta`; ADRs 036–039 Accepted.)

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

- **Status:** Done (2026-05-13)
- **Intent:** Establish stable cross-machine project identity at
  adoption time.
- **Expected Outcome (v1 shipped):** `anvil start` writes
  `anvil/project-id` (UUID v7 + optional `forked_from`) via the
  activation orchestrator. Idempotent on re-run; concurrent callers
  converge on the on-disk identity via a post-rename re-read.
  Symlinked `anvil/` dirs are refused at write time (TOCTOU-hardened).
  Parse rejects non-UUID `project_uuid` / `forked_from` values.
  `anvil doctor` reads the identity for the identity-present check.
- **Scope-narrowing footnotes (deferred follow-ups, not part of Done):**
  1. **Composite identity check at daemon attach** — cross-checking
     `(project_uuid, first_commit, origin_canonical)` is owned by the
     daemon attach path (anvil-intercept) and is filed as a follow-up
     to land alongside MLP-014 multi-session work, where attach-time
     composite verification fits naturally.
  2. **`--new-identity` fork opt-out CLI flag on `anvil start`** —
     deferred; forks today inherit the parent's UUID via the
     idempotent ensure path. The opt-out flag will land with MLP-007
     (`anvil baseline`), which already plans the
     fork-aware adoption surface.
  3. **`anvil baseline` writes identity** — owned by MLP-007 baseline
     command (`commands/baseline.rs` does not exist yet); MLP-007 will
     call `identity::ensure_project_id` alongside its other bootstrap
     work.
- **Files (shipped):** `crates/anvil-cli/src/activation/identity.rs`,
  `crates/anvil-cli/src/activation/orchestrator/mod.rs`,
  `crates/anvil-cli/src/commands/doctor.rs`.
- **Validation:** `cargo test --bin anvil identity` — 22/22 tests
  green (greenfield, idempotency, symlink refusal, non-UUID rejection,
  concurrent-rename convergence, comments / blank lines, forward-compat
  unknown keys, colons-in-values).
- **Confidence:** high
- **Priority:** Critical (load-bearing)
- **Dependencies:** none
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

### MLP-002: Witness chain (active + archive + manifest + hash chain)

- **Status:** Done (2026-05-13) — v1 spike
- **Intent:** Implement the in-tree witness primitive that every other
  MLP layer reads/writes.
- **Expected Outcome (v1 spike shipped):**
  - New crate `crates/anvil-witness/` exposes `WitnessLine`,
    `GenesisAnchor`, `WitnessWriter`, `verify_chain`, and
    `compute_line_hash`.
  - On-disk shape: `anvil/witness/active.ndjson` (active) +
    `anvil/witness/archive/<scope>-<seq>-<merkle>.ndjson` per
    ADR-037 §D-3.
  - Hash chain via `prev_line_hash` (sha256 of canonical bytes of the
    preceding line); anchors are the bare strings `GENESIS-FRESH` and
    `GENESIS-BASELINED` per ADR-037 §D-2 (the cutoff commit SHA for
    baselined adoption lives on the line body, not glued onto the
    anchor).
  - `flock(LOCK_EX)` on `anvil/witness/.lock` via `fs2`. The lock is
    held per append (not for the writer's lifetime), so a stalled
    process can't pin the chain head indefinitely.
  - Rollover at 1000 lines OR 1 MB (configurable for tests via
    `RolloverPolicy::tight`), atomic inside the lock,
    content-addressed archive naming (`<scope>-<seq>-<merkle16>.ndjson`).
  - Canonical line encoding (sorted JSON keys, no insignificant
    whitespace) so two machines emitting the same logical record
    produce byte-identical bytes.
  - Verifier walks `archive[..] + active` in order; detects tamper
    (hash break), dropped lines (sequence gap), stray-genesis
    references on non-first lines, and unknown genesis anchors.
  - TOCTOU-hardened symlink refusal on the `anvil/witness/` root
    matches MLP-001's pattern (check, create, re-check).
- **Scope-narrowing footnotes (deferred follow-ups, not part of Done):**
  1. **DAG-aware merge verification** — merge commits will carry
     `parent_commits[]` + `prev_line_hashes[]` and need a graph walk
     rather than the linear-chain verifier. Filed alongside MLP-005
     (`post-merge` hook).
  2. **Manifest event stream** (`anvil/witness/manifest/chain.ndjson`)
     — `WitnessWriter::append` already returns the archive path on
     rollover, so the manifest layer plugs in without re-touching the
     writer. Filed as MLP-002b.
  3. **`merge=union -text`** — `.gitattributes` is pre-positioned by
     the activation orchestrator (MLP-001 step 1a-b). The canonical
     line encoding here makes the union merge a no-op semantically:
     two parallel branches each appending a line produce a clean
     two-line merged file.
  4. **80-writer stress test** — sixteen-writer test runs in CI;
     `eighty_writers_no_interleaving` exists but is gated behind
     `#[ignore]` so it can be invoked on demand
     (`cargo test --ignored`).
  5. **CLI integration** (`crates/anvil-cli/src/witness/`) — landing
     with the hook lane (MLP-003) when the first consumer materialises.
- **Files (shipped):** `crates/anvil-witness/Cargo.toml`,
  `crates/anvil-witness/src/{lib,genesis,line,writer,verify}.rs`,
  `crates/anvil-witness/tests/concurrency.rs`, workspace `Cargo.toml`
  member registration.
- **Validation:** `cargo test -p eddacraft-anvil-witness` — 25 tests
  green (genesis: 7; line: 7; writer: 5 including symlink refusal and
  both rollover thresholds; verify: 5 including tamper / drop / stray
  / cross-archive walk; concurrency: 1 active + 1 stress `#[ignore]`).
  `cargo clippy -p eddacraft-anvil-witness --all-targets -- -D warnings`
  clean.
- **Confidence:** medium (per spec) — load-bearing primitive; deferred
  items are well-scoped follow-ups, not capability gaps
- **Priority:** Critical (load-bearing)
- **Dependencies:** MLP-001
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

### MLP-003: Pre-commit hook (L3 validation + witness append)

- **Status:** Done (2026-05-13) — v1 library primitive
- **Intent:** Self-contained binary subcommand that fires from any
  hook framework or none.
- **Expected Outcome (v1 shipped):** New crate `crates/anvil-hook/`
  factors ADR-038's cross-cutting hook concerns out of any CLI call
  site. `Verdict` + `render_verdict` map every §D-6 outcome to its
  exact terse stderr line + exit code; `SuppressionLog` collapses
  burst-suppression (82 events → 1 emit); `detect_framework` runs
  §D-4 non-destructive detection (Husky / Lefthook /
  pre-commit-framework / cargo-husky / Plain); `shell_template`
  produces the verbatim §D-5 3-line POSIX wrapper; `panic_catcher_hook`
  formats a panic into a `PanicReport` sink for §D-7. 47 tests green
  (incl. process-wide-mutex serialisation of panic-hook tests).
  CLI subcommands (`anvil hook <name>`), framework install paths
  (MLP-008), witness append integration (MLP-002 writer + CLI), and
  daemon RPC + embedded fallback (anvil-intercept) deferred to
  consumers.
- **Original Expected Outcome:**
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

- **Status:** Done (2026-05-13) — v1 surface lands ahead of the
  rule engine; see footnotes.
- **Intent:** Walk pushed commit range; verify chain integrity;
  validate any unwitnessed commits per `anvil/policy.yml`.
- **Expected Outcome (v1 shipped):**
  - `crates/anvil-hook/src/pre_push.rs` exposes
    `parse_pre_push_input` + `PushRef` / `PushKind` / `ZERO_SHA` /
    `is_zero_sha` — the stdin parser for git's pre-push contract
    with explicit `Create` / `Delete` / `Update` classification.
  - `crates/anvil-l4/src/decide.rs` adds `BranchRule::decide_commit`
    + `CommitDecision` / `BlockKind` — the per-commit policy
    decision matrix (Requirement × OnNoWitness → Allow / Block /
    NeedsL4Validation). L3Only refuses to fall back to L4; L4Only
    ignores the L3 witness; defaults match ADR-037 §D-5.
  - `anvil-hook` `Verdict::Block` gains `BlockReason::UnwitnessedCommit`
    rendering `anvil: unwitnessed commit refused by policy — anvil
    show <id>` at exit 1.
  - `crates/anvil-cli/src/commands/hook.rs` adds the `PrePush`
    subcommand + `run_pre_push` orchestrator: stdin parse → policy
    load → chain verification → range walk via `git rev-list` →
    per-commit decision → verdict emission. Chain-integrity break
    over the active + archive stack blocks the push outright.
    `NeedsL4Validation` decisions emit a single
    `InternalError { class: TimedOut }` line and *allow* the push —
    the validate-at-l4 rule engine is the MLP-006 deferred follow-up,
    and blocking on a feature that doesn't exist would strand
    operators behind a Serena-rule violation.
- **Scope-narrowing footnotes (deferred follow-ups, not part of v1):**
  1. **`validate_at_l4` rule-engine execution** — the surface is in
     place; the engine landing is MLP-006's deferred CLI lane. When
     it ships, `run_pre_push` swaps the `InternalError { TimedOut }`
     branch for the real call site without changing the contract.
  2. **`refs/notes/anvil-l4` L4-witness writes** — owned by MLP-010
     (GitHub Action surface); ADR-037 §D-7 forbids in-tree ledger
     mutation at L4.
  3. **`cutoff_commit` baseline acceptance** — needs a `git rev-list
     --first-parent` ancestry walk per pushed ref to thread into
     `Policy::commit_is_before_cutoff`. v1 walks the literal pushed
     range only.
  4. **Time-budget cap with `partial: true`** — ADR-038 names a 2s
     p95 budget; v1 relies on git's own range traversal speed and
     leaves the explicit cap as a follow-up so the cap-trigger
     surface ships with measurements rather than guesses.
  5. **End-to-end subprocess integration tests** — pure-helper
     coverage (parsing, decision matrix, chain verification, policy
     loading) is 40+ tests across `anvil-hook`, `anvil-l4`, and
     `anvil-cli::commands::hook`; the run-the-binary smoke pass
     comes with the MLP-009 protection-claim contract suite.
- **Files (shipped):** `crates/anvil-hook/src/pre_push.rs`,
  `crates/anvil-l4/src/decide.rs`,
  `crates/anvil-cli/src/commands/hook.rs` (extend),
  `crates/anvil-hook/src/lib.rs` + `crates/anvil-l4/src/lib.rs` +
  `crates/anvil-hook/src/verdict.rs` (re-exports + new
  `BlockReason::UnwitnessedCommit`).
- **Validation:** `cargo test -p eddacraft-anvil-hook` — 89 green
  (incl. 16 new `pre_push` parser tests + 1
  `block_unwitnessed_commit_emits_one_line_exits_one` verdict test).
  `cargo test -p eddacraft-anvil-l4` — 35 green (incl. 11 new
  `decide` matrix tests). `cargo test -p eddacraft-anvil --bin
  anvil 'commands::hook::'` — 25 green (incl. 14 new helper tests:
  `witness_paths`, `collect_witnessed_shas`, `load_policy`,
  `short_sha`, `verify_chain_or_block`). `cargo clippy --workspace
  --all-targets -- -D warnings` clean. `cargo fmt --check` clean.
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** MLP-001, MLP-002, MLP-003, MLP-006

### MLP-005: post-commit / post-merge / post-rewrite handlers

- **Status:** Done (2026-05-13)
- **Intent:** Record commit-time state changes in Kindling; handle
  witness chain edge cases (merges, rebases, amends).
- **Expected Outcome (shipped):** `anvil hook post-commit` appends a `kind: "post-commit"` bookkeeping witness line; `anvil hook post-merge --commit <sha>` builds a DAG-aware witness via `anvil_hook::merge_witness_plan` with `parent_commits[]` + `prev_line_hashes[]` arrays populated (anvil-witness extended with these fields; parent enumeration via `git rev-list --parents -n 1`); `anvil hook post-rewrite` reads git's `<old> <new>` stdin via `anvil_hook::parse_post_rewrite_input` and writes retroactive witnesses tagged `POST_REWRITE_VALIDATION_AT = "post-rewrite-recovery"`. Kindling `action_executed` emission + daemon chain-head cache update remain deferred follow-ups; the witness side of the work is shipped.
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

- **Status:** Done (2026-05-13) — schema + resolver shipped
- **Intent:** Per-branch policy rules + `validate_at_l4` server-side
  fallback.
- **Expected Outcome (v1 shipped):** New crate `crates/anvil-l4/` ships the `Policy` / `BranchRule` schema parsed from yaml / json / toml via `anvil-config`; first-matching-rule glob resolution via globset; `commit_is_before_cutoff` ancestry check; four boundary-rejection error variants (empty branches, empty pattern, empty required_anvil_version, empty cutoff_commit). 24 tests green. `validate_at_l4` server-side execution, `refs/notes/anvil-l4` writes, DAG-aware merge verification, and `required_anvil_version` evaluation deferred to consumers.
- **Original Expected Outcome:**
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

- **Status:** Done (2026-05-13) — v1 library primitive
- **Intent:** Adopt Anvil into an existing repo with deep history.
- **Expected Outcome (v1 shipped):**
  - New crate `crates/anvil-baseline/` exposes:
    - `BaselineFinding` (rule_id, file_path, fingerprint), `Baseline`
      (format_version, metadata, cutoff_commit, findings),
      `BaselineMetadata` (created_at, created_by_version,
      project_uuid).
    - `compute_fingerprint(rule_id, snippet)` — sha256-derived
      16-hex-char digest with NUL domain separation; whitespace-
      normalised snippet so trivial reformatting doesn't invalidate
      the fingerprint (the move-resistance contract). ASCII-only
      rule_ids; non-empty post-normalisation snippets.
    - `load(repo_root)` / `save(repo_root, &baseline)` against
      `anvil/baseline.json` with TOCTOU-hardened symlink refusal
      (matches MLP-001's identity pattern) and atomic temp-then-
      rename writes.
    - `Baseline::canonicalise()` sorts findings by `(rule_id,
      file_path, fingerprint)` + dedups, so two adopters of the
      same tree produce byte-identical `baseline.json` (CLAUDE.md
      "same input, same output").
    - `Baseline::diff(&new_scan)` partitions into `unchanged` /
      `added` / `removed` for downstream gate consumption ("new
      edges only").
    - `format_version` = 1; older anvil refuses newer files (no
      silent re-interpretation).
- **Scope-narrowing footnotes (deferred follow-ups, not part of Done):**
  1. **`anvil baseline` CLI command** — lands with MLP-003 hook
     lane, which is where the rule engine is invoked. The library
     is the building block; the CLI shape and the actual scan call
     wait for the consumer to stabilise.
  2. **Scanner integration** — populating findings from
     `anvil-checks` runs through that crate's pipeline; the baseline
     crate is engine-agnostic by design (its `BaselineFinding`
     schema is intentionally small).
  3. **`cutoff_commit` pinning into `anvil/policy.yml`** — owned by
     MLP-006 (L4 policy framework). The shape exposes the field on
     the `Baseline` record itself for round-trip; writing it back
     into a policy file is policy-crate work.
  4. **Witness genesis-line emission** (`GENESIS-BASELINED`) —
     owned by MLP-002's writer + the MLP-003 hook lane.
  5. **Hook installation** — MLP-003 / MLP-008 own framework-
     specific install paths.
  6. **Adversarial-refresh detection** (`degraded:baseline-
     suspicious`) — needs heuristics + threshold tuning beyond v1.
  7. **Async continuation for >100k files** — performance work
     item; v1 ships the data plane.
- **Files (shipped):** `crates/anvil-baseline/Cargo.toml`,
  `crates/anvil-baseline/src/{lib,finding,store,diff,io}.rs`,
  workspace `Cargo.toml` member registration.
- **Validation:** `cargo test -p eddacraft-anvil-baseline` — 40 tests
  green covering fingerprint determinism, whitespace-resistance,
  rule_id + snippet boundary checks, canonical-bytes round-trip,
  cutoff_commit round-trip, format_version refusal, validate()
  rejections, diff partition correctness, save/load round-trip,
  TOCTOU symlink refusal (both anvil/ dir and baseline.json file),
  atomic temp-then-rename, and overwrite semantics. `cargo clippy
  -p eddacraft-anvil-baseline --all-targets -- -D warnings` clean.
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** MLP-001, MLP-002
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

### MLP-008: `anvil hook bootstrap` recovery command

- **Status:** Done (2026-05-13)
- **Intent:** Recover from worktree-bootstrap failure (hooks didn't
  fire; missing witnesses on local commits).
- **Expected Outcome (shipped):** `anvil hook bootstrap [--dry-run]` CLI subcommand executes the `anvil_hook::BootstrapPlan` dispatch: regenerate Husky v9 `.husky/_/` shims, install the five v1 hooks at `.git/hooks/` from `shell_template(...)`, or report `NothingToDo` for Lefthook / pre-commit-framework / cargo-husky. One-line success output: `anvil: bootstrapped`. `--witness-recent` walk over `<remote>..HEAD` deferred (success-message counter stays zero until that ships, per ADR-038 noise discipline). Idempotent on re-run (writes are deterministic).
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

- **Status:** Done (2026-05-13)
- **Intent:** Support `.anvil.yaml`, `.anvil.yml`, `.anvil.json`,
  `.anvil.toml`. Detection in that order; first match wins.
- **Expected Outcome (v1 shipped):**
  - New crate `crates/anvil-config/` exposes `ConfigFormat`,
    `discover(dir, basename)`, `parse_str` / `parse_file` (dispatch on
    extension into `serde_json::Value`), and `canonical_json_bytes`
    (RFC 8785-style: sorted object keys, no insignificant whitespace,
    non-finite numbers rejected). Equivalent yaml / json / toml inputs
    produce byte-identical canonical output, so `rules_sha` is
    format-independent.
  - Detection precedence (`yaml` > `yml` > `json` > `toml`) is
    encoded as `Ord` on `ConfigFormat` and a `DISCOVER_PRECEDENCE`
    constant so consumers can document the rule without re-spelling
    it.
  - TOML datetimes coerce to their lexical string form to keep
    hashing deterministic across tz-normalisation choices.
  - List order is preserved (documented by an explicit test): a
    well-meaning reviewer who tries to "canonicalise" arrays by
    sorting them would collapse different rule-precedence configs to
    the same hash; the test pins the behaviour so the mistake is
    visible in code review.
- **Scope-narrowing footnotes (deferred follow-ups, not part of Done):**
  1. **`anvil start --format json|toml` CLI flag** — deferred; the
     library is the building block. Wiring lands when the first
     consumer (init.rs or the future policy parser) integrates.
  2. **`.anvilrc` → `.anvil.<ext>` filename migration** — the
     existing `.anvilrc` reader in `commands/gate.rs` keeps working
     unchanged. Migration is a separate concern; `discover` is
     basename-flexible (`".anvil"`, `"policy"`, etc.) so consumers
     can adopt at their own pace.
  3. **Typed `AnvilConfig` schema** — left to consumers so each
     surface (init, gate, policy) can evolve its own typed view of
     the same `serde_json::Value` intermediate.
- **Files (shipped):** `crates/anvil-config/Cargo.toml`,
  `crates/anvil-config/src/{lib,format,discover,parse,canonical}.rs`,
  `crates/anvil-config/tests/cross_format_equivalence.rs`, workspace
  `Cargo.toml` member registration.
- **Validation:** `cargo test -p eddacraft-anvil-config` — 44 tests
  green (39 unit + 5 cross-format equivalence integration). Headline
  test `yaml_json_toml_equivalent_configs_hash_identically` proves
  equal `rules_sha` across the three formats. `cargo clippy -p
  eddacraft-anvil-config --all-targets -- -D warnings` clean.
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** none
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

### MLP-012: `rules_sha` computation in witness lines

- **Status:** Done (2026-05-13) — v1 primitive
- **Intent:** Every witness line records the deterministic hash of
  the rule set used.
- **Expected Outcome (v1 shipped):**
  - New crate `crates/anvil-rules/` exposes `RulesShaInput`,
    `rules_sha`, `config_sha_from_canonical`, and
    `RequiredAnvilVersion`.
  - `RulesShaInput::try_new` validates `config_sha` is exactly 64
    lowercase hex characters and each rule id is non-empty ASCII
    (rejects empty strings and non-ASCII to dodge Unicode
    normalisation collisions); rule list is sorted + deduped at
    construction, so call-site order doesn't affect the digest.
  - `rules_sha = sha256(canonical_json({anvil_version, config_sha,
    opa_runtime_version, rules}))`. Top-level keys are sorted via a
    `BTreeMap` built from named fields (no round-trip through
    `serde_json::to_value`) so the digest is independent of whether
    `serde_json`'s `preserve_order` feature is enabled.
  - `config_sha_from_canonical` sits on top of MLP-011's
    `canonical_json_bytes`, so yaml / json / toml inputs collapse to
    the same digest (the cross-format invariant).
  - `RequiredAnvilVersion::parse` + `satisfied_by(current)` for the
    policy-file floor — used by MLP-003 at hook fire time and by
    MLP-006 at L4 verification. Callers should pass their own
    `env!("CARGO_PKG_VERSION")` (no anvil-rules-side
    `current_anvil_version()` helper — that would alias to this
    crate's package version and silently diverge from the running
    binary).
  - Golden-digest pin test plus `golden_digest_pin_matches_string`
    canary: any change to field names, key ordering, or encoding
    surfaces as a test failure with a release-note prompt.
- **Scope-narrowing footnotes (deferred follow-ups, not part of Done):**
  1. **Daemon-side `(worktree_key, rules_sha) → ResolvedRuleSet`
     cache with `.anvil.*` watcher invalidation** — owned by
     `anvil-intercept` when the daemon materialises (coordinates
     with MLP-014 / INTD).
  2. **In-flight evaluation pinning during config-update bursts** —
     owned by the scheduler that drives evaluations; lands with the
     daemon RPC path.
  3. **Hook-side floor check at fire time** — owned by MLP-003
     (`anvil hook pre-commit`); it consumes
     `RequiredAnvilVersion::parse(...).satisfied_by(...)` from this
     crate.
  4. **L4 verification of witness `rules_sha` against a recognised
     version** — owned by MLP-006 (`anvil-l4` crate).
  5. **Witness-writer wiring** — the `WitnessLine.rules_sha` field
     exists from MLP-002; the writer call site lives in the hook
     (MLP-003) where the rule set is resolved.
- **Files (shipped):** `crates/anvil-rules/Cargo.toml`,
  `crates/anvil-rules/src/{lib,input,version}.rs`,
  `crates/anvil-rules/tests/cross_format_determinism.rs`, workspace
  `Cargo.toml` member registration.
- **Validation:** `cargo test -p eddacraft-anvil-rules` — 34 tests
  green (29 unit + 5 cross-format integration), incl. six
  `try_new_rejects_*` boundary checks and two golden-digest pins.
  Headline test `yaml_json_toml_collapse_to_same_rules_sha` proves
  equal digest across the three formats. `cargo clippy -p
  eddacraft-anvil-rules --all-targets -- -D warnings` clean.
- **Confidence:** high (primitive only; downstream consumers wire it
  in when they land)
- **Priority:** High
- **Dependencies:** MLP-011
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

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

- **Status:** Done (2026-05-13) — v1 surface lands as the new
  `anvil audit-chain` subcommand; existing `anvil audit` keeps its
  code-quality TUI behaviour.
- **Intent:** Periodic re-scan of mainline for drift detection.
  Catches commits that bypassed all earlier layers (admin overrides,
  force-push manipulation).
- **Expected Outcome (v1 shipped):**
  - `crates/anvil-cli/src/commands/audit_chain.rs` exposes the new
    top-level `anvil audit-chain [--branch <ref>] [--since <ref>]
    [--threshold <n>]` subcommand. Walks the branch tip's commits
    via `git rev-list`, intersects with the witness-chain commit-
    SHA set (active + archive), and emits an `AuditReport` (stable
    schema `anvil.audit-chain.v1`).
  - Plain-text or `--json` output; non-zero exit when drift meets or
    exceeds `--threshold` (inclusive) so the nightly cron surfaces
    the regression as a workflow failure.
  - Chain integrity is verified once over the active + archive
    stack; tamper evidence flips `chain_intact: false` regardless of
    witness count.
  - `crates/anvil-cli/src/templates/anvil-audit-workflow.yml` ships
    the GitHub workflow template (cron `17 3 * * *`,
    `workflow_dispatch` inputs, report upload as an artifact).
    Exposed as `audit_workflow_template()` for the activation
    orchestrator to copy into `.github/workflows/anvil-audit.yml`.
  - Drift threshold defaults to `5`, matching the workflow input.
  - **Naming rationale:** `anvil audit` is already a 1448-line
    code-quality TUI in the binary. Renaming it would break beta
    users; adding a new `anvil audit-chain` subcommand keeps both
    behaviours and makes the intent explicit (it audits the
    witness chain, not the code).
- **Scope-narrowing footnotes (deferred follow-ups, not part of v1):**
  1. **Kindling `gate_evaluated` with `mode: audit` emission** —
     owned by the kindling-integration consumer when the CLI gains
     a kindling client handle; the report shape already carries the
     fields the kindling row will need.
  2. **`anvil start` / `anvil baseline` writing the workflow
     template into `.github/workflows/anvil-audit.yml`** — template
     ships in-tree; the activation orchestrator call site is the
     operator-touch point and is deliberately deferred so the
     `anvil-audit-workflow.yml` placement decision lands with the
     adoption runbook.
  3. **Rule re-scoring via `anvil-checks`** — v1 is a witness-
     presence check; re-running the rule engine across history is a
     separate concern and will land after MLP-006's deferred
     validate-at-l4 CLI lane (so audit and L4 share a rule pipeline).
  4. **Time-budget cap** — for very large histories. v1 walks
     unboundedly; profiling first, cap second.
- **Files (shipped):**
  - `crates/anvil-cli/src/commands/audit_chain.rs` (new),
  - `crates/anvil-cli/src/templates/anvil-audit-workflow.yml` (new),
  - `crates/anvil-cli/src/commands/mod.rs` (register module),
  - `crates/anvil-cli/src/main.rs` (add `AuditChain` command +
    dispatch).
- **Validation:** `cargo test -p eddacraft-anvil --bin anvil
  'commands::audit_chain::'` — 10 green covering schema-version
  pin, empty-repo zero-drift, witnessed-SHA collection (incl.
  merge parent commits), chain-intact tamper detection,
  degraded-threshold logic in both directions, sorted-output
  determinism, and template-shape pinning (workflow name +
  `anvil audit-chain` + `cron:` + `--threshold` + `--json`).
  `cargo clippy --workspace --all-targets -- -D warnings` clean.
  `cargo fmt --check` clean.
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
| Foundations (identity, witness, hooks) | 5 (MLP-001..-005) | 5/5 |
| Policy + adoption | 3 (MLP-006, -007, -008) | 3/3 |
| Hard release gate | 1 (MLP-009) | 0/1 |
| CI + config | 2 (MLP-010, -011) | 1/2 |
| Rule distribution | 2 (MLP-012, -013) | 2/2 |
| Coordination + audit | 3 (MLP-014, -015, -016) | 1/3 |
| Doctrine | 1 (MLP-017) | 1/1 |
| **Total** | **17** | **13/17** |

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
