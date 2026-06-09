# Post-merge: feat-gv2-025-hot-read-bench

PR: #NNN
Branch: `feat/gv2-025-hot-read-bench`
APS: GV2-025
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Step 1 — On the first `main` push touching `crates/**`, confirm the
      `Resource Budget` workflow's `per-process resource budgets` job ran the new
      **Measure hot-read latency budget (GV2-025)** step and it passed (p95 well
      under the 80 ms budget). (agent: yes — `gh run list --workflow=resource-budget.yml`)
- [ ] Step 2 — Confirm the **Self-test — hot-read gate trips on a synthetic
      regression** step passed (it asserts the gate exits non-zero under
      `ANVIL_BENCH_HOTREAD_STALL_MS=120`). (agent: yes — check step logs)
- [ ] Step 3 — Flip GV2-025 `In Progress → Merged` in
      `plans/modules/graph-v2-foundation.aps.md` with `Merged YYYY-MM-DD via PR #N`
      (count stays 11/19 until released — Merged is not Done). (agent: yes)

## Notes

- This change is bench + CI-workflow only; **no production code path is
  modified** (the daemon hot-read API and `certify` are unchanged). Risk is
  confined to CI wiring and gate correctness, both verified locally.
- Local verification before PR: `cargo build/clippy -D warnings/fmt --check`
  clean; 120 crate tests pass; bench GREEN (exit 0, p95 0.005–0.039 ms); bench
  RED self-test (120 ms stall) exits 1 with all 5 ops FAIL; `yamllint` clean.
- The gate is a **per-op gross-regression ceiling**, not a composed-path SLO —
  the end-to-end save-time budget is already gated by the
  `validation.service:validate_paths` p95 gate in `ipc_roundtrip` (RLB-008 /
  DSV-006). The two are complementary; see the `HOT_READ_P95_BUDGET` doc comment.
- A compile-time assert pins the corpus depth to `MAX_REVERSE_IMPACT_DEPTH`, so a
  future cap increase (e.g. GV2-026) breaks the build until `build_corpus` grows
  a layer rather than silently under-exercising the new cap.
