<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Suppressions

| Scope | Owner | Priority | Status |
| ----- | ----- | -------- | ------ |
| SUPP  | —     | medium   | Draft  |

## Purpose

Allow developers to intentionally suppress warnings with accountability. Every
suppression requires a human-written note explaining why, stored both inline and
in structured provenance for audit trails.

## In Scope

- Inline suppression comments with warning ID:
  `// @anvil-ignore <WARNING-ID>: <reason>`
- Suppression provenance (who, when, why, which commit)
- Suppression expiry (optional time-bound suppressions via
  `@anvil-ignore-until`)
- Suppression reporting in drift reports

## Out of Scope

- Auto-suppression (always requires human note)
- Suppression approval workflows (v2)

## Interfaces

**Depends on:**

- `architecture-safety` — boundary warnings to suppress
- `antipattern-library` — anti-pattern warnings to suppress

**Exposes:**

- `SuppressionParser` — extract suppressions from source
- `SuppressionStore` — track suppressions with provenance

## Acceptance Criteria

- [ ] `// @anvil-ignore ARCH-boundary: reason` suppresses that specific warning
- [ ] `// @anvil-ignore-until 2025-06-01 ANTI-any: reason` expires automatically
- [ ] Each suppression requires a non-empty reason (parser rejects empty)
- [ ] Suppressions tracked in `.anvil/suppressions.json` with provenance
- [ ] Drift reports show suppression counts and expiry status

## Tasks

_Tasks to be defined when module status is Ready._
