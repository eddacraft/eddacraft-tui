<!--
APS Module: C/C++ Language Support
====================================
Extends Anvil's analysis to C and C++ codebases.
See: plans/aps-rules.md
-->

# C/C++ Language Support

| ID     | Owner | Status    |
| ------ | ----- | --------- |
| CCLAN  | —     | Draft |

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
