<!--
APS Module: Testing Strategy
====================================
Cross-cutting testing strategy for the monorepo.
See: plans/aps-rules.md
-->

# Testing Strategy

| ID     | Owner | Status    |
| ------ | ----- | --------- |
| TEST   | —     | Draft |

## Purpose

Establish and maintain testing strategy across the monorepo: coverage targets,
integration testing matrix, E2E strategy (Playwright), custom ESLint rules for
test quality, test infrastructure evolution, and Rust test harness coordination.

**Problem:** Tests exist across packages but there's no governing strategy for
coverage targets, integration test scope, or how Rust and TypeScript tests
coordinate. The `eslint-plugin-anvil` package (custom test quality rules) has
zero APS coverage.

## In Scope

- **Coverage targets:** Per-package thresholds, gate check integration
- **Integration testing:** Cross-package test patterns, API ↔ CLI ↔ MCP testing
- **E2E strategy:** Playwright test matrix for CLI, TUI, website, docs-site
- **eslint-plugin-anvil:** Custom ESLint rules for test quality (AP-001 through
  AP-007 enforcement, no-empty-catch, no-explicit-any, etc.)
- **Test infrastructure:** Shared test utilities, fixtures management, snapshot
  testing conventions (Rust + TypeScript)
- **Rust ↔ TypeScript coordination:** Dual-run testing, parity validation
- **Benchmark regression:** Performance test thresholds, CI integration

## Out of Scope

- Individual test implementation (covered by each feature module)
- Adversarial testing (covered by adversarial-testing-catalog)

## Interfaces

**Depends on:**

- `eslint-plugin-anvil` — custom rule package
- `apps/e2e` — Playwright test infrastructure
- All packages — test coverage data

**Exposes:**

- Testing policy document
- Coverage target configuration
- E2E test matrix
- Custom ESLint rule catalogue

## Estimated Scope

- **Effort:** 1 week

## Tasks

- TEST-001: Coverage target policy and gate check thresholds
- TEST-002: E2E test matrix definition (what to test where)
- TEST-003: eslint-plugin-anvil rules audit and gap analysis
- TEST-004: Rust ↔ TypeScript parity test framework
- TEST-005: Benchmark regression thresholds and CI integration
- TEST-006: Snapshot testing conventions (insta for Rust, vitest for TS)
