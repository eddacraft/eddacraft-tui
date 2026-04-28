<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# `.env` Files Governance Surface (Track 3)

| ID      | Owner | Status      | Progress |
| ------- | ----- | ----------- | -------- |
| SURFENV | —     | In Progress | 1/6      |

**Last reviewed:** 2026-04-27

> Hygiene note (2026-04-27): SURFENV-001 has landed in
> `crates/anvil-checks/src/surface/env/` with parser, scanner, ADR-029
> suppression support, and integration tests. The remaining work is the
> structural catalogue and validation pass below.

## Purpose

Bring `.env` files to **T1 (Scanned)** per
[2026-04-08 Language and Coverage Design](../specs/2026-04-08-language-and-coverage-design.md)
§5.2, §8.3 row 5. Demand: 2 confirmed (Anvil + User B), assumed universal.
Blast: critical. Strategic: supports.

Most secret detection is already covered by the existing secret scanner —
this module focuses on the **structural** delta: committed `.env`,
`.gitignore` hygiene, and prod values in non-prod files.

Phase 3 deliverable.

## In Scope

- File detection: `.env`, `.env.*`, `.envrc` (direnv).
- Structural pattern catalogue:
  - `.env` committed to the repo when it should be in `.gitignore`
  - `.env` not present in `.gitignore` (warn, not block)
  - Production-shaped values inside `.env.development` / `.env.local`
    (heuristic — `_PROD`, `prod-`, production-domain hostnames)
  - Conflict between `.env.example` and `.env.*` (drift between template
    and actual)
- Hand-off to existing secret scanner for the actual key detection — do
  not duplicate.

## Out of Scope

- General secret detection outside `.env` parsing (covered by the
  existing secret scanner). SURFENV-001 adds `.env`-aware value
  parsing and routes values through the existing secret patterns.
- Encrypted `.env` formats (`.env.vault`, `dotenv-vault`).
- Pulumi ESC / SOPS / age / Doppler integrations — config-intelligence
  territory.

## Interfaces

**Depends on:**

- Existing secret scanner (consumes its findings; does not duplicate).
- [`operational-supplement`](./operational-supplement.aps.md) — check
  registry, per-track feature flag, file-presence guard.
- Rust suppression parser per
  [ADR-029](../decisions/029-suppression-parser-authority.md) — `.env`
  files use `#` comments (already supported).

**Exposes:**

- Structural `.env` rules.

## Prerequisites

- OPSUP slices landed (see SURFSQL).
- [ADR-029](../decisions/029-suppression-parser-authority.md) Accepted.

## Ready Checklist

Change status to **Ready** when:

- [ ] OPSUP slices landed.
- [x] ADR-029 Accepted.
- [x] Secret-scanner contract clear — what SURFENV adds vs. defers.
- [ ] Anvil's own `.env*` files baselined.
- [ ] External codebase validation candidate identified.
- [ ] Owner named.

## Tasks

### SURFENV-001: `.env` file detection and value scan

- **Intent:** `.env`, `.env.*`, and `.envrc` files are parsed as
  environment files and their values are checked with the existing
  secret patterns.
- **Expected Outcome:** Findings include file, line, variable name,
  redacted value context, and ADR-029 suppression state.
- **Validation:** `cargo test -p eddacraft-anvil-checks -- surfenv`
- **Confidence:** high
- **Status:** Complete

### SURFENV-002: `.gitignore` hygiene rules

- **Intent:** Repositories get actionable warnings when sensitive
  `.env` files are not protected by ignore rules.
- **Status:** Todo

### SURFENV-003: Production-value heuristic for non-prod files

- **Intent:** Development env files flag production-shaped values with
  conservative defaults.
- **Status:** Todo

### SURFENV-004: `.env.example` drift detection

- **Intent:** Template env files and concrete env files report
  structural drift without duplicating secret detection.
- **Status:** Todo

### SURFENV-005: Structural rule suppression wiring

- **Intent:** Structural SURFENV findings use the Rust ADR-029
  suppression parser consistently.
- **Status:** Todo

### SURFENV-006: Anvil and external validation runs

- **Intent:** Validate SURFENV findings against this repository and
  one external candidate before broadening the surface.
- **Status:** Todo

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Production-value heuristic FP-noisy | Medium | Conservative defaults; allow opt-out via per-key allowlist |
| `.envrc` (direnv) often legitimately committed | Medium | Different rule set for `.envrc` than for `.env` |
| Duplicates secret scanner | High | Strict scope: structural concerns only, secret detection stays in scanner |

## Open Questions

- [ ] Should `.env.example` be required to exist when `.env*` is in the
      repo?
- [ ] How to surface combined findings when secret scanner and SURFENV both
      hit the same file?
