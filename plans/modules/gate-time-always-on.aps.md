<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Gate-time catalogue on always-on surfaces

| ID   | Owner  | Status   | Progress |
| ---- | ------ | -------- | -------- |
| GTAO | @aneki | Proposed | 0/10     |

**Last reviewed:** 2026-08-22 — created from operator direction after the
regex/AST and “when do gates fire” discussion. Agreed shape: do not auto-run
full `anvil gate` on save; do auto-run the cheap gate-time catalogue (AST +
planless antipatterns) on surfaces that already fire without a human
remembering; keep the named gate as the merge judgement at commit/CI.
Same day: operator added a bounded **Python AST** slice for shapes the PY-*
regex catalogue documents as blind — not a second language programme, and not
a conversion of save-time regex rules to AST.

## Purpose

The AST catalogue (`RS-001`…`008`) and any other `Detection::Ast` rule only run
on CLI `anvil check` / `anvil gate`. The golden path after `anvil start` is
daemon-backed save-time plus MCP `anvil_validate_write`. Those paths are
regex-only. Agents that call MCP `anvil_check` or planless `anvil_gate` also
skip AST. Local `git commit` runs the full gate only if hooks actually
installed. The shipped GitHub Action template runs L4 pre-push, not
`anvil gate`.

Result: the higher-fidelity rules are dark on the always-on path, and the merge
gate is opt-in. This module closes that coverage gap **without** putting
tree-sitter in the daemon, **without** running lint/test/coverage/audit on
every save, and **without** defaulting `anvil watch --action gate`.

## In Scope

- An ADR that records the hot/non-hot split: always-on path runs the cheap
  catalogue (regex + AST antipatterns on changed files); full `anvil gate`
  stays a workflow event (commit hook, CI, explicit CLI/MCP).
- MCP `anvil_check` and planless `anvil_gate` merge the AST tier the same way
  CLI `anvil check` / `anvil gate` already do (ADR-071 §7 completeness).
- After a daemon-allowed write, a background CLI subprocess runs changed-path
  `anvil check` (regex + AST). The save/pre-write verdict is unchanged and must
  not wait for it.
- Findings from that follow-up are visible on an existing surface (status /
  watch / observation) without blocking the write, under ADR-038 noise
  discipline.
- Resource-budget proof the follow-up does not recreate the ADR-061 save-storm
  (whole-repo cold scan per keystroke).
- Adopter CI template gains an `anvil gate --profile ci` merge job; L4
  pre-push remains a distinct layer.
- Docs and shipped skills state honestly when regex, AST, and the full gate
  run.
- Language dispatch in `anvil-checks-ast` (today hardcoded to
  `rust_language()`) so `.py` files can run `Detection::Ast` rules.
- Additive Python AST rules for shapes the current regex catalogue names as
  blind: block-body `except …: pass` (PY-004), eval/exec/compile forms PY-008
  cannot see, `yaml.load` loader distinction (PY-009). Regex PY-* stay on the
  save-time path.

## Out of Scope

- Linking tree-sitter or `anvil-checks-ast` into the resident daemon (ADR-064).
- Running full `anvil gate` (lint, test, coverage, dependency, policy, npm
  audit) on save, MidEdit, or `anvil_validate_write`.
- Changing the default of `anvil watch` from `check` to `gate`.
- Raising AST `RS-*` or new `PY-01x` severity so they fail the gate (separate
  product call). Default the new Python AST rules to the same `info`/`warning`
  posture the FP bar requires; do not silently promote PY-008's `error` onto
  AST-only shapes without a dogfood pass.
- Converting existing regex PY-001…009 to `detection: ast`. That would darken
  them on save-time. Comment-level PY-001/002/003 stay regex forever.
- TypeScript AST catalogues, mutable-default-arg, taint, or import-resolved
  alias tracking beyond a local `from builtins import eval as X` in-file bind.
- Replacing L4 pre-push / witness with the quality gate.
- Taint analysis or SAST (ADR-087).

## Interfaces

**Depends on:**

- [ADR-061](../decisions/061-save-time-daemon-delta-validation.md) — save-time
  is daemon-mediated delta validation; whole-repo child `anvil check` per save
  is the failure mode this module must not revive.
- [ADR-064](../decisions/064-intercept-graph-cache-crate-boundary.md) — daemon
  links no tree-sitter.
- [ADR-067](../decisions/067-daemon-symbol-feed-parse-hook.md) — CLI subprocess
  / parse-hook precedent for expensive work off the daemon crate.
- [ADR-071](../decisions/071-ast-aware-antipattern-detection.md) — AST is a
  gate-time tier; CLI check/gate already merge both scanners.
- [ADR-031](../decisions/031-validation-latency-rubric.md) — interactive
  save/pre-write p95 budgets stay regex-only.
- [ADR-038](../decisions/038-hook-surface-and-noise-discipline.md) — silent on
  success; one line on warn.
- [RLB](./resource-load-benchmarking.aps.md) — per-save CPU budgets.
- [CIB-294](./continuous-improvement-backlog.aps.md) — adopter CI templates
  currently unexercised; GTAO-006 coordinates, does not steal.
- [lang-python](./lang-python.aps.md) — T3 Python catalogue and grammar; GTAO
  does not reopen PYLAN items.
- CIB-332 known limitation (byte-level `compile` receiver) and the PY-004 /
  PY-008 / PY-009 rule-body blind spots named in those `.anvil` files.

**Exposes:**

- Always-on AST (and regex) coverage on MCP check, planless MCP gate, and a
  background follow-up after daemon-allowed writes.
- A CI merge-judgement path that is `anvil gate --profile ci`, distinct from
  L4.
- Python AST rules for regex-blind shapes, on the same gate-time crate.

## Waves

| Wave | Items                        | Gate                                                              |
| ---- | ---------------------------- | ----------------------------------------------------------------- |
| 1    | GTAO-001, GTAO-002, GTAO-008 | ADR indexed; MCP emits Rust AST; Python dispatch + PY-010 proof   |
| 2    | GTAO-003, GTAO-005           | Background follow-up runs and stays inside budget                 |
| 3    | GTAO-004, GTAO-006, GTAO-007 | Findings visible; CI template ships the merge gate; docs match    |
| 4    | GTAO-009, GTAO-010           | PY-011 / PY-012 cover the PY-008 / PY-009 regex blinds            |

## Work Items

### GTAO-001: ADR — always-on cheap catalogue, rare full gate

- **Status:** In Progress
- **Intent:** Record the product split so later items cannot revive a per-save
  full gate or put tree-sitter in the daemon.
- **Expected Outcome:** An ADR (next free number) is Proposed, indexed in
  `DECISION-LOG.md`, and accepted by the operator. It states: (1) always-on
  surfaces run the cheap catalogue (regex antipatterns + AST) on changed files
  only; (2) that work runs in a CLI/`anvil-checks-ast` process, never in
  `anvil-intercept`; (3) it must not delay the interactive verdict or violate
  ADR-031; (4) full `anvil gate` remains commit / CI / explicit invocation;
  (5) `anvil watch --action gate` stays opt-in; (6) CI adopter template should
  run `anvil gate --profile ci` *in addition to* L4, not instead of it; (7) a
  kill switch exists for the background follow-up given the ADR-061 load
  history; (8) Python AST is additive companion rules (new PY-01x ids) on the
  existing `anvil-checks-ast` crate via language dispatch — it does **not**
  convert regex PY-001…009 to AST (that would darken save-time) and does not
  wait on a new crate. Alternatives considered include defaulting watch to
  gate, feeding trees into the daemon, and a feature flag on `anvil-checks`
  (rejected under ADR-064 unification).
- **Validation:** `pnpm adr:check`
- **Files:** `plans/decisions/`, `plans/decisions/DECISION-LOG.md`
- **Dependencies:** —
- **Confidence:** high — operator approved the shape in-session; the ADR
  writes it down.

### 1. File the ADR and log row

- **Checkpoint:** ADR exists, indexed, `pnpm adr:check` clean
- **Validate:** `pnpm adr:check`

### GTAO-002: MCP `anvil_check` and planless `anvil_gate` merge the AST tier

- **Status:** In Progress
- **Intent:** Agents that already call the MCP check/gate tools see the same
  AST findings CLI `anvil check` already emits.
- **Expected Outcome:** MCP `anvil_check` and planless `anvil_gate`
  (`targetFiles` set) run `anvil-checks-ast` alongside `run_antipattern_check`
  and merge deterministically, matching CLI `anvil check`. A `.rs` fixture that
  trips `RS-001` is clean on regex and present in the MCP payload. Non-Rust
  files are unchanged. The daemon dep-boundary test still asserts no
  tree-sitter in `anvil-intercept`. Interactive `anvil_validate_write` is
  **not** changed here.
- **Validation:** `cargo test -p eddacraft-anvil --no-fail-fast mcp_serve_stdio -- --nocapture` and a focused test that MCP check on a Rust unwrap fixture emits `RS-001`
- **Files:** `crates/anvil-cli/src/mcp/tools/check.rs`,
  `crates/anvil-cli/src/mcp/tools/gate.rs`,
  `crates/anvil-cli/src/mcp/tools/shared.rs`,
  `crates/anvil-cli/tests/mcp_serve_stdio.rs`,
  `crates/anvil-intercept/tests/daemon_dep_boundary.rs`
- **Dependencies:** —
- **Confidence:** high — CLI merge already exists; this is the missing caller.
  Independent of GTAO-001 (ADR-071 §7 already requires both scanners on
  check/gate).
- **Identified From:** ADR-071 §7 vs
  `crates/anvil-cli/src/mcp/tools/{check,gate}.rs` (planless path calls
  `run_antipattern_check` only; no `anvil_checks_ast`).

### 1. Merge AST into both MCP planless scanners

- **Checkpoint:** MCP check of a Rust unwrap fixture returns `RS-001`
- **Validate:** `cargo test -p eddacraft-anvil --no-fail-fast mcp_serve_stdio`

### GTAO-003: Background changed-path check after daemon-allowed write

- **Status:** Draft
- **Intent:** Save and pre-write stay fast and regex-only; AST still runs
  automatically on the files that just changed.
- **Expected Outcome:** After `scan_buffer` / `validate_paths` returns, the
  CLI (not the daemon crate) schedules a changed-path `anvil check` subprocess
  that includes the AST tier. The interactive verdict does not wait. Duplicate
  bursts coalesce (one follow-up per path set). Failure of the follow-up is
  fail-safe: the original allow/warn stands, a single skipped diagnostic is
  recorded, the process still exits 0. `daemon_dep_boundary` remains green.
  Kill switch from GTAO-001 honoured.
- **Validation:** `cargo test -p eddacraft-anvil-intercept --no-fail-fast` plus a CLI/integration test that an allowed `.rs` write produces a later AST finding without changing the save verdict
- **Files:** `crates/anvil-cli/src/`, `crates/anvil-intercept/` (schedule hook
  only; no tree-sitter dep), `crates/anvil-checks-ast/`
- **Dependencies:** GTAO-001
- **Confidence:** medium — subprocess and coalesce details are the ADR's job;
  the load risk is real.

### 1. Follow-up does not block the verdict

- **Checkpoint:** Save verdict latency unchanged when follow-up enabled
- **Validate:** existing ADR-031 save-time benches still pass

### 2. Follow-up sees AST on the changed file

- **Checkpoint:** Allowed `.rs` unwrap is reported by the follow-up, not the verdict
- **Validate:** new integration test named in the item Validation field

### GTAO-004: Surface background findings without blocking the write

- **Status:** Draft
- **Intent:** Operators can see AST follow-up findings without a terminal wash
  or a blocked save.
- **Expected Outcome:** Follow-up findings appear on one existing surface
  (watch/status observation or equivalent), silent on empty, one terse line on
  warn per ADR-038, no colour escalation. They are tagged as the AST/follow-up
  tier so “save was green” is not readable as “fully clean”. No new TUI
  product.
- **Validation:** `cargo test -p eddacraft-anvil --no-fail-fast` for the chosen surface plus a golden/terse-output pin
- **Files:** `crates/anvil-cli/src/commands/watch.rs`,
  `crates/anvil-cli/src/commands/status.rs` (or the observation sink the ADR
  names)
- **Dependencies:** GTAO-003
- **Confidence:** medium — surface choice is left to the ADR; noise discipline
  is load-bearing.

### GTAO-005: Resource-budget proof the follow-up does not recreate the save-storm

- **Status:** Draft
- **Intent:** Prove the background check stays on changed paths, coalesces,
  and does not return to ~7-core per-save scans.
- **Expected Outcome:** A bench or load-probe assertion shows the follow-up is
  O(changed files), coalesced under a save burst, and host CPU stays inside
  the RLB watch/intercept budgets. A regression that shells `anvil check --all`
  or `anvil gate` per save fails the bench. Documented in the ADR
  consequences.
- **Validation:** `cargo bench -p eddacraft-anvil-bench --bench watch_resource_budget` (or the RLB process-tree probe the implementer names in the PR, pinned as a CI-visible test where flakiness allows)
- **Files:** `crates/anvil-bench/`, `crates/anvil-intercept/benches/`
- **Dependencies:** GTAO-003
- **Confidence:** medium — budgets exist; wiring a non-flaky assertion onto an
  async follow-up is the work.
- **Coordinates with:** RLB-007 (changed-path scoping), ADR-061 context.

### GTAO-006: Adopter CI template runs `anvil gate --profile ci`

- **Status:** Draft
- **Intent:** Pull-request automation, not a human or agent, is what runs the
  full merge gate.
- **Expected Outcome:** `crates/anvil-cli/src/templates/anvil-workflow.yml`
  (and the audit sibling if it shares the same hole) ships a job that runs
  `anvil gate --profile ci` on pull_request. The existing L4
  `anvil hook pre-push` job remains a separate layer. Template comments state
  the two jobs are not substitutes. This module does not require anvil's own
  repo to adopt the template (that is CIB-294); it requires the template we
  hand to adopters to actually run the gate.
- **Validation:** a fixture or snapshot test of the template contains `anvil gate --profile ci` and still contains the L4 pre-push step; `rg -n "anvil gate --profile ci" crates/anvil-cli/src/templates/anvil-workflow.yml`
- **Files:** `crates/anvil-cli/src/templates/anvil-workflow.yml`,
  `crates/anvil-cli/src/templates/anvil-audit-workflow.yml`,
  `crates/anvil-cli/src/commands/anvil_action.rs`
- **Dependencies:** GTAO-001
- **Confidence:** high — mechanical once the ADR says “in addition to L4”.
- **Coordinates with:** CIB-294 (templates unexercised in anvil's own CI — do
  not close CIB-294 from this item).

### GTAO-007: Honest “when it runs” docs and shipped skills

- **Status:** Draft
- **Intent:** Stop teaching that save-time green means the AST catalogue ran,
  or that someone must remember `anvil gate` for cheap rules.
- **Expected Outcome:** Public evaluation-model, `using-anvil` /
  developer-functions shipped skills, and CLI help distinguish: (1)
  pre-write/save-time = regex + secrets, interactive budget; (2) MCP
  `anvil_check` / CLI `anvil check` = regex + AST; (3) background follow-up =
  AST after allow, non-blocking; (4) `anvil gate` / commit hook / CI = merge
  judgement. Skills tell agents to call `anvil_check` (or CLI
  `anvil check --changed`) before claiming done, and `anvil_gate` only when
  the user asked whether work can merge. No claim that default watch runs the
  full gate.
- **Validation:** `pnpm docs:check`
- **Files:** `docs/public/anvil/concepts/evaluation-model.md`,
  `docs/public/anvil/reference/what-anvil-can-do.md`,
  `crates/anvil-cli/assets/skills/using-anvil/SKILL.md`,
  `crates/anvil-cli/assets/skills/anvil-developer-functions/SKILL.md`,
  `docs/architecture/quality-model.md`
- **Dependencies:** GTAO-001
- **Confidence:** high

### GTAO-008: Python language dispatch + PY-010 except-block swallow

- **Status:** In Progress
- **Intent:** Make `anvil-checks-ast` language-general and prove it with the
  one Python shape PY-004 already documents as regex-blind.
- **Expected Outcome:** `anvil-checks-ast` selects tree-sitter language from
  the file extension (Rust and Python at minimum; unknown extensions skip).
  Queries compile against the matching grammar. Registry `Detection::Ast`
  rules carry `file_extensions` as they do today. New additive rule **PY-010**
  (`detection: ast`) fires on a named `except` handler whose *block body is
  only `pass`* (`except Exception:\n    pass`), which PY-004's line regex
  cannot see. Inline `except Exception: pass` remains PY-004 (regex,
  save-time). Bare `except:` remains PY-004. A handler that logs or re-raises
  is clean. Completeness guard covers PY-010. `tree-sitter-python` is the
  workspace pin already used by the kernel; this crate depends on it
  directly, not via `anvil-kernel`. Daemon dep-boundary stays green.
- **Validation:** `cargo test -p eddacraft-anvil-checks-ast --no-fail-fast`
- **Files:** `crates/anvil-checks-ast/src/lib.rs`,
  `crates/anvil-checks-ast/src/predicates.rs`,
  `crates/anvil-checks-ast/Cargo.toml`,
  `patterns/python-reliability/`, `patterns/compiled/registry.json`
- **Dependencies:** —
- **Confidence:** high — grammar is in-tree; PY-004's own rule body names this
  gap; RSTLAN first-slice pattern (dispatch + one predicate).
- **Identified From:** PY-004.anvil “named handler whose `pass` sits on the
  *next* line … needs block context this regex tier does not have.”

### 1. Dispatch Python and flag a block-body `pass` swallow

- **Checkpoint:** PY-010 fires on multiline `except Exception: pass`, not on regex PY-004's inline form as a duplicate
- **Validate:** `cargo test -p eddacraft-anvil-checks-ast --no-fail-fast`

### GTAO-009: PY-011 AST companions for PY-008 regex blinds

- **Status:** Draft
- **Intent:** Catch dynamic `eval` / `exec` / `compile` shapes PY-008
  documents as unseeable, without taking PY-008 off save-time.
- **Expected Outcome:** Additive **PY-011** (`detection: ast`) fires on: a
  first argument that is concatenation, formatting, an f-string, or a call
  whose opening `(` and first argument span lines; and on an in-file alias
  `from builtins import eval as X` / `compile as X` used as a call. It does
  **not** fire on `re.compile`, `torch.compile`, or `self.compile` (CIB-332
  stays). It **does** fire on `builtins.compile` / `__builtins__.compile` of a
  dynamic argument (the byte-level gate cannot tell those from a random
  `.compile` without a special regex arm). PY-008 regex remains default-on
  `error` at save-time. PY-011 default posture is decided against the PYLAN
  FP bar at authoring time (likely `warning` until dogfood). No import-graph
  resolution of `compile` aliases from other modules.
- **Validation:** `cargo test -p eddacraft-anvil-checks-ast --no-fail-fast` plus the existing `crates/anvil-checks/tests/python_antipatterns.rs` PY-008 cases still green
- **Files:** `crates/anvil-checks-ast/src/`,
  `patterns/python-reliability/PY-011.anvil`,
  `patterns/compiled/registry.json`,
  `crates/anvil-checks/tests/python_antipatterns.rs`
- **Dependencies:** GTAO-008
- **Confidence:** medium — argument-shape predicates are where FP lives.
- **Identified From:** PY-008.anvil blinds (concat, parenthesised, later-line
  first arg) and CIB-332 “syntactic fix needs the AST tier”.

### GTAO-010: PY-012 AST companion for PY-009 `yaml.load` loader

- **Status:** Draft
- **Intent:** Distinguish unsafe `yaml.load` from a same-call SafeLoader,
  which the regex tier cannot do.
- **Expected Outcome:** Additive **PY-012** (`detection: ast`) fires on
  `yaml.load(` / `yaml.full_load(` without a `Loader=` that is `SafeLoader` /
  `CSafeLoader`, and on `pickle.loads` reached via `from pickle import loads`.
  `yaml.safe_load(` stays clean. Same-line `yaml.load(src, Loader=SafeLoader)`
  is clean on PY-012 (PY-009 regex may still fire — documented, suppressible;
  this item does not rewrite PY-009). `os.system` / `shell=True` stay regex
  PY-009.
- **Validation:** `cargo test -p eddacraft-anvil-checks-ast --no-fail-fast`
- **Files:** `crates/anvil-checks-ast/src/`,
  `patterns/python-reliability/PY-012.anvil`,
  `patterns/compiled/registry.json`
- **Dependencies:** GTAO-008
- **Confidence:** medium
- **Identified From:** PY-009.anvil “the regex tier does not distinguish
  loader arguments.”

---

GTAO-001, GTAO-002, and GTAO-008 are Ready by in-session operator approval.
Remaining items stay Draft until the ADR lands (GTAO-003) or the Python
dispatch proof (GTAO-008) is green (GTAO-009/-010). Do not start GTAO-003 on a
feature branch before GTAO-001 is Accepted. Do not convert PY-008 to
`detection: ast`.
