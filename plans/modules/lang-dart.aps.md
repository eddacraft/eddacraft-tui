<!--
APS Module: Dart Language Support
====================================
Extends Anvil's analysis to Dart/Flutter codebases.
See: plans/aps-rules.md
-->

# Dart Language Support

| ID     | Owner | Status    |
| ------ | ----- | --------- |
| DARTLAN | —     | Draft |

## Purpose

Extend Anvil to analyse Dart codebases. Dart is the language behind Flutter,
the dominant cross-platform mobile framework. Flutter apps benefit from
architecture enforcement across feature modules, widget trees, and state
management patterns.

## In Scope

- File extensions: `.dart`
- Import extraction: `import 'package:foo/bar.dart'`, `import 'dart:io'`
- Package structure: `library` declarations, `part`/`part of` directives
- Dart-specific anti-pattern detectors:
  - `// ignore:` directives
  - `as` casts without null checks
  - Empty `catch` blocks
  - `dynamic` type usage
  - `print()` in production code
  - `// TODO` without issue reference
- Suppression syntax: `// @anvil-ignore AP-XXX: reason`
- Entry point detection: `void main()`, `pubspec.yaml`

## Out of Scope

- Flutter widget tree analysis
- State management pattern detection (Provider, Bloc, Riverpod)
- Platform channel analysis

## Estimated Scope

- **Anti-patterns:** 6 new patterns
- **Effort:** 1-2 weeks

## Tasks

- DARTLAN-001: Dart import extraction via tree-sitter-dart
- DARTLAN-002: Dart anti-pattern catalogue
- DARTLAN-003: Tests and documentation
