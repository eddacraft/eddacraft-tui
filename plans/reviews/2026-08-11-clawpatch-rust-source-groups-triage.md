# Clawpatch triage — 2026-08-11 (rust source-group wave)

**Run:** `20260810T195659-92de45`  
**Scope:** all features with `source: rust-source-group` (126 slices) after
deepening the Rust mapper  
**Tooling:** local clawpatch build with rust source groups (not yet required
from a published release)  
**Corpus:** `main` worktree at map/review time  
**Predecessor:** `plans/reviews/2026-08-07-clawpatch-triage.md`  
**Report:** `.clawpatch/reports/20260810T195659-92de45.md` — **local only**,
under the gitignored `.clawpatch/` state directory; not committed and not
retrievable from this repository. This document is the durable record.

## Why this run

Heuristic map previously owned mostly crate entrypoints (`lib.rs` / bins /
integration tests). Source-group slices now own module trees under each crate
`src/`, lifting inventory ownership roughly **42% → ~88%**. This wave is the
first review of those module slices.

Local clawpatch verdicts live in `.clawpatch/` (gitignored). Durable record is
this doc.

## Corpus summary

| Metric | Value |
| --- | ---: |
| Features reviewed (this run) | 126 |
| Findings produced | 184 |
| Open after session | 182 |
| False-positive (session) | 2 |
| Open high | 35 |
| Open medium | 129 |
| Open low | 18 |

### By category (all 184)

| Category | Count |
| --- | ---: |
| bug | 60 |
| security | 44 |
| api-contract | 35 |
| concurrency | 18 |
| data-loss | 15 |
| performance | 8 |
| build-release | 4 |

Scanner auto-triage was almost all `confirmed-bug` / `contract-mismatch` with
**high** confidence (174/184). That overstates trust: many mediums are real but
local; a few highs are slice-context false positives.

### By crate (open high evidence root)

| Crate | Open high |
| --- | ---: |
| anvil-cli | 12 |
| anvil-intercept | 7 |
| anvil-checks | 4 |
| anvil-hook | 3 |
| others (1 each) | 9 |

## Method

1. Inventory all findings on `rust-source-group` features.  
2. **Verify-first** on every **open high** claim against current source (not
   line-trust alone).  
3. Spot-check representative mediums only where they cluster with a verified
   high (same file/subsystem).  
4. Record dispositions with `clawpatch triage` for clear false-positives.  
5. Rank remaining work into fix waves (P0–P2).

## Session verdicts recorded

| Finding | Verdict | Basis |
| --- | --- | --- |
| Crate has no library entry point (`anvil-attribution`) | **false-positive** | `crates/anvil-attribution/src/lib.rs` exists. Source groups **exclude** entrypoints by design. |
| Declared library target has no crate root (`anvil-l4`) | **false-positive** | `crates/anvil-l4/src/lib.rs` exists; same entrypoint exclusion. |

```bash
clawpatch triage --finding fnd_sig-feat-library-1667bea77f-d110_187a1c3bad \
  --status false-positive \
  --note "lib.rs exists; source-group slices exclude crate entrypoints"
clawpatch triage --finding fnd_sig-feat-library-9a85b0b246-09ad_3fd0bd7b7d \
  --status false-positive \
  --note "lib.rs exists; source-group slices exclude crate entrypoints"
```

### Verified **confirmed** (high) — sample with code evidence

| Finding | Path | Why confirmed |
| --- | --- | --- |
| Watcher runtime never consumes filesystem events | `crates/anvil-intercept/src/watcher.rs` | `recv_blocking` ignores `_rx` and `pending().await` forever — production `run` never ingests batches |
| `anvil doctor` write probe can truncate symlink target | `crates/anvil-cli/src/commands/doctor.rs` | Fixed `.anvil/.write-test` + `fs::write` follows symlink and truncates |
| NUL buffer reported clean | `crates/anvil-intercept/src/midedit.rs` | `content.contains(&0)` returns empty successful `ScanBufferResponse` |
| Hard-pinned rules disabled via scalar `"off"` | `crates/anvil-config` | `RuleMode::parse("off")` accepted; hard-pin validator rejects object/bool forms but **not** bare string `"secrets": "off"` |
| `policy test` succeeds without executing tests | `crates/anvil-cli/src/commands/policy` | Stub returns `Ok(())` with skipped count / warning only |
| Export can overwrite source plan | `crates/anvil-cli/src/commands/export.rs` | No source≠output guard before `fs::write` |
| Nested wrappers beyond depth 5 evade command-safety | `crates/anvil-checks/.../parser.rs` | `MAX_UNWRAP_DEPTH` then token match on still-wrapped form |
| Worktree consent file enables snippet egress | `crates/anvil-intercept/src/egress_consent.rs` | Regular file under repo-relative `anvil/witness/` (gitignored, still worktree-writable) is treated as operator consent |

## Fix waves

### P0 — security / enforcement bypass (fix first)

Trust boundary, false clean, or protection disable. Prefer small PRs per
subsystem.

| Title | ID | Area |
| --- | --- | --- |
| Watcher runtime never consumes filesystem events | `fnd_sig-feat-library-b3e4d50fda-0fa3_33c6c7144b` | intercept watcher |
| NUL buffer reported as clean | `fnd_sig-feat-library-a677918f51-fa46_305872e606` | intercept midedit |
| Doctor symlink truncate via write probe | `fnd_sig-feat-cli-command-ff8c453ff8-_7459c9cae4` | cli doctor |
| Hard-pinned rules via scalar `"off"` | `fnd_sig-feat-service-e7c0675adb-af82_9d88a08c76` | config validation |
| Nested wrappers bypass command-safety | `fnd_sig-feat-library-4c7483fa0f-c04a_c66e8db158` | command_safety |
| Shell-equivalent paths evade lexical rules | `fnd_sig-feat-library-4c7483fa0f-d9c9_19bfa0bf76` | command_safety |
| Flow-mapping GHA bypass | `fnd_sig-feat-service-e706bccd06-bf0a_0fc0470f18` | checks/gha |
| Git-history omits lockfile URL credentials | `fnd_sig-feat-library-96da0b4ef1-9b1d_39d90facaf` | secret history |
| Worktree consent enables snippet egress | `fnd_sig-feat-library-40c2cde0d8-ee12_b4c4c89514` | gctx consent |
| Byte ledger overlapping reassembly | `fnd_sig-feat-library-af98120000-0836_cd0a8625fe` | gctx-egress |
| Manifest symlink pack escape | `fnd_sig-feat-library-4c0f69d393-bb5a_e1e21a3ab7` | policy-engine |
| Fixed temp policy + symlink race | `fnd_sig-feat-library-9a85b0b246-50e6_192ab575f9` | l4 |
| Edda index path escape | `fnd_sig-feat-cli-command-ff8c453ff8-_5503f7c7b4` | cli edda |
| Release builds trust dev signing key | `fnd_sig-feat-cli-command-c2cc6bd208-_e6b2eeb4df` | update/sign |
| Special-char names bypass L4 validation | `fnd_sig-feat-cli-command-79ebbc42f6-_ddb9293a0c` | cli L4 path |
| Unknown nested changes fence directory not worktree | `fnd_sig-feat-library-b3e4d50fda-bac3_9662b05f31` | intercept fence |

**Suggested order inside P0:** intercept watcher + midedit NUL → doctor probe →
config hard-pin scalar → command_safety pair → secret/gha → consent/gctx.

### P1 — data-loss / false success / reliability

| Title | ID |
| --- | --- |
| Export destination can overwrite source | `fnd_sig-feat-cli-command-ee59c35924-_42b63aba64` |
| `policy test` reports success without running tests | `fnd_sig-feat-cli-command-075ddad274-_9758939831` |
| In-place fix write partial truncation | `fnd_sig-feat-cli-command-774cae3cf1-_bda0b96fcb` |
| Suppression write truncation | `fnd_sig-feat-cli-command-b4aca56a4f-_d89ac26e83` |
| Inline TUI save overwrites with window only | `fnd_sig-feat-library-59c662a7e7-06f3_d47210bdf4` |
| GC treats upstream errors as “no upstream” | `fnd_sig-feat-library-895b9882f0-dc8b_d96805a06d` |
| Synthetic-repo cleanup can delete pre-existing dir | `fnd_sig-feat-library-c4aca41ba1-ebb1_d5459bfe54` |
| Full gate subprocess pipe deadlock | `fnd_sig-feat-cli-command-774cae3cf1-_cb8c23f173` |
| Timed-out scans free slot while worker continues | `fnd_sig-feat-library-a677918f51-5ae2_30f0fea4b4` |
| Late scan restores timed-out workspace to Clean | `fnd_sig-feat-library-40c2cde0d8-7c6e_77ad509e9d` |
| Husky runtime swallows hook failures | `fnd_sig-feat-library-4ca5c0b455-d109_78e64607ab` |
| Husky coexistence aborts when anvil missing | `fnd_sig-feat-library-4ca5c0b455-6c14_743120d261` |
| Coexistence install never writes Lefthook/pre-commit | `fnd_sig-feat-library-4ca5c0b455-cc97_635a946825` |

### P2 — contract / concurrency polish / low

Remaining open high concurrency/api-contract items (refresh-token serialise,
MCP attestation promotion, capsule range binding, WatcherHandle drop nuance,
suite timeout join, Claude MCP config path) plus **all open mediums (129)** and
**lows (18)**.

Do not start a fix-all loop. Prefer:

```bash
# after a fix
clawpatch fix --finding <id>
# or targeted revalidate
clawpatch revalidate --finding <id>
```

## Process notes

1. **Source-group blind spot:** any claim that a crate “has no `lib.rs`” while
   reviewing a `rust-source-group` feature is almost certainly FP — entrypoints
   are owned by `rust-library` / `rust-command` features instead.  
2. **TOCTOU / race findings** (symlink swap, concurrent refresh) are usually
   real but need careful fix scope; keep as P0 only when they cross a security
   boundary, else P1.  
3. **Spike / publish=false crates** still deserve correctness if linked from
   production paths; do not auto-wont-fix.  
4. Medium **security** cluster (~27) should get a second pass after P0 — many
   are symlink races and soft fail-open modes adjacent to confirmed highs.

## Recommended next actions

1. Open fix PRs for **P0 intercept pair** (watcher `recv_blocking` + NUL clean)
   first — highest product impact, small surface.  
2. Then **doctor probe** + **hard-pin scalar `"off"`** (config).  
3. Export a periodic audit JSON when the open-high count drops, mirroring
   `plans/audits/2026-08-07-clawpatch-periodic-scan.json`.  
4. Keep this doc as the queue; do not re-review all 126 slices until the next
   mapper or major `main` delta.

## Exit state

| | |
| --- | --- |
| Open findings (rsg wave) | 182 |
| Open high | 35 |
| FPs closed this session | 2 |
| Durable triage | this file |
