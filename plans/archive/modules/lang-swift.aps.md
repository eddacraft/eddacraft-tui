<!--
APS Module: Swift Language Support
====================================
Extends Anvil's analysis to Swift codebases.
See: plans/aps-rules.md
-->

# Swift Language Support — ARCHIVED (CUT)

> **Archived 2026-04-22 — cut by the
> [2026-04-08 Language and Coverage Design](../../specs/2026-04-08-language-and-coverage-design.md)**
> §13, §17.3 step 1. Zero confirmed demand, no plausible near-term user.
> No implementation planned. Re-entry requires a new demand signal, at
> which point Swift re-scores under §6 like any other candidate.

| ID     | Owner | Status            |
| ------ | ----- | ----------------- |
| SWIFTLAN | —     | Archived (cut — no demand) |

## Purpose

Extend Anvil to analyse Swift codebases. Swift is the primary language for
Apple platform development (iOS, macOS, watchOS, tvOS). Swift's protocol-
oriented design and module system create clear architecture boundaries worth
enforcing.

## In Scope

- File extensions: `.swift`
- Import extraction: `import Foundation`, `import class Module.ClassName`
- Module structure: `module` declarations, access control levels
- Swift-specific anti-pattern detectors:
  - `// swiftlint:disable` directives
  - `try!` (force try)
  - `as!` (force cast)
  - `!` (force unwrap) on optionals
  - `fatalError()` in non-test code
  - `// MARK:` without logical grouping
- Suppression syntax: `// @anvil-ignore AP-XXX: reason`
- Entry point detection: `@main`, `Package.swift`

## Out of Scope

- SwiftUI view hierarchy analysis
- Combine/async stream pattern detection
- Xcode project file analysis

## Estimated Scope

- **Anti-patterns:** 6 new patterns
- **Effort:** 1-2 weeks

## Tasks

- SWIFTLAN-001: Swift import extraction via tree-sitter-swift
- SWIFTLAN-002: Swift anti-pattern catalogue (force unwrap, force try)
- SWIFTLAN-003: Tests and documentation
