#!/usr/bin/env bash
# Multi-agent watch load-ramp harness (RLB-001).
#
# Promotes benchmarks/prototypes/anvil-load-probe.py into a committed,
# manually-runnable harness. It ramps N concurrent `anvil watch` agents
# against a synthetic repo, drives real file churn, and reports the whole
# process-tree CPU/RSS per agent level to find the saturation tipping point
# that the idle-path watch_resource_budget bench cannot see.
#
# This reproduces the beta-tester high-CPU report: bare `anvil watch` defaults
# to `--action check`, spawning a per-save scan; the ramp shows where a handful
# of concurrent agents saturate the box.
#
# See plans/modules/resource-load-benchmarking.aps.md (GH #2156).
#
# Usage:
#   bash scripts/bench/load-ramp.sh --smoke        # fast, low-footprint sanity run
#   bash scripts/bench/load-ramp.sh                # full ramp (1,2,4,8 agents)
#   ANVIL_BIN=/path/to/anvil bash scripts/bench/load-ramp.sh --agents 1,2,4
#
# The harness never touches a real checkout: the probe builds a throwaway
# synthetic repo in a temp dir (or an empty --repo you supply) and removes it.

set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
probe="${repo_root}/benchmarks/prototypes/anvil-load-probe.py"

smoke=0
do_build=0
bin="${ANVIL_BIN:-}"
files=""
agents=""
settle=""
measure=""
churn=""
action=""
repo=""

usage() {
  cat <<'EOF'
Usage: bash scripts/bench/load-ramp.sh [options]

Ramps concurrent `anvil watch` agents and prints a per-level process-tree
CPU/RSS table to find the saturation tipping point.

Options:
  --smoke              Fast, low-footprint run (40 files, 1,2 agents, short
                       windows). Proves the harness end-to-end; not a
                       precise saturation measurement.
  --agents LIST        Comma-separated agent counts to ramp (default: 1,2,4,8).
  --files N            Synthetic repo size (default: 1500; smoke: 40).
  --settle S           Seconds to let the cold scan settle before measuring.
  --measure S          Measurement window seconds per level.
  --churn-ms MS        Save interval per agent in milliseconds (default: 200).
  --action ACTION      Single-cell action override (check|gate|none). Default
                       ramps `check` at every level plus a `none` control.
  --repo DIR           Empty/new dir to build the synthetic repo in. With this
                       set the probe keeps the repo; the default temp repo is
                       created and removed by the probe (incl. on Ctrl-C).
  --bin PATH           anvil binary to drive (else $ANVIL_BIN, a built target
                       binary, or `anvil` on PATH).
  --build              cargo build the debug anvil binary if none is found.
  --help               Show this help.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --smoke) smoke=1 ;;
    --build) do_build=1 ;;
    --agents) agents="$2"; shift ;;
    --files) files="$2"; shift ;;
    --settle) settle="$2"; shift ;;
    --measure) measure="$2"; shift ;;
    --churn-ms) churn="$2"; shift ;;
    --action) action="$2"; shift ;;
    --repo) repo="$2"; shift ;;
    --bin) bin="$2"; shift ;;
    --help|-h) usage; exit 0 ;;
    *) echo "unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
  shift
done

# Smoke defaults keep the run fast and cheap while still exercising the ramp
# (>=2 levels) and printing a table. Explicit flags still win.
if [[ $smoke -eq 1 ]]; then
  files="${files:-40}"
  agents="${agents:-1,2}"
  settle="${settle:-1.5}"
  measure="${measure:-2.5}"
  churn="${churn:-200}"
fi

# Resolve the anvil binary. Priority: explicit --bin/$ANVIL_BIN, then a built
# binary under the active cargo target dir, then `anvil` on PATH.
target_dir="${CARGO_TARGET_DIR:-${repo_root}/target}"
if [[ -z "$bin" ]]; then
  for candidate in "${target_dir}/release/anvil" "${target_dir}/debug/anvil"; do
    if [[ -x "$candidate" ]]; then bin="$candidate"; break; fi
  done
fi
if [[ -z "$bin" ]] && command -v anvil >/dev/null 2>&1; then
  bin="$(command -v anvil)"
fi
if [[ -z "$bin" && $do_build -eq 1 ]]; then
  echo "# building debug anvil (no binary found) ..." >&2
  ( cd "$repo_root" && cargo build -p eddacraft-anvil --bin anvil )
  bin="${target_dir}/debug/anvil"
fi
if [[ -z "$bin" || ! -x "$bin" ]]; then
  # Exit 3 = precondition not met (no binary), distinct from a harness crash so
  # smoke callers can treat "nothing built yet" as a skip rather than a failure.
  echo "error: no anvil binary found." >&2
  echo "  set ANVIL_BIN=/path/to/anvil, pass --bin PATH, or re-run with --build." >&2
  exit 3
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "error: python3 is required to run the load probe." >&2
  exit 1
fi

if [[ ! -f "$probe" ]]; then
  echo "error: load probe not found at ${probe}" >&2
  exit 1
fi

probe_args=(--bin "$bin")
[[ -n "$files" ]]   && probe_args+=(--files "$files")
[[ -n "$agents" ]]  && probe_args+=(--agents "$agents")
[[ -n "$settle" ]]  && probe_args+=(--settle "$settle")
[[ -n "$measure" ]] && probe_args+=(--measure "$measure")
[[ -n "$churn" ]]   && probe_args+=(--churn-ms "$churn")
[[ -n "$action" ]]  && probe_args+=(--action "$action")
[[ -n "$repo" ]]    && probe_args+=(--repo "$repo")

smoke_label=""
[[ $smoke -eq 1 ]] && smoke_label=" (smoke)"
echo "# load-ramp harness — bin=${bin}${smoke_label}"
exec python3 "$probe" "${probe_args[@]}"
