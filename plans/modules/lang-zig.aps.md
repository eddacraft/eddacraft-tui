<!--
APS Module: Zig Language Support
====================================
Extends Anvil's analysis to Zig codebases.
See: plans/aps-rules.md
-->

# Zig Language Support

| ID     | Owner | Status    |
| ------ | ----- | --------- |
| ZIGLAN | —     | Draft |

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
