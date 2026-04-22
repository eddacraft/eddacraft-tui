<!--
APS Module: Go Language Support
====================================
Extends Anvil's analysis to Go codebases.
See: plans/aps-rules.md
-->

# Go Language Support — ARCHIVED

> **Archived 2026-04-22.** Folded into
> [lang-tail-wave](../../modules/lang-tail-wave.aps.md) per
> [2026-04-08 Language and Coverage Design](../../specs/2026-04-08-language-and-coverage-design.md)
> §8.2, §17.3 step 2. Demand point: 0. Tail-wave acceptance is T1 only
> (parsed + symbol graph inclusion). The placeholder content below is
> preserved for historical reference.

| ID     | Owner | Status   |
| ------ | ----- | -------- |
| GOLAN  | —     | Archived |

## Purpose

Extend Anvil to analyse Go codebases. Go dominates infrastructure and platform
engineering — the exact teams that enforce strict architectural boundaries
(packages, modules, internal/external). Go's conventions make boundary detection
particularly valuable: `internal/` directories, exported vs unexported symbols,
and module paths encode architecture intent.

## In Scope

- File extensions: `.go`
- Import extraction: `"fmt"`, `"github.com/org/repo/pkg"`, relative `"./pkg"`
- Package structure: `package` declarations, `internal/` convention
- Go-specific anti-pattern detectors:
  - `//nolint` directives (equivalent to `eslint-disable`)
  - `panic()` in non-main packages
  - Empty `default:` in type switches
  - `interface{}` usage (any in Go 1.18+)
  - Ignored error returns (`_, _ =` patterns)
  - `init()` function side effects
- Suppression syntax: `// @anvil-ignore AP-XXX: reason`
- Entry point detection: `func main()`, `go.mod`

## Out of Scope

- `go.sum` dependency auditing (use existing dependency check)
- `cgo` interop analysis
- Generics pattern analysis (Go 1.18+)
- Vendored dependency analysis

## Interfaces

**Depends on:**

- `save-time-trust` — runner and warning schema
- `antipattern-library` — scanner infrastructure
- `architecture-safety` — edge detector, layer detector
- `suppressions` — suppression parser
- `html-css-support` — configurable extensions infrastructure (HTMLCSS-001)

**Exposes:**

- Go anti-pattern definitions
- Go import extraction (tree-sitter-go)
- Go `//` comment suppression support
- Go package/module structure analysis

## Prerequisites

- HTMLCSS-001 (configurable extensions) must be complete
- tree-sitter-go available in Rust kernel

## Estimated Scope

- **Anti-patterns:** 6 new patterns
- **Edge detection:** Import extraction via tree-sitter-go
- **Suppression:** `//` comment syntax (already supported)
- **Entry points:** `func main()`, `go.mod` detection
- **Effort:** 1-2 weeks

## Tasks

Tasks will be defined when this module moves to Ready status. Expected
breakdown:

- GOLAN-001: Go import extraction via tree-sitter-go
- GOLAN-002: Go anti-pattern catalogue (nolint, panic, ignored errors)
- GOLAN-003: Go package boundary detection (internal/, exported)
- GOLAN-004: Go module path resolution (go.mod)
- GOLAN-005: Go entry point detection
- GOLAN-006: Tests and documentation
