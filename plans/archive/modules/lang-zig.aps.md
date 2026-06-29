<!--
APS Module: Zig Language Support
====================================
Extends Anvil's analysis to Zig codebases.
See: plans/aps-rules.md
-->

# Zig Language Support — ARCHIVED (CUT, then RE-ENTERED at T1)

> **Re-entered 2026-06-29** — owner-directed addition at **T1 (Parsed)** via
> [ADR-093](../../decisions/093-tail-wave-2-wasm-text-and-zig-reentry.md). The T1
> parse/extract/graph-inclusion slice is folded into
> [`lang-tail-wave-2`](../../modules/lang-tail-wave-2.aps.md) (LTW2) — this
> module is **not** un-archived as a standalone active module (mirrors how the
> wave-1 `lang-*` modules folded into LANGTAIL). The T2 anti-pattern catalogue
> below does **NOT** re-enter; only the T1 slice does. This file stays archived
> as the historical record.
>
> **Originally archived 2026-04-22 — cut by the
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
