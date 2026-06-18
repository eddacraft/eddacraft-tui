# PYLAN-009 — external-codebase validation evidence

**Date:** 2026-06-18
**Scope:** Discharge the spec §16.5 #9 (C-014) acceptance bar for Python —
"FP rate < N% on Anvil's repo AND ≥1 external codebase validation run" — and
propose `N` from the observed rate.

## Why an external corpus was required

RSTLAN-008 satisfied the bar mostly by dogfooding Anvil's own crates (571 Rust
files). Anvil contains **2 Python files** total (one benchmark prototype, one
test fixture), so the "Anvil's own repo" half is vacuous for Python — the
external run is the load-bearing evidence. The operator elected a public OSS
corpus of the author's choosing.

## Corpus

| Repo | Why | `.py` files |
| ---- | --- | ----------- |
| [`encode/httpx`](https://github.com/encode/httpx) | modern, fully type-annotated, `src/`-free flat layout | ~60 |
| [`Textualize/rich`](https://github.com/Textualize/rich) | large; `Console.print()` everywhere — adversarial test for the PY-006 `.print()` exclusion | ~213 |

~270 Python files, same order of magnitude as RSTLAN-008's Rust dogfood.

## Method

`anvil check --all` (antipattern catalogue) over each repo, JSON output; every
PY-* finding classified by inspecting the flagged line as **true positive**
(rule fired on a real instance of the targeted pattern) or **false positive**
(mis-detection). Test files are auto-excluded by each rule's allowlist. A
separate `--include-opt-in` pass exercised PY-006/PY-007.

## Results — first pass (catalogue as shipped in #2734)

| Surface | Findings | False positives | Rate |
| ------- | -------- | --------------- | ---- |
| Default-on (PY-001..005) | 34 (httpx 32, rich 2) | **1** | **2.9%** |
| Parse robustness | ~270 files | 0 panics, 0 parse errors | — |

The 1 FP: `PY-004` matched `except:` **inside a string literal**
(`rich/traceback.py:319` — `"... not called in except: block"`); the regex tier
has no string/comment masking. Opt-in: `PY-006` 67 findings, **0** `.print()`
method-call FPs (the `(^|[^.\w])print` exclusion held on rich); `PY-007` 131
findings, **5** FPs (`from typing import …, Any` matched the `, Any` branch —
importing the name, not annotating).

Two precision signals (correct detections, but noise): **16/16 `PY-005`
findings were `__init__.py` re-export `from .sub import *`** — idiomatic; and the
two FP shapes above.

## Fixes applied (this PR)

1. **PY-005** allowlists `**/__init__.py` — package-API re-export is the one
   conventional wildcard use.
2. **PY-007** pattern → `\[[^\]]*\bAny\b` (subscript context) replaces the bare
   `[\[,]\s*Any` branch, so `Dict[str, Any]` / `List[Any]` still fire but
   `from typing import …, Any` does not.
3. **PY-004** joins `rule_is_code_scoped` — it runs against the comment/string
   masked view, so `except:` inside a string literal no longer matches.
   (PY-001/-002/-003 stay out: they are `#`-comment rules by nature.)

Each has a regression test in `python_antipatterns.rs`.

## Results — after fixes

| Surface | Findings | False positives | Rate |
| ------- | -------- | --------------- | ---- |
| Default-on (PY-001..005) | 17 (httpx 16, rich 1) | **0** | **0.0%** |
| Opt-in PY-006 | 67 | 0 method-call FP | 0.0% |
| Opt-in PY-007 | 115 | 0 import-line FP | 0.0% |
| Parse robustness | ~270 files | 0 panics, 0 parse errors | — |

The surviving default-on findings are all true positives: 11×PY-001 (real bare
`# type: ignore`), 5×PY-002 (real bare `# noqa`), 1×PY-004
(`rich/logging.py:298`, a real bare `except:` in a demo block).

## Proposed N

**N = 1%** for the shipping default-on catalogue, observed **0.0%** on this
corpus. Rationale: the Rust precedent effectively held 0%; a non-zero allowance
exists only for the rare regex-tier edge case (e.g. an anti-pattern token inside
a Python triple-quoted docstring code example, which this corpus did not
exhibit). A 5% bar would be too loose — at default-on severity it is a UX
regression to let one-in-twenty findings be noise.

## Residual / known limitations (not blocking)

- Python `#` comments and triple-quoted docstrings are **not** masked by the
  shared (JS/TS-oriented) `mask_non_code_lines`; the measured FPs did not need
  it, but a future Python-aware masking pass would close the theoretical gap for
  PY-006/PY-007 occurrences inside `#` comments / docstring code examples.
- `cast(Any, x)` is not flagged by PY-007 (no `:`/`->`/`[` context) — a known
  opt-in gap.

## Still operator-gated (for PYLAN-009 → done, and module → Complete)

- Accept the proposed **N = 1%** as the governance bar.
- Name the anchor owner; run the anchor re-scoring gate (§16.5 #8).
- The specific **User B / User C** codebases (this run used public OSS at the
  operator's direction); fold in if/when those become available.
- Module advances to **Complete** only with a release tag, per the APS lifecycle.
