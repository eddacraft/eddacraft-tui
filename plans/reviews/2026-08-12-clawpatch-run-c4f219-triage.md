# Clawpatch triage — 2026-08-12 (run `c4f219`)

**Run:** `20260812T062029-c4f219`  
**Scope:** 20 claimed features (`--limit 20 --jobs 5`), 24 new findings  
**Tooling:** clawpatch `0.7.2` (pnpm global)  
**Corpus:** worktree at map/review time (`HEAD` `24070b867`)  
**Predecessor:** `plans/reviews/2026-08-11-clawpatch-rust-source-groups-triage.md`

## Why this run

Operator-driven review batch after repairing corrupt local finding history
entries that blocked post-review loads (`history[].note` missing from
`clawp-fix*` side writes). This document is the durable triage record;
local verdicts live in `.clawpatch/findings/` (gitignored).

## Corpus summary

| Metric | Value |
| --- | ---: |
| Features reviewed | 20 |
| Findings produced | 24 |
| Open after triage | 23 |
| False-positive | 1 |
| Open high | 4 |
| Open medium | 17 |
| Open low | 2 |

### By category (all 24)

| Category | Count |
| --- | ---: |
| api-contract | 8 |
| concurrency | 4 |
| bug | 4 |
| security | 3 |
| data-loss | 2 |
| performance | 1 |
| test-gap | 1 |
| build-release | 1 |

## Method

1. Inventory every finding id from the run record.  
2. **Verify-first** against current source for all highs, then every medium.  
3. Record dispositions with `clawpatch triage` (notes on open; one FP).  
4. Rank remaining work into fix waves (P0–P2).

## Session verdicts

### False-positive

| Finding | Verdict | Basis |
| --- | --- | --- |
| Declared CLI command name is not built by Cargo | **false-positive** | Feature title uses Cargo **package** name `eddacraft-anvil-dashboard-server`. `[[bin]]` is `anvil-dashboard-server` in `crates/anvil-dashboard-server/Cargo.toml`. Mapper artefact, not a missing binary. |

```bash
clawpatch triage --finding fnd_sig-feat-cli-command-80b5a97294-_c201575f0f \
  --status false-positive \
  --note "FP: package name vs [[bin]] name; binary is anvil-dashboard-server"
```

### Verified **open** (high)

| Finding | Path | Why confirmed |
| --- | --- | --- |
| Concurrent baseline saves share one temp path | `crates/anvil-baseline/src/io.rs` | Fixed `anvil/.baseline.json.tmp`; concurrent writers can interleave; Windows remove-then-rename widens the race |
| Compose drops surviving re-export edges into modified overlay | `crates/anvil-graph-cache` | `base_reresolve` is Imports-only; golden `reexport_call_divergence` records the intentional gap vs cold scan |
| Husky bootstrap omits Git hook entrypoints | `crates/anvil-hook/src/bootstrap.rs` | Only `.husky/_/h` + `husky.sh`; real Husky needs `.husky/_/<hook>` with `core.hooksPath=.husky/_`; generated `h` expects `$1` not `basename $0` |
| Malformed markers can erase user content on uninstall | `crates/anvil-hook/src/coexistence.rs` | Unmatched `MARKER_BEGIN` treated as absent → append new block → uninstall spans old begin to new end |

### Verified **open** (medium) — sample with code evidence

| Finding | Why confirmed |
| --- | --- |
| `detect_circular` inert | Option never read in `validate_files` / `check_boundaries` |
| `autolib = false` ignored | `PackageSection` has `autobins` only; always infers `src/lib.rs` |
| Witness cutoff / project_uuid not enforced | Verifier only checks genesis anchor + hash chain |
| Exception schema on `add` / TOCTOU load | `add` does not reject bad versions; symlink check then separate `open` |
| Trace file interleave | Per-`write` mutex on `SharedTraceWriter` |
| TUI unbounded drain | `drain_events` loops until channel empty |
| NAPI registry double-load | Load-or-err then discard; helper reloads |
| `annotate_trust` partial imports | Privileges from caller slice only (kernel currently passes full list) |
| Windows `atomic_write` data loss | `remove_file` then `persist` |
| Base-graph 8 MiB incomplete | `wait_with_output` still buffers full batch stdout (commented follow-up) |
| Air-gap test weak exit assert | Only requires non-signal exit |
| Secret/reasoning vs Removed | Antipattern skips deletions; these rules do not |
| Layer-overlap witness sampling | Finite witnesses miss some glob intersections |
| Husky `exec` blocks after-marker | Coexistence intent loses host commands after managed block |
| `discover` basename escape | Unvalidated `basename` can `../` out of `dir` (call sites mostly fixed strings) |

### Verified **open** (low)

| Finding | Why confirmed |
| --- | --- |
| `DepthExhausted` unvisited PID | `max_depth=0` returns start PID without visiting |
| Fan-out counts edges not targets | Repeated calls to one callee mark heuristic overload |

## Fix waves

### P0 — enforcement / recovery / data integrity

| Title | ID | Area |
| --- | --- | --- |
| Husky bootstrap omits hook entrypoints | `fnd_sig-feat-library-9f5673610a-345d_fdd883f516` | hook bootstrap |
| Malformed markers erase user content | `fnd_sig-feat-library-9f5673610a-eab6_e8b33972c4` | hook coexistence |
| Concurrent baseline temp path | `fnd_sig-feat-library-09a1363c0c-dd6b_c754ec6784` | baseline IO |
| Compose drops re-export edges | `fnd_sig-feat-library-5d338c4f1f-df79_3b0aafab0a` | graph-cache compose |

**Suggested order:** hook bootstrap entrypoints + marker parse safety → unique baseline temp → compose re-export/call re-resolve (or explicit cold-scan fallback).

### P1 — security / trust / contract that can mislead gates

| Title | ID |
| --- | --- |
| Exception store symlink TOCTOU | `fnd_sig-feat-library-7bd1da826d-891a_0ee835de52` |
| Exception unsupported schema via `add` | `fnd_sig-feat-library-7bd1da826d-5da6_349af05e45` |
| Partial imports declassify privileged files | `fnd_sig-feat-library-5d338c4f1f-b156_a959aa14e6` |
| Witness cutoff_commit semantics | `fnd_sig-feat-library-549f398eb4-028d_a579130ea6` |
| Witness project_uuid identity | `fnd_sig-feat-library-549f398eb4-f364_e86cb12d5e` |
| NAPI registry health vs scan path | `fnd_sig-feat-library-5f9e6b4709-2d50_bf594a29b1` |
| Secret/reasoning block deletions | `fnd_sig-feat-library-6b12f2fa2a-eb69_36c05ffc0e` |
| Husky `exec` skips after-marker host hooks | `fnd_sig-feat-library-9f5673610a-277a_5599b0f48e` |
| Windows baseline/YAML replace loss | `fnd_sig-feat-library-767142007b-3624_6a23809cc2` |

### P2 — correctness polish / perf / low

| Title | ID |
| --- | --- |
| `detect_circular` inert | `fnd_sig-feat-library-767142007b-011e_832f2ae14e` |
| `autolib` ignored | `fnd_sig-feat-library-767142007b-8679_569e9cba25` |
| Layer-overlap witness gaps | `fnd_sig-feat-library-767142007b-050f_41c88f27e3` |
| Trace record interleave | `fnd_sig-feat-library-92b5c75519-bca3_c0ce8def24` |
| TUI unbounded drain | `fnd_sig-feat-library-3caf637119-5282_0abdc64004` |
| Base-graph batch memory | `fnd_sig-feat-cli-command-ba5ccdd3a6-_c5445f44fe` |
| Air-gap exit codes | `fnd_sig-feat-cli-command-171a82ab94-_d6c684ec34` |
| `discover` basename hygiene | `fnd_sig-feat-library-105b94af10-bc81_a6bc357d8d` |
| Fan-out edge count | `fnd_sig-feat-library-5d338c4f1f-2614_a776be05d9` |
| DepthExhausted docs/API | `fnd_sig-feat-library-00294b16bd-be10_2dee9063ff` |

Do not start a fix-all loop. Prefer:

```bash
clawpatch fix --finding <id>
# or
clawpatch revalidate --finding <id>
```

## Side notes (tooling)

- Nine local findings had invalid `history[]` entries written by a non-schema
  `clawp-fix*` path (missing `note` / alternate `{at,by}` shape). Those were
  normalised before this triage so loads no longer fail.
- Prefer `clawpatch triage --status fixed --note '…'` for closeout so history
  stays schema-valid.
