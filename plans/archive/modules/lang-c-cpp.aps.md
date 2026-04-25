<!--
APS Module: C/C++ Language Support
====================================
Extends Anvil's analysis to C and C++ codebases.
See: plans/aps-rules.md
-->

# C/C++ Language Support — ARCHIVED

> **Archived 2026-04-22.** Folded into
> [lang-tail-wave](../../modules/lang-tail-wave.aps.md) per
> [2026-04-08 Language and Coverage Design](../../specs/2026-04-08-language-and-coverage-design.md)
> §8.2, §17.3 step 2. Demand point: 0. Tail-wave policy per spec §12.3:
> drop C/C++ from the wave if `tree-sitter-c`/`tree-sitter-cpp` quality
> blocks the batch (e.g. C++20/23 partial-parse issues — council finding
> C-005). The placeholder content below is preserved for historical
> reference.

| ID     | Owner | Status   |
| ------ | ----- | -------- |
| CCLAN  | —     | Archived |

## Purpose

Extend Anvil to analyse C and C++ codebases. C/C++ power embedded systems,
operating systems, game engines, and performance-critical libraries. Header
file inclusion patterns encode architecture intent (public API headers vs
internal headers).

## In Scope

- File extensions: `.c`, `.h`, `.cpp`, `.hpp`, `.cc`, `.hh`
- Import extraction: `#include <foo.h>`, `#include "foo.h"`
- Header structure: Header guards, `#pragma once`
- C/C++ specific anti-pattern detectors:
  - `#pragma warning(disable: ...)` (suppress warnings)
  - `assert()` in production code
  - `NULL` pointer patterns (prefer `nullptr` in C++)
  - `using namespace std;` in headers (C++)
  - Missing include guards
  - C-style casts in C++ (`(int)x` vs `static_cast<int>(x)`)
- Suppression syntax: `// @anvil-ignore AP-XXX: reason`
- Entry point detection: `int main()`, `CMakeLists.txt`, `Makefile`

## Out of Scope

- Template metaprogramming analysis
- C++20 modules analysis
- Build system deep analysis (CMake dependency graph)
- Cross-compilation target analysis

## Estimated Scope

- **Anti-patterns:** 6 new patterns (C), 4 new patterns (C++)
- **Edge detection:** `#include` extraction (different from import semantics)
- **Effort:** 2-3 weeks (header resolution is complex)

## Tasks

- CCLAN-001: C/C++ include extraction via tree-sitter-c + tree-sitter-cpp
- CCLAN-002: C anti-pattern catalogue
- CCLAN-003: C++ anti-pattern catalogue
- CCLAN-004: Header boundary detection (public vs internal)
- CCLAN-005: Tests and documentation
