<!--
APS Module: Python Language Support
====================================
Extends Anvil's analysis to Python codebases.
See: plans/aps-rules.md
-->

# Python Language Support

| ID     | Owner | Status    |
| ------ | ----- | --------- |
| PYLAN  | —     | Placeholder |

## Purpose

Extend Anvil to analyse Python codebases. Python is the second most common
language in AI-assisted development and shares many of the same architecture
drift problems Anvil solves for TypeScript. This module adds Python import
extraction, Python-specific anti-pattern detection, and Python ecosystem entry
point detection.

## In Scope

- Python file extensions: `.py`, `.pyi`
- Import extraction: `import foo`, `from foo import bar`,
  `from foo.bar import baz`, `importlib.import_module("foo")`
- Python-specific anti-pattern detectors:
  - `# type: ignore` directives (equivalent to `@ts-ignore`)
  - `# noqa` directives (equivalent to `eslint-disable`)
  - `except: pass` / bare `except:` (equivalent to empty catch)
  - `Any` type usage from `typing` (equivalent to TS `any`)
  - `print()` in production code (equivalent to `console.log`)
  - `# pylint: disable` directives
- Suppression syntax using `#` comments: `# @anvil-ignore AP-XXX: reason`
- Entry point detection: `if __name__ == '__main__'`, `pyproject.toml`,
  `setup.py`, `setup.cfg`
- Path resolution for relative imports (`from . import`, `from .. import`)

## Out of Scope

- Virtual environment analysis
- Pip/poetry/conda dependency resolution
- Python AST analysis (ast module) — start with regex, graduate later
- Django/Flask/FastAPI framework-specific patterns — future add-on packs
- Type checking (mypy/pyright integration)

## Interfaces

**Depends on:**

- `save-time-trust` — runner and warning schema
- `antipattern-library` — scanner infrastructure
- `architecture-safety` — edge detector, layer detector
- `suppressions` — suppression parser
- `html-css-support` — configurable extensions infrastructure (HTMLCSS-001)

**Exposes:**

- Python anti-pattern definitions
- Python import extraction regexes
- Python comment suppression support
- Python entry point detection

## Prerequisites

- HTMLCSS-001 (configurable extensions) must be complete

## Estimated Scope

- **Anti-patterns:** 6 new patterns
- **Edge detection:** 3-4 new import regexes + relative import resolution
- **Suppression:** 1 new comment syntax regex (`#`)
- **Entry points:** New detection strategy for Python ecosystem
- **Effort:** 1-2 weeks

## Tasks

Tasks will be defined when this module moves to Ready status. Expected
breakdown:

- PYLAN-001: Python import extraction regexes
- PYLAN-002: Python anti-pattern catalogue
- PYLAN-003: Python `#` comment suppression syntax
- PYLAN-004: Python entry point detection
- PYLAN-005: Python path resolution for relative imports
- PYLAN-006: Tests and documentation

## Risks

| Risk                             | Impact | Mitigation                              |
| -------------------------------- | ------ | --------------------------------------- |
| Regex misses complex imports     | Medium | Start with common patterns; iterate     |
| Relative import resolution hard  | Medium | Use `__init__.py` presence as heuristic |
| Framework-specific patterns vary | Low    | Defer to add-on packs                   |

## Open Questions

- [ ] Should Python support require a `pyproject.toml` or work without one?
- [ ] How to handle namespace packages (implicit `__init__.py`)?
- [ ] Should `mypy.ini` / `pyproject.toml` `[tool.mypy]` inform type-ignore
      detection?
