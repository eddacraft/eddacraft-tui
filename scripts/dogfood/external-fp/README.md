# External-codebase false-positive dogfood

A repeatable, pinned harness that runs the Anvil anti-pattern catalogues over
diverse, idiomatic **external** OSS repositories and quantifies the genuine
false-positive rate (council §16.5 #9 bar). It is the external-corpus companion
to the in-repo internal dogfood test
(`crates/anvil-checks-ast/tests/dogfood.rs`, RSTLAN-008): that test guards Anvil
against its own source; this harness guards the rule catalogues against code
nobody on the team wrote, which is where idiomatic-noise false positives hide.

## Why this exists

The original RSTLAN-008 (Rust) run only dogfooded Anvil's own crates, and the
TS/JS family (LANGTS) shipped with no external multi-repo FP run at all. The
Python run (PYLAN-009) that scanned `httpx`/`rich` immediately surfaced a real
bug (`PY-004` firing inside a string literal). This harness makes that kind of
run a first-class, repeatable procedure for **every** language anchor so the
same churn doesn't recur on a future grammar bump or rule edit.

## Corpus

`corpus.json` pins three repos per language group at **fixed commit SHAs** (so
re-runs are deterministic), each chosen to adversarially stress specific rules:

| Group  | Repos                     | Adversarial intent                                                          |
| ------ | ------------------------- | --------------------------------------------------------------------------- |
| rust   | ripgrep, tokio, alacritty | RS-001/002 exclusions; RS-003 documented-`unsafe`; RS-004 opt-in            |
| langts | zod, vite, excalidraw     | AP-015/016 Zod rules; AP-008/009 bundler eval; AP-003/GS-001 type machinery |

Bump a SHA deliberately (e.g. to re-test after a grammar update) and record the
bump in the evidence writeup.

## Run

```bash
# 1. Build (or reuse) an anvil binary out-of-tree so it doesn't fight other work.
CARGO_TARGET_DIR=~/.cache/anvil-targets/ext-fp \
  cargo build -p eddacraft-anvil --bin anvil

export ANVIL_BIN=~/.cache/anvil-targets/ext-fp/debug/anvil

# 2. Clone the pinned corpus + scan (default catalogue + --include-opt-in passes).
./run.sh all          # or: ./run.sh rust | ./run.sh langts

# 3. Build the TP/FP worksheet (joins each finding to its source line).
python3 classify.py all

# 4. Fill in the `Verdict` column (TP / FP) in the generated worksheets, then:
python3 classify.py all --score   # prints the genuine-FP rate per group
```

Artifacts land in `$EXT_FP_OUT` (default `/tmp/anvil-ext-fp/out`):
`<repo>.default.json`, `<repo>.optin.json`, `<repo>.arch.json` (+ `.err`), and
`<group>.worksheet.md`.

## What counts

- **Default-catalogue findings** drive the §16.5 #9 genuine-FP rate. Opt-in
  findings (`--include-opt-in`) are characterised separately — an opt-in rule
  firing widely is _noise to be left off by default_, not a false positive (this
  is how RSTLAN-008 dispositioned RS-004).
- **Genuine FP** = the rule matched something that is not the thing it claims to
  detect (e.g. `except:` inside a string literal). Idiomatic-but-real matches
  (e.g. `import *` in an `__init__.py` re-export) are **true positives** with a
  noise note, not FPs.
- **Zero-panic / clean-parse bars**: `run.sh` greps each repo's stderr for
  panics; parse-skips show up as the `AST_PARSE_SKIP_ID` finding id.
- `architecture validate` needs `.anvil/architecture.yaml`, which external repos
  lack, so it reports N/A here; the language rule families
  (`RS-*`/`AP-*`/`PY-*`) all surface through `anvil check`'s antipattern-scan,
  which is what this harness exercises.

## Evidence

Write the per-group verdict to the path named in
`corpus.json#groups.<g>.evidence`:

- Rust → `plans/reviews/2026-06-18-rstlan-external-fp.md`
- TS/JS → `plans/reviews/2026-06-18-langts-external-fp.md`

Mirror the RSTLAN-008 writeup shape: corpus + commit pins, parse/panic bars,
findings by rule, the FP classification with each FP class and its fix, and the
final genuine-FP rate. Any FP fix should land a regression fixture in the owning
crate's tests so the harness's finding becomes a standing guard.
