<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Shell Scripts Governance Surface (Track 3)

| ID     | Owner | Status |
| ------ | ----- | ------ |
| SURFSH | —     | Draft  |

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

Change status to **Ready** when:

- [ ] OPSUP slices landed.
- [ ] ADR-029 Accepted.
- [ ] `command_safety` overlap mapped — no duplicate rule definitions.
- [ ] Anvil's own `.sh` files baselined.
- [ ] External codebase validation candidate identified.
- [ ] Owner named.

## Work Items

Anticipated:

- SURFSH-001: File detection (extension + shebang).
- SURFSH-002: Destructive-command catalogue (rm, chmod, eval).
- SURFSH-003: Pipe-to-shell rule.
- SURFSH-004: Unquoted variable rule (in destructive contexts only — not a
  shellcheck replacement).
- SURFSH-005: Suppression + policy hook wiring.
- SURFSH-006: Anvil + external validation runs.

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
