#!/usr/bin/env bash
# run-candidate.sh — build the in-repo Anvil candidate and set up a
# side-by-side test environment.
#
# Purpose: dogfood a pre-release candidate (typically what's on HEAD of
# main) without uninstalling the production Anvil install. There is no
# ANVIL_HOME override today (tracked as GH #1726), so this script
# minimises the collision surface by:
#
#   1. Building the candidate to target/release/anvil
#   2. Stopping the production daemon so the candidate's daemon can
#      bind the socket
#   3. Symlinking the candidate as ~/.local/bin/anvil-beta (kept distinct
#      from the prod `anvil` on PATH)
#   4. Pre-creating an isolated scratch project under
#      /tmp/anvil-candidate-<sha>/ so per-project state stays out of
#      real repos
#
# Use it for: smoke-testing the candidate before tag, validating fixes
# during the multi-pass loop in v0.7.0-beta-release-runbook.md §2,
# or running the demo runbook against the candidate.
#
# Do NOT use it for: Boring Week — that explicitly forbids developer
# overrides ("install how a real first-time user would").
#
# Usage:
#   scripts/dev/run-candidate.sh             # build + setup
#   scripts/dev/run-candidate.sh --ref <sha> # build a specific git ref
#   scripts/dev/run-candidate.sh --scratch <path>  # custom scratch dir
#   scripts/dev/run-candidate.sh --keep-prod-daemon  # skip daemon stop
#   scripts/dev/run-candidate.sh --restore   # undo: remove symlink,
#                                            #   restart prod daemon
#   scripts/dev/run-candidate.sh --status    # show current candidate
#                                            #   install state
#
# Exit codes:
#   0  success
#   1  build failure
#   2  daemon stop / start failure
#   3  symlink failure
#   4  bad CLI args
#
# Related: GH #1726 (ANVIL_HOME proposal — makes this script unnecessary).

set -euo pipefail

# ── Configuration ────────────────────────────────────────────────────────

SCRIPT_NAME="$(basename "$0")"
REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
SYMLINK_PATH="${HOME}/.local/bin/anvil-beta"
PROD_BINARY="$(command -v anvil || echo /usr/local/bin/anvil)"

MODE="setup"
GIT_REF=""
SCRATCH_DIR=""
KEEP_PROD_DAEMON=false

# ── Helpers ──────────────────────────────────────────────────────────────

c_step() { printf '\033[1;34m▶\033[0m %s\n' "$*"; }
c_ok()   { printf '\033[1;32m✓\033[0m %s\n' "$*"; }
c_warn() { printf '\033[1;33m!\033[0m %s\n' "$*"; }
c_err()  { printf '\033[1;31m✗\033[0m %s\n' "$*" >&2; }
c_info() { printf '  %s\n' "$*"; }

usage() {
  sed -n '3,33p' "$0" | sed 's/^# \?//'
  exit "${1:-0}"
}

# ── Parse args ───────────────────────────────────────────────────────────

while [ $# -gt 0 ]; do
  case "$1" in
    --ref)     GIT_REF="$2"; shift 2 ;;
    --scratch) SCRATCH_DIR="$2"; shift 2 ;;
    --keep-prod-daemon) KEEP_PROD_DAEMON=true; shift ;;
    --restore) MODE="restore"; shift ;;
    --status)  MODE="status"; shift ;;
    --help|-h) usage 0 ;;
    *)         c_err "unknown arg: $1"; usage 4 ;;
  esac
done

# ── Daemon helpers ───────────────────────────────────────────────────────

# Returns 0 if a prod-anvil daemon is running, 1 otherwise.
prod_daemon_running() {
  pgrep -f 'anvil intercept start' >/dev/null 2>&1
}

stop_prod_daemon() {
  if ! prod_daemon_running; then
    c_info "no prod daemon process detected"
    return 0
  fi
  c_step "stopping prod daemon (SIGTERM)"
  pkill -TERM -f 'anvil intercept start' || true
  # Wait up to 5 s for graceful exit
  for _ in 1 2 3 4 5; do
    if ! prod_daemon_running; then
      c_ok "prod daemon stopped"
      return 0
    fi
    sleep 1
  done
  c_warn "daemon still running after 5 s; sending SIGKILL"
  pkill -KILL -f 'anvil intercept start' || true
  sleep 1
  if prod_daemon_running; then
    c_err "could not stop prod daemon"
    return 2
  fi
  c_ok "prod daemon force-killed"
}

# ── Modes ────────────────────────────────────────────────────────────────

mode_status() {
  c_step "candidate install state"
  if [ -L "$SYMLINK_PATH" ]; then
    local target
    target="$(readlink "$SYMLINK_PATH")"
    c_info "symlink: $SYMLINK_PATH → $target"
    if [ -x "$target" ]; then
      c_info "candidate version: $("$target" --version 2>/dev/null || echo '(could not read version)')"
    else
      c_warn "target is not executable (build cleaned?)"
    fi
  else
    c_info "no candidate symlink at $SYMLINK_PATH"
  fi
  if [ -x "$PROD_BINARY" ]; then
    c_info "prod binary: $PROD_BINARY ($("$PROD_BINARY" --version 2>/dev/null || echo '(no version)'))"
  fi
  if prod_daemon_running; then
    c_info "prod daemon: running ($(pgrep -f 'anvil intercept start' | tr '\n' ' '))"
  else
    c_info "prod daemon: not running"
  fi
  exit 0
}

mode_restore() {
  c_step "removing candidate symlink"
  if [ -L "$SYMLINK_PATH" ]; then
    rm "$SYMLINK_PATH"
    c_ok "removed $SYMLINK_PATH"
  else
    c_info "no candidate symlink to remove"
  fi
  c_step "restarting prod daemon"
  if prod_daemon_running; then
    c_info "prod daemon already running"
  elif [ -x "$PROD_BINARY" ]; then
    nohup "$PROD_BINARY" intercept start >/dev/null 2>&1 &
    sleep 1
    if prod_daemon_running; then
      c_ok "prod daemon started"
    else
      c_warn "started but not detected after 1 s — check 'anvil intercept status'"
    fi
  else
    c_warn "no prod binary at $PROD_BINARY; start the daemon manually"
  fi
  exit 0
}

mode_setup() {
  cd "$REPO_ROOT"

  # If --ref given, check out that ref to a detached worktree for a
  # clean build. Skipped by default — operator usually builds whatever
  # is checked out.
  local resolved_ref
  if [ -n "$GIT_REF" ]; then
    resolved_ref="$(git rev-parse --short "$GIT_REF")"
    c_step "checking out $GIT_REF ($resolved_ref) into build worktree"
    local build_wt="/tmp/anvil-candidate-build-$resolved_ref"
    if [ -d "$build_wt" ]; then
      git -C "$build_wt" checkout "$GIT_REF" >/dev/null 2>&1
    else
      git worktree add "$build_wt" "$GIT_REF" >/dev/null 2>&1
    fi
    cd "$build_wt"
    c_ok "build worktree at $build_wt"
  else
    resolved_ref="$(git rev-parse --short HEAD)"
    c_info "building from current HEAD ($resolved_ref)"
  fi

  c_step "building candidate (cargo build --release -p eddacraft-anvil)"
  if ! cargo build --release -p eddacraft-anvil 2>&1 | tail -3; then
    c_err "build failed"
    exit 1
  fi
  local binary="$(pwd)/target/release/anvil"
  if [ ! -x "$binary" ]; then
    c_err "expected binary at $binary but not found"
    exit 1
  fi
  c_ok "built $binary"
  c_info "version: $("$binary" --version 2>/dev/null || echo '(no version)')"

  if [ "$KEEP_PROD_DAEMON" = false ]; then
    stop_prod_daemon || exit 2
  else
    c_info "skipping prod-daemon stop (--keep-prod-daemon)"
  fi

  c_step "linking $binary → $SYMLINK_PATH"
  mkdir -p "$(dirname "$SYMLINK_PATH")"
  ln -sf "$binary" "$SYMLINK_PATH"
  if [ ! -L "$SYMLINK_PATH" ]; then
    c_err "symlink not created"
    exit 3
  fi
  c_ok "symlink ready"

  if [ -z "$SCRATCH_DIR" ]; then
    SCRATCH_DIR="/tmp/anvil-candidate-$resolved_ref"
  fi
  c_step "preparing scratch project $SCRATCH_DIR"
  mkdir -p "$SCRATCH_DIR"
  if [ ! -d "$SCRATCH_DIR/.git" ]; then
    (cd "$SCRATCH_DIR" && git init -q -b main && echo '# candidate test scratch' > README.md && git add . && git commit -q -m 'init')
    c_ok "initialised git repo in $SCRATCH_DIR"
  else
    c_info "scratch project already initialised"
  fi

  # Final operator instructions
  printf '\n'
  c_ok "candidate ready"
  printf '\n'
  printf '  Use the candidate by invoking \033[1manvil-beta\033[0m (not anvil):\n\n'
  printf '    cd %s\n' "$SCRATCH_DIR"
  printf '    anvil-beta start\n'
  printf '    anvil-beta intercept start --foreground &\n'
  printf '    anvil-beta watch\n'
  printf '    # ...\n\n'
  printf '  Caveats:\n'
  printf '    - %s/ is still shared with prod (no ANVIL_HOME yet — see GH #1726).\n' "$HOME/.anvil"
  printf '    - Project state under %s/.anvil/ is isolated by virtue of the scratch path.\n' "$SCRATCH_DIR"
  printf '    - When you are done, restore prod with: %s --restore\n\n' "scripts/dev/run-candidate.sh"
  printf '  Status: %s --status\n' "scripts/dev/run-candidate.sh"
}

case "$MODE" in
  setup)   mode_setup ;;
  restore) mode_restore ;;
  status)  mode_status ;;
  *)       c_err "unknown mode: $MODE"; exit 4 ;;
esac
