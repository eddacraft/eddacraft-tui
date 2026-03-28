<!--
APS Module: .NET Language Support
==================================
Extends Anvil's analysis to C# and .NET codebases.
See: plans/aps-rules.md
-->

# .NET Language Support

| ID     | Owner | Status      |
| ------ | ----- | ----------- |
| DNLAN  | —     | Draft |

## Purpose

Extend Anvil to analyse C# and .NET codebases. .NET projects have well-defined
architecture patterns (Clean Architecture, vertical slice, CQRS) that map
directly to Anvil's boundary detection model. C# has its own set of escape
hatches and suppression directives that need dedicated detection.

## In Scope

- .NET file extensions: `.cs`, `.csx`, `.vb`, `.fs`, `.fsx`
- Import extraction: `using System.IO;`, `using static`,
  `global using`, `using alias = Namespace.Type`
- C#-specific anti-pattern detectors:
  - `#pragma warning disable` (linter suppression)
  - `[SuppressMessage(...)]` attributes
  - `dynamic` type usage (equivalent to TS `any`)
  - Empty catch `catch { }` / `catch (Exception) { }` (already detected for
    JS but needs C# syntax variant)
  - `Console.Write` / `Console.WriteLine` in non-console projects
  - `// ReSharper disable` directives
  - `object` type as parameter/return (weak typing)
- Suppression syntax using `//` comments (already supported)
- Entry point detection: `static void Main`, `Program.cs` top-level statements,
  `.csproj` output type
- Namespace-to-folder mapping for boundary detection

## Out of Scope

- NuGet dependency resolution
- MSBuild / `.csproj` dependency analysis
- Roslyn analyser integration
- Entity Framework / LINQ query analysis
- ASP.NET route analysis
- F# and VB.NET anti-patterns (start with C# only, expand later)

## Interfaces

**Depends on:**

- `save-time-trust` — runner and warning schema
- `antipattern-library` — scanner infrastructure
- `architecture-safety` — edge detector, layer detector
- `suppressions` — suppression parser
- `html-css-support` — configurable extensions infrastructure (HTMLCSS-001)

**Exposes:**

- C#/.NET anti-pattern definitions
- C# `using` import extraction regexes
- .NET entry point and namespace detection

## Prerequisites

- HTMLCSS-001 (configurable extensions) must be complete

## Estimated Scope

- **Anti-patterns:** 7 new patterns
- **Edge detection:** 3-4 new import regexes (`using` variants)
- **Suppression:** None needed — C# uses `//` comments which are already
  supported
- **Entry points:** New detection for `.csproj`, `Program.cs`, `Main` method
- **Effort:** 1-2 weeks

## Tasks

Tasks will be defined when this module moves to Ready status. Expected
breakdown:

- DNLAN-001: C# `using` import extraction regexes
- DNLAN-002: C# anti-pattern catalogue
- DNLAN-003: .NET entry point detection (`.csproj`, `Program.cs`,
  `static void Main`)
- DNLAN-004: C# namespace-to-folder mapping for boundaries
- DNLAN-005: Tests and documentation

## Risks

| Risk                              | Impact | Mitigation                              |
| --------------------------------- | ------ | --------------------------------------- |
| `using` doesn't map to file paths | Medium | Use namespace-to-folder conventions     |
| `global using` adds noise         | Low    | Detect but don't create edges for them  |
| Framework-specific patterns vary  | Low    | Defer ASP.NET/EF patterns to add-ons   |

## Open Questions

- [ ] Should namespace hierarchy be inferred from folder structure or `.csproj`?
- [ ] How to handle `global using` directives (C# 10+)?
- [ ] Should `.razor` / `.cshtml` files be included? (Closer to HTML/CSS module)
- [ ] Solution-level (`.sln`) vs project-level (`.csproj`) analysis scope?
