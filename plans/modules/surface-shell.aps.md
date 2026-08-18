<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Shell Scripts Governance Surface (Track 3)

| ID     | Owner      | Status      |
| ------ | ---------- | ----------- |
| SURFSH | joshuaboys | In Progress |

**Last reviewed:** 2026-08-17

> Note (2026-04-26): the existing `command_safety` runtime check lives at
> `crates/anvil-checks/src/command_safety/`. Coordinate rule sharing with that
> crate (one source-of-truth catalogue, two consumers).
>
> Note (2026-08-17): SURFSH-008 design accepted —
> [2026-08-17-surfsh-008-shell-catalogue.md](../specs/2026-08-17-surfsh-008-shell-catalogue.md).
> Unquoted-vars parked as SURFSH-009.

## Purpose

Bring `.sh` scripts to **T1 (Scanned)** per
[2026-04-08 Language and Coverage Design](../specs/2026-04-08-language-and-coverage-design.md)
§5.2, §8.3 row 4. Demand: 2 (Anvil + User B). Blast: high. Strategic: supports.

Extends the existing `command_safety` runtime check to static analysis of
checked-in `.sh` files.

Phase 3 deliverable.

## In Scope

- File detection: `*.sh`, `*.bash`, files with shell shebang
  (`#!/bin/sh`, `#!/bin/bash`, `#!/usr/bin/env bash`).
- Pattern catalogue (per spec §8.3 row 4):
  - `rm -rf /` and variants reaching `/`
  - `curl … | sh` / `wget … | sh` install one-liners
  - Unquoted variables in destructive contexts (`rm $var`, `mv $a $b`) — SURFSH-009
  - `eval` on user-controlled input
  - `chmod 777` and equivalent permissive modes
- Suppression syntax: `# @anvil-ignore <ID> -- <reason>`.
- Reuses pattern logic from the existing `command_safety` runtime check
  where applicable — do not duplicate.

## Out of Scope

- Full shell parsing / shellcheck replacement (shellcheck stays
  authoritative for shell linting).
- Zsh/Fish-specific patterns.
- Makefile / Justfile recipes (recipes inside those files use shell, but
  the wrapping format is different — separate surface if demand arrives).

## Interfaces

**Depends on:**

- Existing `command_safety` runtime check — share rule definitions.
- [`operational-supplement`](../archive/modules/operational-supplement.aps.md) — check
  registry, per-track feature flag, file-presence guard.
- Rust suppression parser per
  [ADR-029](../decisions/029-suppression-parser-authority.md) — `#`
  comment style.

**Exposes:**

- Static `.sh` pattern catalogue.

## Prerequisites

- OPSUP slices landed (see SURFSQL).
- [ADR-029](../decisions/029-suppression-parser-authority.md) Accepted.

## Ready Checklist

Promoted Draft → In Progress 2026-06-18. Checklist satisfied:

- [x] OPSUP slices landed — same set as the other surfaces (OPSUP-001/-003/-005).
- [x] ADR-029 Accepted — `#` comment style already in the suppression parser.
- [x] `command_safety` overlap mapped — **no duplicate rule definitions**:
      SURFSH reuses `command_safety::{parse_compound_command, analyse_command}`
      against the shared `default_filesystem_rules()` (one catalogue, two
      consumers). Shell-only rules were deferred to SURFSH-008; that item is
      now Ready against the accepted 2026-08-17 spec.
- [x] Anvil's own `.sh` files baselined — corpus scanned; standard scripts
      (scoped `rm`, `set -euo pipefail`) are clean. FP target **N = 1%**.
- [x] External codebase validation candidate identified — a popular OSS repo
      with install/CI shell scripts (final pick in SURFSH-006-validation).
- [x] Owner named — joshuaboys.

## Work Items

Delivered as slices mirroring the other surfaces. T1 (Scanned).

### SURFSH-001 — File detection

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-18 via PR #2785
- **Intent:** Identify `*.sh`/`*.bash` shell scripts.
- **Expected Outcome:** `*.sh`/`*.bash` (case-insensitive) are detected; other
  files are not. Shebang-only (no-extension) scripts are a documented
  follow-up.
- **Files:** `crates/anvil-checks/src/surface/shell/scanner.rs`
- **Validation:** `cargo test -p eddacraft-anvil-checks --lib surface::shell::scanner::tests::detects_shell_files_by_extension`
- **Confidence:** high

### SURFSH-002 — Dangerous-command scan (shared `command_safety` catalogue)

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-18 via PR #2785
- **Intent:** Flag dangerous commands in checked-in shell scripts **without
  duplicating** the `command_safety` catalogue.
- **Expected Outcome:** Each command (incl. compound `&&`/`|`/`;` parts, with
  `\` line-continuation assembly) is parsed and analysed against the shared
  `default_filesystem_rules()`; Block/Warn results surface as warn-only
  findings with `#` suppression. The `rm -rf /` family is covered today via
  the shared catalogue.
- **Files:** `crates/anvil-checks/src/surface/shell/{scanner,check}.rs`
- **Validation:** `cargo test -p eddacraft-anvil-checks --lib surface::shell`
- **Confidence:** high

### SURFSH-005 — Gate/catalogue registration + flag gating

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-18 via PR #2786
- **Intent:** Surface SURFSH in the gate behind `track.surface.sh`.
- **Expected Outcome:** `ANV-SURF-SH-001` registered + wired (warn-only),
  gated behind a `track.surface.sh` leaf flag under the OPSUP-005
  `track.surface` umbrella, opt-in via `ANVIL_TRACK_SURFACE_SH=1` — the
  SURFSQL-005 pattern.
- **Validation:** `cargo test -p eddacraft-anvil commands::check_catalog`
- **Dependencies:** SURFSH-002, OPSUP-005 (Merged)
- **Confidence:** high

### SURFSH-008 — Extend the shared `command_safety` catalogue (shell-only rules)

- **Status:** In Progress
- **Pull Request:** [#3984](https://github.com/eddacraft/anvil-001/pull/3984)
- **Design:** [2026-08-17-surfsh-008-shell-catalogue.md](../specs/2026-08-17-surfsh-008-shell-catalogue.md)
  (approved 2026-08-17)
- **Intent:** Add the shell-only governance patterns SURFSH-002 cannot cover
  from `default_filesystem_rules()`: pipe-to-shell, dynamic `eval`, and
  numeric `chmod 777` / `0777`.
- **Expected Outcome:** Shared catalogue grows a `default_shell_rules()` pack
  plus one `analyse_compound` helper. Runtime command-safety **Blocks**
  `pipe-to-shell` and **Warns** on `eval-dynamic` and `chmod-777`. SURFSH stays
  warn-only and picks the rules up through the helper — no SURFSH-only
  duplicate matcher. Unquoted-vars are out of this item (SURFSH-009).
- **Files:** `crates/anvil-checks/src/command_safety/`,
  `crates/anvil-checks/src/surface/shell/{scanner,check}.rs`,
  `crates/anvil-checks/tests/command_safety_validation.rs`,
  `scripts/agent/guidance.sh`,
  `scripts/cache/anvil-target-evict.test.sh`
- **Validation:** `cargo test -p eddacraft-anvil-checks --lib command_safety`
  and `cargo test -p eddacraft-anvil-checks --lib surface::shell` and
  `cargo test -p eddacraft-anvil-checks --test command_safety_validation`
  plus an Anvil + ripgrep FP re-check via
  `anvil gate --only-checks shell-scripts --format json` (target still < 1% FP).
- **Dependencies:** SURFSH-002 (Released/Shipped)
- **Confidence:** high — design accepted; remaining risk is FP on the new
  eval/chmod/pipe rules, gated by the corpus re-check.
- **Evidence:** 2026-08-18 — Council BLOCK repairs (`sh -c` identity,
  `2>&1`/`|&`, trailing-`|` join, docs-owed, override honesty).
  `cargo test -p eddacraft-anvil-checks --lib command_safety`;
  `--lib surface::shell`; `--test command_safety_validation`.
  2026-08-18 — post-merge analyser overgrowth reverted to that designed
  contract (`55da15ea4`); rollback wording (`--skip-checks=command-safety`,
  `# @anvil-ignore SURFSH-002 -- <reason>`) kept.

### 1. Shared shell catalogue and compound helper exist

- **Checkpoint:** `default_shell_rules` plus `analyse_compound` are the only matchers
- **Validate:** `cargo test -p eddacraft-anvil-checks --lib command_safety`

### 2. Runtime and SURFSH consume the same helper

- **Checkpoint:** Both consumers flag pipe-to-shell, dynamic eval, and chmod 777
- **Validate:** `cargo test -p eddacraft-anvil-checks --lib surface::shell`

### 3. Dogfood evals suppressed; FP bar still holds

- **Checkpoint:** Anvil eval sites suppressed; corpus FP still under 1%
- **Validate:** `anvil gate --only-checks shell-scripts --format json`

### SURFSH-009 — Unquoted variables in destructive contexts

- **Status:** Draft
- **Intent:** Flag unquoted expansions in destructive commands (`rm $var`,
  `mv $a $b`) without blowing the 1% FP bar.
- **Expected Outcome:** Parser preserves quote status so `rm $var` fires and
  `rm "$var"` does not; scoped to `rm` / `mv` / `cp` / `dd`. Shared catalogue,
  both consumers.
- **Files:** `crates/anvil-checks/src/command_safety/parser.rs`,
  `crates/anvil-checks/src/command_safety/`
- **Validation:** unit tests for quoted vs unquoted destructive args; FP
  re-check on Anvil + ripgrep.
- **Dependencies:** SURFSH-008
- **Confidence:** medium — needs quote tracking the current tokenizer strips.

### SURFSH-006-validation — Anvil + external validation runs

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-18 via PR #2791
- **Intent:** Prove the acceptance bar (FP < 1% on Anvil + ≥1 external repo).
- **Expected Outcome:** Validated 2026-06-18 — Anvil (110 in-scope shell
  scripts, 0 findings) + `BurntSushi/ripgrep` (2 scripts, 0 findings),
  **0% FP → PASS**. Evidence:
  `plans/reviews/2026-06-18-surface-validation.md`. (No dangerous-command
  corpus in either repo — external true-positive confirmation is light; unit
  tests cover detection.)
- **Validation:** FP report committed under `plans/reviews/`.
- **Dependencies:** SURFSH-002, SURFSH-005
- **Confidence:** medium

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Overlap with shellcheck creates "tool fatigue" | Medium | Scope T1 to governance-shaped patterns only; punt linting to shellcheck |
| Unquoted-variable rule too noisy in non-destructive contexts | Medium | Parked as SURFSH-009; limit to `rm`, `mv`, `cp`, `dd` |
| `command_safety` and SURFSH rule definitions drift | Medium | One source-of-truth catalogue, two consumers |
| New Block rule on hard-pinned command-safety | Medium | Pipe-to-shell only; eval/chmod stay Warn; per-rule overrides remain |

## Open Questions

- [x] How is the rule catalogue shared between runtime `command_safety` and
      static SURFSH? — Resolved 2026-08-17: one `analyse_compound` helper plus
      `default_shell_rules()`; see
      [2026-08-17-surfsh-008-shell-catalogue.md](../specs/2026-08-17-surfsh-008-shell-catalogue.md).
- [ ] T2 promotion (drift baseline + policy hooks) — explicit demand
      required, or fold into Phase 3 scope?
