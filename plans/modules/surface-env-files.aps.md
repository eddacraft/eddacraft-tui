<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# `.env` Files Governance Surface (Track 3)

| ID      | Owner | Status |
| ------- | ----- | ------ |
| SURFENV | —     | Draft  |

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

- Secret detection itself (covered by the existing secret scanner).
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
- [ ] ADR-029 Accepted.
- [ ] Secret-scanner contract clear — what SURFENV adds vs. defers.
- [ ] Anvil's own `.env*` files baselined.
- [ ] External codebase validation candidate identified.
- [ ] Owner named.

## Tasks

Anticipated:

- SURFENV-001: File detection (`.env`, `.env.*`, `.envrc`).
- SURFENV-002: `.gitignore` hygiene rules.
- SURFENV-003: Production-value heuristic for non-prod files.
- SURFENV-004: `.env.example` drift detection.
- SURFENV-005: Suppression wiring.
- SURFENV-006: Anvil + external validation runs.

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
