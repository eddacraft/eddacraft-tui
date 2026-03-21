<!--
APS Module: Release Management
====================================
Release cadence, changelog, and publish strategy.
See: plans/aps-rules.md
-->

# Release Management

| ID      | Owner | Status    |
| ------- | ----- | --------- |
| RELMGMT | —     | Draft |

## Purpose

Establish release management practices for the growing set of packages: npm
packages (TypeScript), Rust crates, and the CLI. Covers release cadence,
changelog governance, semver policy, and version coordination across
monorepo packages.

**Problem:** The project has npm packages, Rust crates, a CLI, a website,
and a docs site — but no release management module. Releases happen ad-hoc
via `release.ts`. There's no changelog governance, semver policy, or
coordination between TypeScript and Rust release cycles.

## In Scope

- **Release cadence:** How often to release, what triggers a release
- **Changelog governance:** Format (Keep a Changelog), automation, review
- **Semver policy:** What constitutes major/minor/patch across packages
- **Version coordination:** npm packages vs Rust crates — coupled or independent?
- **Publish pipeline:** npm publish, cargo publish, Vercel deploy
- **Pre-release strategy:** Alpha/beta/rc channels
- **Release notes:** Auto-generated vs manual, communication strategy
- **Breaking change process:** Migration guides, deprecation period

## Out of Scope

- CI/CD pipeline implementation (covered by CI modules)
- Feature flags (separate concern)

## Interfaces

**Depends on:**

- CI pipeline — automated release checks
- All packages — version data

**Exposes:**

- Release policy document
- Changelog format specification
- Semver decision matrix

## Estimated Scope

- **Effort:** 1 week

## Tasks

- RELMGMT-001: Release cadence policy and triggers
- RELMGMT-002: Changelog format specification and automation
- RELMGMT-003: Semver policy across npm + Rust packages
- RELMGMT-004: Publish pipeline documentation
- RELMGMT-005: Pre-release channel strategy (alpha/beta/rc)
- RELMGMT-006: Breaking change process and migration guide template
