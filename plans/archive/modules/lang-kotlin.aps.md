<!--
APS Module: Kotlin Language Support
====================================
Extends Anvil's analysis to Kotlin codebases.
See: plans/aps-rules.md
-->

# Kotlin Language Support — ARCHIVED

> **Archived 2026-04-22.** Folded into
> [lang-tail-wave](../../modules/lang-tail-wave.aps.md) per
> [2026-04-08 Language and Coverage Design](../../specs/2026-04-08-language-and-coverage-design.md)
> §8.2, §17.3 step 2. Demand point: 0. Tail-wave acceptance is T1 only
> (parsed + symbol graph inclusion). The placeholder content below is
> preserved for historical reference.

| ID     | Owner | Status   |
| ------ | ----- | -------- |
| KOTLAN | —     | Archived |

## Purpose

Extend Anvil to analyse Kotlin codebases. Kotlin is the primary Android language
and growing in backend (Ktor, Spring Boot). Kotlin's null-safety and coroutine
patterns introduce new anti-pattern surfaces worth detecting.

## In Scope

- File extensions: `.kt`, `.kts`
- Import extraction: `import foo.Bar`, `import foo.*`
- Package structure: `package` declarations
- Kotlin-specific anti-pattern detectors:
  - `@Suppress` annotations
  - `!!` (force unwrap / non-null assertion)
  - Empty `catch` blocks
  - `lateinit var` overuse
  - Blocking calls in coroutines (`runBlocking` in suspending context)
  - `TODO()` stubs left in production
- Suppression syntax: `// @anvil-ignore AP-XXX: reason`
- Entry point detection: `fun main()`, `build.gradle.kts`

## Out of Scope

- Android-specific patterns (Compose, Activities) — future add-on
- KSP/KAPT annotation processor analysis
- Multiplatform source set analysis

## Interfaces

**Depends on:**

- `save-time-trust`, `antipattern-library`, `architecture-safety`
- `html-css-support` — configurable extensions infrastructure (HTMLCSS-001)

**Exposes:**

- Kotlin anti-pattern definitions
- Kotlin import extraction (tree-sitter-kotlin)

## Estimated Scope

- **Anti-patterns:** 6 new patterns
- **Effort:** 1-2 weeks

## Tasks

- KOTLAN-001: Kotlin import extraction via tree-sitter-kotlin
- KOTLAN-002: Kotlin anti-pattern catalogue (null-safety, coroutines)
- KOTLAN-003: Kotlin package boundary detection
- KOTLAN-004: Tests and documentation
