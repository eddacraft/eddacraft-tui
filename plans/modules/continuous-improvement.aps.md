<!--
APS Module: Continuous Improvement
====================================
Ongoing refactoring, code quality, shared libraries, and generators.
See: plans/aps-rules.md
-->

# Continuous Improvement

| ID  | Owner | Status |
| --- | ----- | ------ |
| CI  | —     | Draft |

## Purpose

Ongoing improvement of code quality, developer experience, and codebase
organisation. Covers refactoring, code reviews, shared library extraction,
utility consolidation, Nx generator creation, and technical debt tracking.

**Why a dedicated theme:** Improvement work is easy to defer when feature
pressure mounts. A dedicated theme makes it visible, schedulable, and
trackable — not just "we'll clean it up later."

## In Scope

- **Refactoring:** Extract repeated patterns into shared packages, decompose
  large modules, improve naming and structure
- **Code reviews:** Track findings from Forge reviews, code audits, and PR
  feedback. Non-urgent improvements that accumulate.
- **Shared libraries:** Extract commonly-used utilities into `packages/shared`
  (CLI helpers, formatting, error handling, workspace resolution)
- **Generators:** Nx generators for scaffolding new CLI commands, gate checks,
  adapters, and Rust crate templates
- **Tech debt:** Track known issues, deprecation cleanup, legacy pattern removal
- **DX improvements:** Faster builds, better error messages, improved dev
  workflow

## Out of Scope

- Feature development (covered by other themes)
- Security hardening (covered by security module)
- Testing infrastructure (covered by testing-strategy)
- Documentation (covered by documentation-sync)

## Interfaces

**Depends on:**

- `code-review-backlog` — existing review findings (29 tasks, mostly complete)
- `codebase-maintenance` — existing pattern extraction work
- `packages/shared` — target for shared utilities
- `tools/generators` — Nx generator infrastructure

**Exposes:**

- Refactoring backlog
- Shared library catalogue
- Generator templates

## Related Modules

| Module | Scope | Status |
| ------ | ----- | -------- |
| [codebase-maintenance](./codebase-maintenance.aps.md) | MAINT | In Progress |
| [code-review-backlog](./code-review-backlog.aps.md) | CRB | Mostly Complete |

## Tasks

- CI-001: Shared utility audit — identify repeated patterns across packages
- CI-002: packages/shared scaffold (util/, testing/, brand/)
- CI-003: CLI helper consolidation (error formatting, output, workspace root)
- CI-004: Nx generator for new CLI commands
- CI-005: Nx generator for new gate checks
- CI-006: Nx generator for new Rust crate (anvil-* template)
- CI-007: Tech debt register and tracking process
- CI-008: Refactoring backlog from code review findings
- CI-009: Large module decomposition (audit CLI commands, gate runner)
- CI-010: DX improvement backlog (build speed, error messages, dev workflow)
