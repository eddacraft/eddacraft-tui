#!/usr/bin/env bash
# CIB-391: run the primary CLI suite under a deliberately hostile ambient
# profile so tests that inherit umask, privacy opt-out, or a live daemon fail
# deterministically instead of as flakes on unrelated PRs.
#
# Nightly only — do not wire this as a required per-PR gate until it has been
# green once. The first red is the point: a known umask-dependent failure
# (`stop_clears_a_stale_pid_file`) must show up here before that test is fixed.
set -euo pipefail

umask 002
export DO_NOT_TRACK=1
export ANVIL_DEV=1
export ANVIL_SKIP_WELCOME=1

echo "[hostile-ambient] umask=$(umask) DO_NOT_TRACK=${DO_NOT_TRACK}"

cargo build -p eddacraft-anvil

bin="${CARGO_TARGET_DIR:-target}/debug/anvil"
if [[ -x "$bin" ]]; then
  if ! "$bin" intercept start; then
    echo "[hostile-ambient] live daemon did not start; suite still runs under umask 002 and DO_NOT_TRACK" >&2
  fi
else
  echo "[hostile-ambient] anvil binary missing after build; suite still runs" >&2
fi

exec cargo test -p eddacraft-anvil --no-fail-fast -- --test-threads=1
