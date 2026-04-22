<!--
APS Module: Zig Language Support
====================================
Extends Anvil's analysis to Zig codebases.
See: plans/aps-rules.md
-->

# Zig Language Support — ARCHIVED (CUT)

> **Archived 2026-04-22 — cut by the
> [2026-04-08 Language and Coverage Design](../../specs/2026-04-08-language-and-coverage-design.md)**
> §13, §17.3 step 1. Zero confirmed demand, no plausible near-term user.
> No implementation planned. Re-entry requires a new demand signal, at
> which point Zig re-scores under §6 like any other candidate.

| ID     | Owner | Status                       |
| ------ | ----- | ---------------------------- |
| ZIGLAN | —     | Archived (cut — no demand) |

## Purpose

Extend Anvil to analyse Zig codebases. Zig is a growing systems language
used in performance-critical infrastructure (bun, TigerBeetle). Zig's
`@import` system and comptime features create unique architecture patterns.

## In Scope

- File extensions: `.zig`, `.zon`
- Import extraction: `@import("foo.zig")`, `@cImport(...)`
- Module structure: `pub` declarations, build.zig module graph
- Zig-specific anti-pattern detectors:
  - `@intToPtr` / `@ptrToInt` (unsafe pointer casts)
  - `unreachable` in non-test code
  - `@panic()` usage
  - `@embedFile` with hardcoded paths
  - Unused `_` discards on errors
  - `comptime` blocks with side effects
- Suppression syntax: `// @anvil-ignore AP-XXX: reason`
- Entry point detection: `pub fn main()`, `build.zig`

## Out of Scope

- Build system deep analysis (build.zig)
- C interop analysis (`@cImport`)
- Comptime evaluation analysis

## Estimated Scope

- **Anti-patterns:** 6 new patterns
- **Effort:** 1-2 weeks

## Tasks

- ZIGLAN-001: Zig import extraction via tree-sitter-zig
- ZIGLAN-002: Zig anti-pattern catalogue (unsafe casts, panic)
- ZIGLAN-003: Tests and documentation
