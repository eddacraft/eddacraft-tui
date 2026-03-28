<!--
APS Module: Java Language Support
====================================
Extends Anvil's analysis to Java codebases.
See: plans/aps-rules.md
-->

# Java Language Support

| ID     | Owner | Status    |
| ------ | ----- | --------- |
| JAVALAN | —     | Draft |

## Purpose

Extend Anvil to analyse Java codebases. Java remains dominant in enterprise
backend systems with strong architecture conventions (packages, modules,
layered architecture). Anvil can detect cross-layer imports, unauthorised
dependency edges, and Java-specific anti-patterns.

## In Scope

- File extensions: `.java`
- Import extraction: `import foo.Bar`, `import foo.*`, `import static foo.Bar`
- Package structure: `package` declarations, module-info.java
- Java-specific anti-pattern detectors:
  - `@SuppressWarnings` annotations (equivalent to `eslint-disable`)
  - `catch(Exception e) {}` (empty catch)
  - Raw type usage (`List` instead of `List<String>`)
  - `System.out.println()` in production code
  - `Thread.sleep()` in non-test code
  - `@Deprecated` without migration plan
- Suppression syntax: `// @anvil-ignore AP-XXX: reason`
- Entry point detection: `public static void main`, `pom.xml`, `build.gradle`

## Out of Scope

- Maven/Gradle dependency resolution
- Spring-specific annotations analysis
- Java module system (JPMS) deep analysis
- Annotation processor patterns

## Interfaces

**Depends on:**

- `save-time-trust` — runner and warning schema
- `antipattern-library` — scanner infrastructure
- `architecture-safety` — edge detector, layer detector
- `html-css-support` — configurable extensions infrastructure (HTMLCSS-001)

**Exposes:**

- Java anti-pattern definitions
- Java import extraction (tree-sitter-java)
- Java package boundary analysis

## Estimated Scope

- **Anti-patterns:** 6 new patterns
- **Effort:** 1-2 weeks

## Tasks

- JAVALAN-001: Java import extraction via tree-sitter-java
- JAVALAN-002: Java anti-pattern catalogue
- JAVALAN-003: Java package boundary detection
- JAVALAN-004: Build tool integration (pom.xml, build.gradle)
- JAVALAN-005: Tests and documentation
