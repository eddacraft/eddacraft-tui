<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Python Language Anchor (Track 1)

| ID    | Owner | Status      |
| ----- | ----- | ----------- |
| PYLAN | —     | In Progress |

**Last reviewed:** 2026-06-18 (substrate + governance slice landed; PYLAN-009
FP bar accepted at N=1%. Promoted
Draft → In Progress on operator direction — "build lang-python first" — to
unblock GCALL-005 Python call-site extraction. Prerequisites LANGTS and RSTLAN
both Complete; the remaining Ready-Checklist items (re-scoring gate, named
owner, User C framework choice) stay deferred by the operator. **PYLAN-001/-002**
(grammar + symbol/import extractor) Merged via #2716; **PYLAN-005**
(entry-point detection) via #2731; **PYLAN-006/-008** (import resolver +
boundary/architecture-validate surface) via #2732; **PYLAN-003/-004/-007**
(`python-reliability` anti-pattern catalogue + `#`-suppression + `.py` drift
default-on) via #2734; **PYLAN-009** (T3 dogfood + FP bar — 0.0% < N=1% on
httpx + rich) via #2740. **All nine items Merged** — Python is at T3 (parsed,
symbol graph, entry points, boundary enforcement, anti-pattern catalogue,
drift). The module stays In Progress until a release tag ships these items
(Released/Shipped → **Complete**), per the APS lifecycle.)

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
- Suppression syntax: `# @anvil-ignore <ID> -- <reason>` (the ADR-029 parser's
  `--` separator; matches TS/Rust `// @anvil-ignore <ID> -- <reason>`).
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

- [`lang-ts-audit`](../archive/modules/lang-ts-audit.aps.md) — T3 acceptance checklist.
- [`lang-rust`](../archive/modules/lang-rust.aps.md) — sequenced after; reuses
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

PYLAN-001..009 are all Merged (see each item below). The module stays In
Progress until a release tag ships them (Released/Shipped → Complete).

#### PYLAN-001: Tree-sitter-python grammar wired through the extractor trait

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-17 via #2716 — `tree-sitter-python` added (workspace + kernel),
  `Language::Python` with `.py` / `.pyi` detection and `ts_language()` wiring,
  folded into the grammar-version cache key. Delivered in the foundational PR.
- **Intent:** Make Python files parseable by the existing kernel parser surface.
- **Expected Outcome:** `.py` / `.pyi` files route through the
  `LanguageExtractor` dispatch like TS and Rust; no orchestrator `if lang ==`
  cascade.
- **Validation:** `Language::from_path` maps `.py`/`.pyi`; the parser produces a
  tree and the dispatch reaches the Python extractor.
- **Files:** `Cargo.toml`, `crates/anvil-kernel/Cargo.toml`,
  `crates/anvil-kernel/src/parser/languages.rs`,
  `crates/anvil-kernel/src/parser/extract/mod.rs`
- **Dependencies:** —

---

#### PYLAN-002: Python symbol/import extraction

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-17 via #2716 — `python.rs` `PythonExtractor` emits `FileSymbols`:
  `def` → Function, `class` → Class, class-body `def` → `Owner.method` Method
  (decorated defs unwrapped), leading-underscore visibility; and one
  `ImportEdge` per `import` / `from`-import module, preserving relative-import
  dot prefixes (`.`, `..pkg`) and recording the module for star imports.
  Re-exports and call sites are out of scope (call sites are GCALL-005).
  Delivered in the foundational PR with fixture tests.
- **Intent:** Surface Python modules in the symbol graph exactly as TS/Rust.
- **Expected Outcome:** Symbols + import edges for the Python shapes in scope
  (absolute, dotted, aliased, relative, star, namespace packages best-effort).
- **Validation:** Fixture tests cover functions/classes/methods, decorated
  defs, visibility, and plain/dotted/aliased/from/relative/star imports;
  deterministic.
- **Files:** `crates/anvil-kernel/src/parser/extract/python.rs`
- **Dependencies:** PYLAN-001

---

#### PYLAN-003: Python T2 anti-pattern catalogue

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-17 via #2734 — new `python-reliability` pattern
  family (PY-001..007), the Python parallel of `rust-reliability`, as RE2-legal
  `Detection::Regex` rules scoped to `.py` (no AST tier needed; they run on the
  daemon-safe save-time hot path like the TS rules): PY-001 `# type: ignore`
  without an `[error-code]`, PY-002 bare `# noqa`, PY-003 `# pylint: disable`,
  PY-004 bare `except:` / inline `except ...: pass`, PY-005 wildcard
  `from x import *`, PY-006 `print()` (opt-in), PY-007 `Any` annotation incl.
  qualified `typing.Any` (opt-in). Patterns are lookahead-free (RE2 drops
  lookahead silently); a regression test guards `registry_compile_diagnostics`.
- **Intent:** Govern the Python-specific anti-patterns from spec §8.1.
- **Expected Outcome:** The catalogue fires on Python via `anvil
  check`/`gate`/drift and the save-time daemon; opt-in for the noisy rules.
- **Validation:** Per-rule positive + justified-negative tests, RE2-compile,
  extension scoping, opt-in gating; council + Copilot hardening (`\bexcept`
  FP, `^from` anchoring, `(^|[^.\w])print`, qualified `typing.Any`). Dogfood:
  0 false positives on clean idiomatic Python, all default rules fire on
  anti-pattern code.
- **Files:** `patterns/python-reliability/*.anvil`,
  `patterns/compiled/registry.json`,
  `crates/anvil-checks/src/antipattern/types.rs`,
  `crates/anvil-checks/tests/python_antipatterns.rs`
- **Dependencies:** PYLAN-001

---

#### PYLAN-004: `#`-comment suppression syntax

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-17 via #2734 — the `# @anvil-ignore <ID> -- reason`
  suppression for Python is handled by the existing ADR-029 parser (the `#`
  comment prefix was already in the suppression regex); delivered with the
  PYLAN-003 catalogue and covered by tests (marks the finding suppressed;
  a wrong-id suppression does not silence it).
- **Intent:** Let Python findings be suppressed with a reason, like TS/Rust.
- **Expected Outcome:** `# @anvil-ignore PY-NNN -- reason` suppresses the
  matching Python finding.
- **Validation:** Suppression tests in `python_antipatterns.rs`.
- **Files:** `crates/anvil-checks/tests/python_antipatterns.rs` (behaviour
  already in `crates/anvil-checks/src/antipattern/scanner.rs`)
- **Dependencies:** PYLAN-003

---

#### PYLAN-005: Python entry-point detection

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-17 via #2731 — `detect_python_entry_points(workspace_root)` added
  to `anvil-architecture`, mirroring RSTLAN-004's `detect_rust_entry_points`:
  emits `EntryPoint`s for `pyproject.toml` `[project.scripts]` (Cli) /
  `[project.gui-scripts]` (Application), `setup.cfg`
  `[options.entry_points]` console/gui scripts (High), best-effort `setup.py`
  `console_scripts`/`gui_scripts` string literals (Medium — setup.py is never
  executed), and `if __name__ == "__main__":` guards in `.py` files
  (Application, High). Declared `module:object` targets resolve to an existing
  `.py` file (flat or `src/` layout, module-or-package); unresolved targets are
  dropped. Output is workspace-relative, forward-slash, sorted, de-duplicated
  by path (declared script wins over a bare guard). The `__main__` walk prunes
  VCS/build/virtualenv/cache dirs. As with RSTLAN-004, this lands the detection
  primitive with full unit coverage ahead of its consumer (PYLAN-006/-008).
- **Intent:** Surface a Python (or mixed) repo's roots so baseline creation and
  the `anvil architecture` surfaces treat them the way they treat Rust `[[bin]]`
  and TS package `bin`/`main`.
- **Expected Outcome:** `EntryPoint`s for the in-scope Python entry shapes
  (`__main__`, `pyproject.toml`, `setup.py`/`setup.cfg`), deterministic and
  workspace-relative.
- **Validation:** Unit tests cover each source, `module:object` resolution
  (flat + `src/` + `__init__.py`), unresolved-target drop, confidence tiers,
  dedup precedence, directory pruning, and determinism.
- **Files:** `crates/anvil-architecture/src/python_detection.rs`,
  `crates/anvil-architecture/src/util.rs` (shared `relative_slash`),
  `crates/anvil-architecture/src/lib.rs`
- **Dependencies:** PYLAN-001/-002

---

#### PYLAN-006: Python layer/boundary enforcement

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-17 via #2732 — `resolve_python_import(workspace_root, from_file,
  module)` added to `anvil-architecture` (mirroring RSTLAN-005's
  `resolve_rust_import`): maps a Python import's module string to the
  workspace-relative `.py` file. Absolute `foo.bar` resolves against flat and
  `src/` package roots (`foo/bar.py` / `foo/bar/__init__.py`); relative imports
  resolve their leading-dot prefix against the importing file's package
  (`.x`, `..pkg.sub`, climbing `N-1` parents); stdlib/third-party/namespace
  modules that exist nowhere in the tree return `None` and the edge is dropped
  (conservative — never a false boundary violation). Wired into
  `gate::extract_import_edges` via a language-aware dispatch (`.py` → the Python
  resolver) and `.py` added to the boundary-scan `include_extensions`.
- **Intent:** Let layer/boundary rules reach Python packages and modules, the
  same enforcement Rust and TS get.
- **Expected Outcome:** Cross-layer Python imports surface as boundary
  violations with verbatim `.py` paths; external imports never do.
- **Validation:** `python_resolve` unit tests (absolute flat/`src`, relative
  single/double-dot, bare-dot package init, climb-above-root drop, external
  drop, malformed drop, star-import module); gate integration tests proving an
  end-to-end cross-layer Python violation and that stdlib imports drop out.
- **Files:** `crates/anvil-architecture/src/python_resolve.rs`,
  `crates/anvil-architecture/src/lib.rs`,
  `crates/anvil-architecture/src/validator.rs`,
  `crates/anvil-cli/src/commands/gate.rs`
- **Dependencies:** PYLAN-002

---

#### PYLAN-007: Drift baseline default-on for `.py`

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-17 via #2734 — `.py` added to
  `AntipatternCheckConfig`'s default scan extensions, so the Python rules fire
  across `anvil check`/`gate`/drift and the save-time daemon by default
  (mirroring RSTLAN-006 for `.rs`).
- **Intent:** Make Python drift/anti-pattern scanning default-on.
- **Expected Outcome:** `.py` is in the default scanned-extension set.
- **Validation:** Test asserts `.py` in `AntipatternCheckConfig::default()`.
- **Files:** `crates/anvil-checks/src/antipattern/types.rs`
- **Dependencies:** PYLAN-003

---

#### PYLAN-008: `architecture-validate` includes Python packages

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-17 via #2732 — `.py` added to
  `validator::collect_source_files`'s `include_extensions`, so the public
  validate surface (CLI / MCP / dashboard) enumerates and layer-assigns Python
  files. Layer assignment is path-glob based, so no language gate is needed;
  the Python boundary edges come from PYLAN-006's resolver. Delivered alongside
  PYLAN-006.
- **Intent:** Surface Python packages and module graphs in
  `anvil architecture validate`.
- **Expected Outcome:** `.py` files appear in the validate surface and their
  cross-layer edges are reported, with no "Python ignored" silent path.
- **Validation:** Validator tests — `.py` layer assignment and a Python
  cross-layer violation through `validate_with_files_and_edges`.
- **Files:** `crates/anvil-architecture/src/validator.rs`
- **Dependencies:** PYLAN-006

---

### Acceptance (all nine items Merged; module → Complete on a release tag)

#### PYLAN-009: Dogfood T3 acceptance + FP bar (§16.5 #9)

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-06-18 via #2740 — external-codebase validation
  (spec §16.5 #9 / C-014) executed on public OSS at the operator's direction:
  `encode/httpx` + `Textualize/rich` (~270 `.py` files, 0 panics, 0 parse
  errors), FP rate **0.0% < N = 1%** (**N accepted by the operator 2026-06-18**).
  The catalogue as shipped scored **2.9% FP** (1 mis-detection:
  `except:` inside a string literal); three precision fixes landed there drop it
  to **0.0% FP** default-on and 0% on the opt-in PY-006/PY-007 surfaces. Fixes:
  PY-005 allowlists `**/__init__.py` (re-export idiom), PY-007 uses a
  subscript-context `Any` match (no `from typing import Any` FP), and PY-004
  joins the comment/string-masked view (`rule_is_code_scoped`). Evidence:
  [`plans/reviews/2026-06-18-pylan-009-external-validation.md`](../reviews/2026-06-18-pylan-009-external-validation.md).
  **N = 1%** (accepted; observed 0.0%). NOTE: Anvil itself has ~no Python, so the
  "own repo" half of the bar is discharged via the public-OSS external run.
- **Intent:** Demonstrate the full Python T3 stack on real-world code at an
  acceptable FP rate per §16.5 #9.
- **Expected Outcome:** ≥1 external-codebase run with FP rate < N%; evidence
  recorded.
- **Validation:** httpx + rich classification (TP vs FP) in the evidence note;
  per-fix regression tests in `python_antipatterns.rs`; full `anvil-checks`
  green.
- **Files:** `patterns/python-reliability/PY-004.anvil` (via code-scoping),
  `PY-005.anvil`, `PY-007.anvil`, `patterns/compiled/registry.json`,
  `crates/anvil-checks/src/antipattern/scanner.rs`,
  `crates/anvil-checks/tests/python_antipatterns.rs`,
  `plans/reviews/2026-06-18-pylan-009-external-validation.md`
- **Remaining for module → Complete (not for this item):** all nine work items
  are Merged, but the module stays In Progress until a release tag ships them
  (Released/Shipped → Complete), per the APS lifecycle. Still-open governance
  housekeeping, none blocking the shipped behaviour: name the anchor owner; run
  the §16.5 #8 re-scoring gate; (optional) re-run against the specific
  User B / User C codebases when available.
- **Dependencies:** PYLAN-003, PYLAN-005, PYLAN-006

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
