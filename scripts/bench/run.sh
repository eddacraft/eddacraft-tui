#!/usr/bin/env bash
# Run the repository benchmark suite from one stable local entrypoint.
#
# This intentionally mirrors the manually-dispatchable benchmark workflows
# without depending on GitHub Actions. CI can call this later once the local
# contract is settled.

set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$repo_root"

include_nightly_stress=0
run_kindling=1
run_resource_budget=1

usage() {
  cat <<'EOF'
Usage: pnpm bench [-- <options>]

Runs all routine benchmark surfaces from one command:
  - Benchmark-history normaliser contract test
  - Rust benchmark compile checks (incl. graph-cache gate benches)
  - kernel, checks, stress, antipattern_scan, secret_scan_parallel,
    walk_discovery
  - intercept ipc_roundtrip and midedit_roundtrip
  - graph-cache hot_read and call_lift latency gates
  - midedit baseline comparison
  - resource budgets: watch, intercept daemon, MCP server, and the
    concurrent all-three aggregate (RLB-002/-003/-004/-005)
  - kindling-integration Vitest benchmark

Options:
  --include-nightly-stress   Also run the full anvil-bench stress runner
  --skip-kindling            Skip the TypeScript Kindling benchmark
  --skip-resource-budget     Skip the watch/intercept/MCP/concurrent resource
                             budgets (they need a quiet box + inotify headroom)
  --help                     Show this help
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --)
      ;;
    --include-nightly-stress)
      include_nightly_stress=1
      ;;
    --skip-kindling)
      run_kindling=0
      ;;
    --skip-resource-budget)
      run_resource_budget=0
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift
done

timestamp=$(date -u +%Y%m%dT%H%M%SZ)
artifact_dir="benchmark-results/manual-${timestamp}"
mkdir -p "$artifact_dir"

run_logged() {
  local name="$1"
  shift
  local log="${artifact_dir}/${name}.log"

  printf '\n==> %s\n' "$name"
  set -o pipefail
  "$@" 2>&1 | tee "$log"
}

run_logged_shell() {
  local name="$1"
  shift
  local log="${artifact_dir}/${name}.log"

  printf '\n==> %s\n' "$name"
  set -o pipefail
  bash -c "$*" 2>&1 | tee "$log"
}

echo "Benchmark artifacts: ${artifact_dir}"

run_logged benchmark-history-test python3 scripts/bench/to_history_test.py
run_logged cargo-check-anvil-bench cargo check -p anvil-bench --all-targets
run_logged cargo-check-kernel-benches cargo check -p eddacraft-anvil-kernel --benches
run_logged cargo-check-checks-benches cargo check -p eddacraft-anvil-checks --benches
run_logged cargo-check-intercept-benches cargo check -p eddacraft-anvil-intercept --benches --features bench-internals
run_logged cargo-check-graph-cache-benches cargo check -p eddacraft-anvil-graph-cache --benches
run_logged cargo-test-anvil-bench cargo test -p anvil-bench

run_logged kernel-bench cargo bench -p eddacraft-anvil-kernel --bench kernel -- --output-format bencher
run_logged checks-bench cargo bench -p eddacraft-anvil-checks --bench checks -- --output-format bencher
run_logged stress-bench cargo bench -p anvil-bench --bench stress -- --output-format bencher
run_logged antipattern-scan-bench cargo bench -p anvil-bench --bench antipattern_scan -- --output-format bencher
run_logged secret-scan-parallel-bench cargo bench -p anvil-bench --bench secret_scan_parallel -- --output-format bencher
run_logged walk-discovery-bench cargo bench -p anvil-bench --bench walk_discovery -- --output-format bencher
run_logged ipc-roundtrip-bench cargo bench -p eddacraft-anvil-intercept --features bench-internals --bench ipc_roundtrip
run_logged midedit-roundtrip-bench cargo bench -p eddacraft-anvil-intercept --features bench-internals --bench midedit_roundtrip
run_logged hot-read-bench cargo bench -p eddacraft-anvil-graph-cache --bench hot_read
run_logged call-lift-bench cargo bench -p eddacraft-anvil-graph-cache --bench call_lift

run_logged check-midedit-baseline bash scripts/check-midedit-baseline.sh \
  "${artifact_dir}/midedit-roundtrip-bench.log" \
  crates/anvil-intercept/benches/baselines/midedit_roundtrip.json

if [[ "$run_resource_budget" -eq 1 ]]; then
  run_logged build-release-anvil cargo build -p eddacraft-anvil --release --bin anvil
  # RLB-002/-003/-004: per-process CPU/RSS budgets. Each bench exits non-zero
  # when its budget verdict is not "pass", so run_logged_shell gates the suite.
  run_logged_shell watch-resource-budget \
    "ANVIL_BENCH_ANVIL_BIN=target/release/anvil cargo bench -p anvil-bench --bench watch_resource_budget"
  run_logged_shell intercept-resource-budget \
    "ANVIL_BENCH_ANVIL_BIN=target/release/anvil cargo bench -p eddacraft-anvil-intercept --bench intercept_resource_budget"
  run_logged_shell mcp-resource-budget \
    "ANVIL_BENCH_ANVIL_BIN=target/release/anvil cargo bench -p anvil-bench --bench mcp_resource_budget"
  # RLB-005: aggregate budget for all three processes at once. Heavier and, like
  # watch, needs inotify headroom — kept behind the same resource-budget gate.
  run_logged_shell concurrent-resource-budget \
    "ANVIL_BENCH_ANVIL_BIN=target/release/anvil cargo bench -p anvil-bench --bench concurrent_processes"
fi

if [[ "$run_kindling" -eq 1 ]]; then
  run_logged kindling-emission-overhead pnpm --filter @eddacraft/anvil-kindling-integration bench -- --run
fi

if [[ "$include_nightly_stress" -eq 1 ]]; then
  run_logged nightly-stress-runner cargo run -p anvil-bench --release
fi

echo
echo "Benchmark suite complete. Logs: ${artifact_dir}"
