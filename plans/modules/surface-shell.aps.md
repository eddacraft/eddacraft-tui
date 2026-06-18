<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Shell Scripts Governance Surface (Track 3)

| ID     | Owner      | Status      |
| ------ | ---------- | ----------- |
| SURFSH | joshuaboys | In Progress |

**Last reviewed:** 2026-04-26

> Note (2026-04-26): the existing `command_safety` runtime check lives at
> `crates/anvil-checks/src/command_safety/`. Coordinate rule sharing with that
> crate (one source-of-truth catalogue, two consumers).

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
  - Unquoted variables in destructive contexts (`rm $var`, `mv $a $b`)
  - `eval` on user-controlled input
  - `chmod 777` and equivalent permissive modes
- Suppression syntax: `# @anvil-ignore <ID>: <reason>`.
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
- [`operational-supplement`](./operational-supplement.aps.md) — check
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
      consumers). Shell-only rules (`chmod 777`, pipe-to-shell) are deferred to
      a follow-up that extends the shared catalogue, not SURFSH.
- [x] Anvil's own `.sh` files baselined — corpus scanned; standard scripts
      (scoped `rm`, `set -euo pipefail`) are clean. FP target **N = 1%**.
- [x] External codebase validation candidate identified — a popular OSS repo
      with install/CI shell scripts (final pick in SURFSH-006-validation).
- [x] Owner named — joshuaboys.

## Work Items

Delivered as slices mirroring the other surfaces. T1 (Scanned).

### SURFSH-001 — File detection

- **Status:** Merged 2026-06-18 via PR #2785
- **Intent:** Identify `*.sh`/`*.bash` shell scripts.
- **Expected Outcome:** `*.sh`/`*.bash` (case-insensitive) are detected; other
  files are not. Shebang-only (no-extension) scripts are a documented
  follow-up.
- **Files:** `crates/anvil-checks/src/surface/shell/scanner.rs`
- **Validation:** `cargo test -p eddacraft-anvil-checks --lib surface::shell::scanner::tests::detects_shell_files_by_extension`
- **Confidence:** high

### SURFSH-002 — Dangerous-command scan (shared `command_safety` catalogue)

- **Status:** Merged 2026-06-18 via PR #2785
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

- **Status:** Merged 2026-06-18 via PR #2786
- **Intent:** Surface SURFSH in the gate behind `track.surface.sh`.
- **Expected Outcome:** `ANV-SURF-SH-001` registered + wired (warn-only),
  gated behind a `track.surface.sh` leaf flag under the OPSUP-005
  `track.surface` umbrella, opt-in via `ANVIL_TRACK_SURFACE_SH=1` — the
  SURFSQL-005 pattern.
- **Validation:** `cargo test -p eddacraft-anvil commands::check_catalog`
- **Dependencies:** SURFSH-002, OPSUP-005 (Merged)
- **Confidence:** high

### SURFSH-006-validation — Anvil + external validation runs

- **Status:** Ready
- **Intent:** Prove the acceptance bar (FP < 1% on Anvil + ≥1 external repo).
- **Validation:** FP report committed under `plans/reviews/`.
- **Dependencies:** SURFSH-002, SURFSH-005
- **Confidence:** medium

### Deferred (extend the shared catalogue, don't duplicate)

Pipe-to-shell (`curl … | sh`), `chmod 777`, `eval` on user input, and the
unquoted-variable-in-destructive-context rule are **not** filesystem-command
rules already in `command_safety`. They are deferred to a follow-up that adds
them to the shared `command_safety` ruleset, so both the runtime check and
SURFSH gain them together (the module's "one source of truth" directive).

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Overlap with shellcheck creates "tool fatigue" | Medium | Scope T1 to governance-shaped patterns only; punt linting to shellcheck |
| Unquoted-variable rule too noisy in non-destructive contexts | Medium | Limit to `rm`, `mv`, `cp`, `dd` invocations |
| `command_safety` and SURFSH rule definitions drift | Medium | One source-of-truth catalogue, two consumers |

## Open Questions

- [ ] How is the rule catalogue shared between runtime `command_safety` and
      static SURFSH?
- [ ] T2 promotion (drift baseline + policy hooks) — explicit demand
      required, or fold into Phase 3 scope?
