<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# `.env` Files Governance Surface (Track 3)

| ID      | Owner | Status   | Progress |
| ------- | ----- | -------- | -------- |
| SURFENV | —     | Complete | 6/6      |

**Last reviewed:** 2026-04-29

> Hygiene note (2026-04-29): All six SURFENV slices have landed in
> `crates/anvil-checks/src/surface/env/`. The structural catalogue is
> wired through the shared `suppression` helper (ADR-029), each rule
> has unit + integration coverage, and a baseline test pins the anvil
> repo to a clean state so regressions surface in CI.

## Purpose

Bring `.env` files to **T1 (Scanned)** per
[2026-04-08 Language and Coverage Design](../../specs/2026-04-08-language-and-coverage-design.md)
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
- [`operational-supplement`](operational-supplement.aps.md) — check
  registry, per-track feature flag, file-presence guard.
- Rust suppression parser per
  [ADR-029](../../decisions/029-suppression-parser-authority.md) — `.env`
  files use `#` comments (already supported).

**Exposes:**

- Structural `.env` rules.

## Prerequisites

- OPSUP slices landed (see SURFSQL).
- [ADR-029](../../decisions/029-suppression-parser-authority.md) Accepted.

## Ready Checklist

Change status to **Ready** when:

- [ ] OPSUP slices landed.
- [x] ADR-029 Accepted.
- [x] Secret-scanner contract clear — what SURFENV adds vs. defers.
- [x] Anvil's own `.env*` files baselined (see
      `crates/anvil-checks/tests/surfenv_anvil_baseline.rs`).
- [x] External codebase validation candidate exercised via the
      synthetic-repo smoke test in the same file; a real third-party
      candidate is the next follow-up if SURFENV gains a CLI surface
      (Phase 4 work).
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
- **Expected Outcome:** Each unprotected `.env` file produces one
  finding with a suggested gitignore pattern; intentionally-committed
  filenames (`.env.example`, `.envrc`, …) are skipped; file-header
  `# @anvil-ignore SURFENV-002` directives suppress.
- **Validation:** `cargo test -p eddacraft-anvil-checks --test surfenv_gitignore_hygiene`
- **Status:** Complete

### SURFENV-003: Production-value heuristic for non-prod files

- **Intent:** Development env files flag production-shaped values with
  conservative defaults.
- **Expected Outcome:** Three indicators fire on non-prod env
  filenames (`production` word at boundaries, `prod-` host segment in
  multi-part hostnames, `_PROD` key suffix) with staging/local/test
  short-circuits; line-level `# @anvil-ignore SURFENV-003` directives
  suppress.
- **Validation:** `cargo test -p eddacraft-anvil-checks --test surfenv_prod_value`
- **Status:** Complete

### SURFENV-004: `.env.example` drift detection

- **Intent:** Template env files and concrete env files report
  structural drift without duplicating secret detection.
- **Expected Outcome:** Per-key findings in both directions
  (`MissingFromExample`, `MissingFromConcrete`); file-header
  `# @anvil-ignore SURFENV-004` directives suppress on the relevant
  side without leaking across directions.
- **Validation:** `cargo test -p eddacraft-anvil-checks --test surfenv_drift`
- **Status:** Complete

### SURFENV-005: Structural rule suppression wiring

- **Intent:** Structural SURFENV findings use the Rust ADR-029
  suppression parser consistently.
- **Expected Outcome:** Every rule routes through the shared
  `surface::env::suppression` helpers (`resolve_line_suppression`,
  `resolve_file_header_suppression`); cross-rule audit test enforces
  that a directive for one rule never silences another.
- **Validation:** `cargo test -p eddacraft-anvil-checks --test surfenv_suppression_audit`
- **Status:** Complete

### SURFENV-006: Anvil and external validation runs

- **Intent:** Validate SURFENV findings against this repository and
  one external candidate before broadening the surface.
- **Expected Outcome:** Baseline test runs all four rules against
  anvil's committed `.env*` files (currently template-only) and a
  synthetic external repo; both must produce zero unsuppressed
  findings on anvil and the expected interaction findings on the
  synthetic case. A regression flips the test red and names the file
  + rule that drifted.
- **Validation:** `cargo test -p eddacraft-anvil-checks --test surfenv_anvil_baseline`
- **Status:** Complete

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
