# SCAN-core — Parallel rollout, ReDoS guard, bounded thread pool

## Purpose

Cover SCAN-001 (extend the parallel-scan pattern to remaining call-sites),
SCAN-002 (per-line length guard against ReDoS in `scan_content`), and
SCAN-003 (bound rayon thread count for first-run scans) as one slice that
hardens the shared scanner primitives before broader rollout.

## Actions

### 1. Map the remaining serial scan call-sites

- **Purpose:** Confirm every `walkdir` + serial-regex surface called out in SCAN-001 is still present and in scope.
- **Produces:** Verified inventory of fan-out sites in `gate`, `audit`, `check`, `drift`, `policy`, `architecture/validator`, `kernel/watcher`.
- **Checkpoint:** Inventory matches SCAN-001 file list with current line refs.
- **Validate:** `rg -n "walkdir::WalkDir" crates/`

### 2. Apply the parallel-scan pattern to each call-site

- **Purpose:** Bring `ignore::WalkBuilder` + `rayon::par_iter` + `catch_unwind` to the surfaces still on the slow path (SCAN-001).
- **Produces:** Updated call-sites sharing the welcome-screen discovery shape, with deterministic finding order and contained per-file panics.
- **Checkpoint:** Audit, check, drift, gate, policy, validator, and watcher use the parallel pattern.
- **Validate:** `cargo test -p eddacraft-anvil -p eddacraft-anvil-checks -p eddacraft-anvil-kernel`

### 3. Benchmark a representative surface

- **Purpose:** Confirm the rollout delivers the SCAN-001 acceptance threshold (>3× wall-time reduction on `entx`-class repos).
- **Produces:** Benchmark numbers captured under `crates/anvil-bench/` or `plans/scratch/`.
- **Checkpoint:** Benchmark shows >3× wall-time win on at least one rolled-out surface.
- **Validate:** `cargo bench -p eddacraft-anvil-bench -- scan`

### 4. Add the per-line length guard to `scan_content`

- **Purpose:** Stop pathological lines from triggering worst-case backtracking, especially against custom regexes (SCAN-002).
- **Produces:** Configurable threshold (default 8 KB) wired into `scan_content`, with skipped lines counted on the result.
- **Checkpoint:** Long-line input is skipped and surfaced via a counter.
- **Validate:** `cargo test -p eddacraft-anvil-checks secret::scanner`

### 5. Confirm the guard is benchmark-neutral

- **Purpose:** Guarantee SCAN-002 does not regress the happy-path scan throughput.
- **Produces:** Recorded bench delta comparing pre- and post-guard runs.
- **Checkpoint:** Secret-scan bench shows no regression beyond noise band.
- **Validate:** `cargo bench -p eddacraft-anvil-bench -- secret`

### 6. Coordinate the first-run thread-pool env var with RTAI

- **Purpose:** SCAN-003 introduces a first-run rayon-pool override; the canonical env var name and default must align with the daemon-side debounced scan surface in `plans/modules/realtime-ai-validation.aps.md`, otherwise users hit two competing knobs.
- **Produces:** Decision recorded inline against SCAN-003 (env var name, default value, scope) after a quick handshake with the RTAI owner.
- **Checkpoint:** Env var name and default agreed and noted alongside SCAN-003.

### 7. Bound the first-run rayon pool

- **Purpose:** Keep the TUI responsive on underpowered machines while the welcome-screen scan completes (SCAN-003).
- **Produces:** Scoped rayon pool capped at `num_cpus::get().min(4)`, honouring the env override decided in step 6.
- **Checkpoint:** First-run scan honours pool cap and env override.
- **Validate:** `cargo test -p eddacraft-anvil welcome::pool`

### 8. Stage and integrate the slice

- **Purpose:** Land SCAN-001/002/003 together with a green workspace.
- **Produces:** Staged changes across the rolled-out call-sites, scanner guard, and welcome-screen pool.
- **Checkpoint:** Workspace builds and tests cleanly with the slice in place.
- **Validate:** `git add crates/ Cargo.toml Cargo.lock && cargo test --workspace --no-fail-fast`
