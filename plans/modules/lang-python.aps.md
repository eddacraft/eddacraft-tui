<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Python Language Anchor (Track 1)

| ID    | Owner | Status |
| ----- | ----- | ------ |
| PYLAN | —     | Draft  |

**Last reviewed:** 2026-04-26

## Purpose

Bring Python to **T3 (Governed)** per
[2026-04-08 Language and Coverage Design](../specs/2026-04-08-language-and-coverage-design.md)
§5.1, §8.1. Python is the strongest **strategic** unlock on the anchor list —
"Anvil governs AI/ML stacks" needs a Python anchor, and the Python-substrate
LLM Provider extension (`pack-llm-provider` Phase 2) lands here. Two confirmed
demand points (User B + User C). Sequenced after Rust per spec §7.2 because
Rust's "governs systems code" strategic narrative is the higher-ROI next
anchor after TS — User C raises Python *confidence*, not its relative
priority versus Rust.

This module **rewrites** the previous regex-era Python placeholder. The
previous content assumed regex-based parsing and an `HTMLCSS-001` prerequisite
that has since been archived. Tree-sitter-based reality changes the
implementation shape entirely.

## In Scope

- `tree-sitter-python` grammar wired through whatever extractor abstraction
  `LANGTS` produces.
- File detection: `.py`, `.pyi`.
- Symbol/import extraction handling Python shapes:
  - `import foo`, `import foo.bar as baz`
  - `from foo import bar`, `from foo.bar import baz, qux`
  - Relative imports (`from . import x`, `from ..pkg import y`)
  - Namespace packages (PEP 420 — implicit `__init__.py`)
  - Conditional imports inside `try/except ImportError` (best-effort)
  - Avoid `importlib.import_module(...)` magic — document as limitation.
- T2 anti-pattern catalogue (per spec §8.1):
  - `# type: ignore` without justification comment
  - `# noqa` without specific rule
  - Bare `except:` / `except Exception: pass`
  - `Any` type usage from `typing`
  - `print()` in production code
  - `# pylint: disable` without justification
  - Import star (`from foo import *`)
- Suppression syntax: `# @anvil-ignore <ID>: <reason>`.
- Entry-point detection: `if __name__ == '__main__'`, `pyproject.toml`
  `[project.scripts]` and `[project.gui-scripts]`, `setup.py` console_scripts,
  `setup.cfg`.
- Layer/boundary enforcement reaching Python packages and modules — apply
  the same architecture-enforcement-location decision recorded for Rust
  (council §16.5 #5).
- Drift baseline default-on for `.py` files.
- `architecture-validate` includes Python packages and module graphs.

## Out of Scope

- Virtualenv / pip / poetry / conda dependency-graph analysis (lives in
  `config-intelligence`).
- Python AST analysis beyond what tree-sitter provides.
- Type checker replacement (mypy/pyright integration is out of scope).
- Django, FastAPI, LangChain framework patterns (lives in `pack-llm-provider`
  Phase 2 extension and Phase 3 Django/FastAPI packs).

## Interfaces

**Depends on:**

- [`lang-ts-audit`](./lang-ts-audit.aps.md) — T3 acceptance checklist.
- [`lang-rust`](./lang-rust.aps.md) — sequenced after; reuses
  architecture-enforcement-location decision.
- Existing kernel parser, architecture analysis, policy pipeline, drift
  baseline, suppression parser.

**Exposes:**

- Python at T3 — substrate-tier prerequisite for the Phase 2
  Python-substrate extension of `pack-llm-provider`, and (Phase 3)
  `pack-django` / `pack-fastapi`.

## Prerequisites

- `lang-ts-audit` complete (T3 checklist exists).
- `lang-rust` complete (validates the checklist and the anchor pattern).
- Re-scoring gate run per
  [docs/guides/anchor-rescoring-process.md](../../docs/guides/anchor-rescoring-process.md)
  before this module starts.

## Ready Checklist

Change status to **Ready** when:

- [ ] LANGTS and RSTLAN both Complete.
- [ ] Re-scoring gate run; Python still anchor #3 after Rust.
- [ ] Owner named for the anchor work.
- [ ] User C's framework choice documented (informs Phase 3 pack scheduling).

## Work Items

Tasks will be defined when this module moves to Ready. Anticipated shape:

- PYLAN-001: Tree-sitter-python grammar wired through extractor trait.
- PYLAN-002: Python symbol/import extraction (absolute, relative, namespace
  packages).
- PYLAN-003: Python T2 anti-pattern catalogue.
- PYLAN-004: `#`-comment suppression syntax integrated with suppression
  parser.
- PYLAN-005: Entry-point detection (`__main__`, `pyproject.toml`,
  `setup.py`/`setup.cfg`).
- PYLAN-006: Layer/boundary enforcement reaches Python.
- PYLAN-007: Drift baseline default-on for `.py`.
- PYLAN-008: `architecture-validate` includes Python packages.
- PYLAN-009: Validate against User B + User C codebases — FP rate < N% per
  council §16.5 #9.

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Namespace package extraction underdetermined | Medium | Use `__init__.py` presence as primary heuristic; document PEP 420 limitation |
| Relative-import resolution requires path context | Medium | Resolve via package root inferred from `pyproject.toml`/`setup.cfg` |
| Framework-specific patterns leak into anchor T2 | Medium | Strict separation — frameworks live in packs |
| Single-stack User C validation is narrow | Medium | Pair User C run with Anvil's own one Python file + any open-source Python codebase as second data point |
| Anchor stalled while Rust slips | Low | Sequence is firm; do not pre-empt Rust delays into Python work |

## Open Questions

- [ ] Should Python support require `pyproject.toml` or work without one?
- [ ] How is `mypy.ini` / `[tool.mypy]` consulted, if at all?
- [ ] How are conditional / lazy imports inside `try/except ImportError`
      represented in the symbol graph?
- [ ] Does User C use Django or FastAPI? — informs Phase 3 pack scheduling.
