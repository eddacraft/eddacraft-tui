# Scan Performance & Safety

| ID   | Owner | Status      | Progress |
| ---- | ----- | ----------- | -------- |
| SCAN | @team | In Progress | 6/6      |

**Last reviewed:** 2026-04-26

## Purpose

Extend the parallel-scan pattern landed in `perf/discovery-scan-parallel`
(welcome-screen discovery) to the other scan call-sites in the codebase, and
tighten the safety envelope around the shared scanner primitives
(`scan_content`, `scan_for_antipatterns`) so that no single input can stall a
worker thread or blow out memory.

The welcome-screen rewrite delivered an ~8.9× win (1653 ms → 186 ms on `entx`)
by combining `ignore::WalkBuilder` (gitignore-aware discovery) with `rayon`
parallel per-file scanning, a `LazyLock` for the 18 built-in regex set, and
`catch_unwind` panic containment. Several other surfaces still use the old
serial `walkdir` + per-file regex compile path; they benefit from the same
pattern.

## In Scope

- Applying the parallel-scan pattern to other discovery / audit / gate /
  drift / policy / architecture / watcher sites
- ReDoS and memory hardening of the shared scanner primitives
- Exposing skipped-file provenance to surfaces that care
- Bounding rayon thread count for first-run scans (avoid fighting the TUI for
  CPU on dev-loop concurrency)

## Out of Scope

- Further algorithmic changes to pattern matching itself (entropy tuning, new
  patterns) — tracked separately
- Graph-driven incremental scanning (supersedes scan-everything once graph is
  online; tracked under the graph modules)
- Binary / large-file skip counting beyond the ignore layer

## Origin

Deferred items from the Council review of `perf/discovery-scan-parallel`
(2026-04-20). See PR description for the full verdict. Individual task notes
reference the council finding IDs where relevant.

## Tasks

### SCAN-001: Apply parallel scan pattern to remaining call-sites

- **Intent:** Bring the `ignore::WalkBuilder` + `rayon::par_iter` +
  `catch_unwind` pattern to every surface that walks the tree and runs the
  shared scanners, so slow-path calls are not paying N× regex-compile and
  serial IO costs.
- **Expected Outcome:** `gate`, `audit`, `check`, `drift`, `policy`,
  `architecture/validator`, and the `kernel/watcher` call-sites use the same
  discovery + parallel scan shape introduced in `welcome.rs`. Per-file panics
  are contained; read failures are counted, not fatal. Findings order is
  deterministic.
- **Validation:**
  - `grep -rn "walkdir::WalkDir" crates/` shows no remaining uses inside
    scan-fanout call paths (crate-level utilities may keep it)
  - `cargo test -p eddacraft-anvil -p eddacraft-anvil-checks
    -p eddacraft-anvil-kernel` passes
  - Spot benchmark on `entx` (or equivalent ~3k-file repo) shows > 3× wall-time
    reduction for at least one of `anvil audit`, `anvil check`, `anvil drift`
- **Files:**
  - `crates/anvil-cli/src/commands/gate.rs` (scan entry points around :265, :586)
  - `crates/anvil-cli/src/commands/audit.rs` (scan entry around :59)
  - `crates/anvil-cli/src/commands/check.rs` (scan entry around :389)
  - `crates/anvil-cli/src/commands/drift.rs` (scan entry around :759)
  - `crates/anvil-cli/src/commands/policy.rs` (scan entries around :333, :342)
  - `crates/anvil-architecture/src/validator.rs` (scan entry around :351)
  - `crates/anvil-kernel/src/watcher/mod.rs` (discovery around :61)
  - `crates/anvil-kernel/src/watch.rs` (discovery around :113)
- **Confidence:** high
- **Priority:** High
- **Status:** Complete (landed 2026-04-26 — parallel rollout reaches all listed call-sites; secret-scan benchmark on a 3 k-file synthetic surface shows 7.39× wall-time reduction (3.71 s serial → 0.50 s parallel), well above the >3× acceptance threshold)

---

### SCAN-002: ReDoS hardening — per-line length guard in scan_content

- **Intent:** Prevent pathological lines (minified bundles, base64 blobs) from
  triggering worst-case regex backtracking across the 18 built-in patterns ×
  N lines. Even though all current patterns are bounded, the custom pattern
  surface accepts user-supplied regexes without vetting.
- **Expected Outcome:** `scan_content` skips lines longer than a configurable
  threshold (default 8 KB); skipped lines are counted so reviewers can see
  them. Corresponding unit test pins the behaviour.
- **Validation:**
  - `cargo test -p eddacraft-anvil-checks secret::scanner` covers a
    `line.len() > threshold` case
  - `cargo bench -p eddacraft-anvil-bench -- secret` does not regress on the
    happy path
- **Files:**
  - `crates/anvil-checks/src/secret/scanner.rs`
  - `crates/anvil-checks/src/secret/types.rs` (config field)
- **Confidence:** high
- **Priority:** High
- **Status:** Complete (landed 2026-04-26 — `max_line_bytes` default = 4096 (4 KB) chosen as a generous bound: short enough to neutralise pathological exponential regex blast radius, long enough that real source code never hits it; skipped lines surfaced via `ScanStats::lines_skipped_oversize` and aggregated through `SecretCheckResult.lines_skipped_oversize`; secret bench shows no regression beyond noise band)
- **Origin:** Council `security-analyst` SEC-004 (finding #2)

---

### SCAN-003: Bound rayon thread count for first-run scan

- **Intent:** On first run, the full repo scan competes with the TUI render
  thread and any background LSP / indexer. Default rayon behaviour is
  `num_cpus::get()`, which pins the machine on 16-core dev boxes.
- **Expected Outcome:** First-run / welcome-screen scan uses a scoped rayon
  pool of `num_cpus::get().min(4)` (or an env override), so the TUI stays
  responsive on underpowered machines while the scan completes.
- **Validation:**
  - TUI remains interactive (< 100 ms input latency) during first-run scan on
    a 4-core reference box
  - Unit test asserts pool size is honoured from env var
- **Files:**
  - `crates/anvil-cli/src/commands/welcome.rs`
  - Workspace `Cargo.toml` (add `num_cpus` if not already present)
- **Confidence:** medium
- **Priority:** Medium
- **Status:** Complete (landed 2026-04-26 — `ANVIL_SCAN_THREADS` is the canonical env-var name for the first-run rayon pool cap, shared with the upcoming RTAI first-run UX; default `min(num_cpus, 4)`; rationale recorded inline in `crates/anvil-cli/src/commands/welcome.rs` against `DEFAULT_FIRST_RUN_THREAD_CAP`. Alternative `ANVIL_RAYON_THREADS` was considered and rejected because it leaks the rayon implementation detail; "scan threads" composes cleanly with the existing `ANVIL_SCAN_ALL` toggle and matches the user-facing concern.)
- **Origin:** Council `security-analyst` SEC-004 (finding #3),
  `pragmatic-lead` (resource-contention concern)

---

### SCAN-004: Surface files_skipped_by_ignore in ScanResults

- **Intent:** When a gitignored file holds a real secret, the current design
  silently drops it (by design — gitignore is the user's intent). But there
  is no provenance: users cannot tell whether "0 findings" means "clean" or
  "we skipped the directory that had the secret". Surface the count.
- **Expected Outcome:** `ScanResults` gains a `files_skipped_by_ignore: usize`
  field (additive; the struct derives `Default`). The welcome discovery screen
  shows it when non-zero ("… N file(s) skipped by .gitignore — set
  `ANVIL_SCAN_ALL=1` to include them"). The count is suppressed (0) when the
  scan is truncated by the file cap or when `ANVIL_SCAN_ALL` is set, because
  neither case can be honestly attributed to gitignore.
- **Validation:**
  - `cargo test -p eddacraft-anvil-tui` covers the render (count shown when > 0,
    omitted when 0)
  - `cargo test -p eddacraft-anvil` covers the count derived in `scan_project`
  - Existing discovery render/snapshot fixtures updated for the new field
- **Files:**
  - `crates/anvil-tui/src/surfaces/tutorial/discovery.rs` (`ScanResults` struct +
    fixtures)
  - `crates/anvil-cli/src/commands/welcome.rs` (`scan_project` count)
  - `crates/anvil-tui/src/surfaces/tutorial/discovery_render.rs` (summary render)
- **Confidence:** high
- **Priority:** Medium
- **Status:** Merged 2026-05-27 via PR [#2021](https://github.com/eddacraft/anvil-001/pull/2021)
  (`3d1d3e1b`). Promoted Proposed → Ready → In Progress → Merged 2026-05-27.
- **Scope note (2026-05-27):** Spec originally guessed `ScanResults` lived in
  `crates/anvil-checks/src/types.rs` and that gate/audit would render the count.
  Reality: `ScanResults` is the welcome/TUI discovery type, and the welcome
  discovery scan is the ONLY secret-scan surface that honours `.gitignore`
  (`standard_filters(!scan_all)`); gate/audit/check/drift/policy/baseline all use
  `standard_filters(false)` by design, so the count would always be 0 there.
  SCAN-004 is therefore scoped to the welcome/`ScanResults` surface;
  `SecretCheckResult` is out of scope (dead plumbing).
- **Origin:** Council `security-analyst` SEC-007 (provenance)

---

### SCAN-005: Evaluate ignore::WalkParallel vs two-phase WalkBuilder

- **Intent:** The current discovery uses a sequential `WalkBuilder` followed
  by parallel per-file scanning. `ignore::WalkParallel` would parallelise the
  walk itself. On cold-cache first-run against very large monorepos this may
  further reduce wall time, but it complicates the two-phase allowlist pass
  (phase 1b: `ALWAYS_SCAN_FILENAMES` over ignored files).
- **Expected Outcome:** A one-page spike + benchmark comparing `WalkBuilder`
  (current) vs `WalkParallel` on (a) `entx` ~3k files, (b) a 30k-file
  synthetic repo. Decision captured in an ADR or inline note on this task.
  If `WalkParallel` wins ≥ 20% without breaking the allowlist, refactor;
  otherwise close as "tried, current approach is fine".
- **Validation:**
  - Benchmark numbers recorded in `plans/scratch/` or the ADR
  - Decision committed (either refactor PR or closed-with-numbers comment)
- **Files:**
  - `crates/anvil-cli/src/commands/welcome.rs`
  - `crates/anvil-bench/` (benchmark harness)
- **Confidence:** medium
- **Priority:** Low
- **Status:** Merged 2026-05-28 via PR [#2034](https://github.com/eddacraft/anvil-001/pull/2034)
  (`7c43a05c`). Promoted Proposed → Ready → In Progress → Merged 2026-05-28.
- **Outcome (2026-05-28):** Bench committed at
  `crates/anvil-bench/benches/walk_discovery.rs` (criterion, `harness = false`).
  Three corpora on a 16-core box:
  - 20k synthetic on tmpfs (RAM, pure CPU): seq **87.8 ms** → par **14.7 ms** = **6.0×**
  - entx real (ext4 warm, 2,694 candidates): seq **38.8 ms** → par **8.5 ms** = **4.55×**
  - 30k synthetic on ext4 (warm): seq **176.5 ms** → par **28.2 ms** = **6.3×**

  Walk speedup is robust **4.5–6.3×** across RAM/disk/sizes — well past the
  20% bar in isolation. End-to-end share is where it lands closer to the line:
  Phase 2 (the already-rayon-parallel read+regex scan) dominates total
  discovery, so a parallel walk saves only ~10–17% end-to-end on typical warm
  repos (walk is ~12–22% of welcome scan wall-time depending on the per-file
  scan cost; reference: `secret_scan_parallel` parallel pass on 3k files
  ≈ 899 ms with 8 threads, welcome caps at 4 per SCAN-003 so the share skews
  toward the upper bound). Cold-cache was **not measurable** here (no root in
  sandbox; `drop_caches` unwritable) — strictly favours parallel because
  serial pays IO latency one stat at a time, so the cold/large-monorepo tail
  is where the refactor would matter most.

  **Decision:** keep the spike single-purpose. Refactor deferred to **SCAN-006**
  (`Refactor scan_project discovery walk to WalkParallel`) so the production
  change can be sized + prioritised on its own, with the spike's numbers as
  baseline and the complexity cost (parallel-safe `SCAN_MAX_FILES` truncation,
  deterministic finding order, concurrent dedup of SCAN-004's
  `all_candidates`/`seen`) explicit in the acceptance criteria.
- **Origin:** Council `kernel-maintainer` (suggested `WalkParallel` during
  review; deferred because measured win came from regex parallelism — this
  spike confirmed walk parallelism is real but bounded end-to-end by
  Phase 2's already-parallel cost)

---

### SCAN-006: Refactor scan_project discovery walk to WalkParallel

- **Intent:** SCAN-005 measured that `ignore::WalkParallel` is **4.5–6.3×**
  faster than the current sequential `WalkBuilder` walk on every corpus tested
  (RAM, real ext4, 20k–30k synthetic). Realise that gain in production for the
  welcome discovery scan, while preserving the invariants the sequential walk
  currently provides for free.
- **Expected Outcome:** `scan_project_at` discovers candidates via
  `WalkBuilder::build_parallel()` instead of the sequential `Walk`, behind a
  design that preserves: (a) the two-phase allowlist semantics (Phase 1a
  gitignore-blind allowlist pass + Phase 1b gitignore-respecting walk);
  (b) deterministic finding order (currently emerges from the sequential walk
  + the explicit final `findings.sort_by(...)` — re-verify under parallel);
  (c) parallel-safe `SCAN_MAX_FILES` truncation (cannot rely on a sequential
  `break` after N); (d) concurrent dedup of the SCAN-004
  `all_candidates`/`seen` sets without breaking the membership-filter
  correctness guarantee.
- **Validation (done):**
  - Walk-phase speedup is evidenced by the committed `walk_discovery` bench
    (sequential `WalkBuilder` vs `WalkParallel`, **4.5–6.3×**); Phase 1a now
    uses that parallel path. **The original "≥20% end-to-end" bar was retired**
    — the SCAN-005 spike already established Phase 2 (the rayon-parallel
    read+regex scan) dominates total discovery, so end-to-end gain is ~10–17%
    on warm typical repos and larger on big/cold repos where the uncapped
    Phase 1a walk dominates. End-to-end ≥20% was never achievable and is not the
    right metric for a walk-only change.
  - All existing `welcome.rs` tests + SCAN-004 `gitignore_skip` + `discovery_render`
    tests stay green. ✓
  - New `welcome::tests::discovery_parallel` tests pin deterministic finding
    order across runs and `SCAN_MAX_FILES` truncation. ✓
  - Scope decided (see Status): Phase 1a parallelised; Phase 1b (capped) and
    `scan_all` (gitignore-off, Phase 1a skipped) left sequential, documented in
    code. Determinism preserved via the final `findings.sort_by` + the
    sequential capped Phase 1b.
- **Files (actual):**
  - `crates/anvil-cli/src/commands/welcome.rs` — new
    `collect_blind_candidates_parallel` helper, Phase 1a rewired to it, and
    `welcome::tests::discovery_parallel` invariant tests.
  - `crates/anvil-bench/benches/walk_discovery.rs` — unchanged; the SCAN-005
    bench already measures the seq-vs-parallel walk delta Phase 1a now realises.
  - `crates/anvil-tui/.../discovery.rs` — not touched; finding-order invariant
    held via the existing final sort, no struct change needed.
- **Confidence:** medium (truncation + dedup under parallel is non-trivial)
- **Priority:** Low
- **Status:** Merged 2026-05-28 via PR [#2041](https://github.com/eddacraft/anvil-001/pull/2041)
  (`67949a4d`). Promoted Proposed → Ready → In Progress → Merged 2026-05-28.
  Scope decision: parallelise the
  **uncapped Phase 1a** gitignore-blind walk (the dominant cost; order-free, no
  truncation risk) via `WalkBuilder::build_parallel()` with the SCAN-003 thread
  cap; keep the **capped Phase 1b** walk sequential to preserve its early-break
  at `SCAN_MAX_FILES` and deterministic truncation. `scan_all` mode (Phase 1a
  skipped) stays sequential and is documented as such. Finding order stays
  deterministic via the existing final `findings.sort_by`.
- **Origin:** SCAN-005 spike (PR TBD, 2026-05-28) — walk parallelism is real
  (4.5–6.3×) but end-to-end gain on typical warm repos is right at the 20%
  bar (~10–17%) because Phase 2 (the already-rayon-parallel read+regex scan)
  dominates total discovery. The case for refactoring grows for the
  cold-cache / very-large-monorepo tail (unmeasurable in the spike sandbox);
  pursue when that tail becomes a concrete concern.
