# CIB-083 Mini Council — Template Interpolation Masker

| Type   | Authority | Owner | Status |
| ------ | --------- | ----- | ------ |
| Review | Advisory  | CIB   | Closed |

## Scope

- Work item: CIB-083
- Target: `crates/anvil-checks/src/antipattern/mask.rs`
- Review tier: mini
- Roles: general, adversarial
- Date: 2026-06-26

## Verdict

**PASS with constraints → constraints addressed in implementation.**

The current scalar `TemplateInterp(depth)` model could not distinguish braces in
interpolation strings/comments/regexes from interpolation delimiters. The
implementation replaces it with a carried context stack, preserving byte offsets
while masking interpolation literals and keeping real interpolation code visible.

## Findings and resolution

| Severity | Role | Finding | Resolution |
| -------- | ---- | ------- | ---------- |
| Major | Adversarial | Braces inside interpolation strings/comments/regexes could close interpolation early and hide real AP-003/GS-001 code. | Added stack frames for strings, comments, regexes, template text, and interpolation code. |
| Major | Adversarial | Multiline interpolation carry needed a parent-aware stack, not `TemplateInterp(depth)`. | `Carry` now holds the full frame stack across lines. |
| Major | General | Nested templates must mask template text while preserving nested and parent interpolation code. | Added nested-template masking and scanner tests for visible real non-null assertions. |
| Major | General | Byte offsets and AP-003/GS-001 scanner integration needed proof, not only masker tests. | Added scanner-level AP-003/GS-001 tests including multibyte column preservation. |

## Evidence

- `cargo test -p eddacraft-anvil-checks`
- `cargo clippy -p eddacraft-anvil-checks --all-targets -- -D warnings`
- `cargo fmt --check`
- External TS/JS FP dogfood:
  - `ANVIL_BIN=/tmp/anvil-cib083-target/debug/anvil EXT_FP_WORK=/tmp/anvil-ext-fp-cib083 EXT_FP_OUT=/tmp/anvil-ext-fp-cib083/out scripts/dogfood/external-fp/run.sh langts`
  - `EXT_FP_OUT=/tmp/anvil-ext-fp-cib083/out python3 scripts/dogfood/external-fp/classify.py langts`

## External FP summary

The CIB-083 run over the pinned `langts` corpus completed with **0 panics**.

| Rule | Findings |
| ---- | -------- |
| AP-001 | 7 |
| AP-003 | 1386 |
| AP-004 | 61 |
| AP-006 | 31 |
| AP-009 | 4 |
| AP-015 | 50 |
| GS-001 | 681 |

AP/GS default findings totalled **2,220**, down from the prior documented
**2,228**. The delta is the eight known GS-001 template-text false positives
from the 2026-06-18 TS/JS dogfood review.

Artifacts were written under `/tmp/anvil-ext-fp-cib083/out`, including
`langts.worksheet.md`.
