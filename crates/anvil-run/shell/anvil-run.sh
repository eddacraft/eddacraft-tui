# shellcheck shell=bash
# INTL-006: shell integration for `anvil-run`.
#
# Source this file from `.zshrc` or `.bashrc` to route common agent
# tools through the launcher. Each wrapper delegates to the binary
# resolved on `$PATH` — no shell-side path assumptions.
#
#   . "$(brew --prefix anvil)/share/anvil-run/anvil-run.sh"   # Homebrew
#   . "$HOME/.local/share/anvil/anvil-run.sh"                 # curl
#
# The script supports zsh and bash. Fish has a separate file (out of
# scope for INTL-006 v1; see INTL-008 follow-ups).
#
# Wrappers respect `ANVIL_RUN_DISABLE=1` so users who want to bypass
# the launcher temporarily can do so without unloading the script.

if [ -n "${ZSH_VERSION-}" ]; then
  emulate -L bash
fi

# Locate the anvil-run binary once at source time. Re-evaluated on
# every call so a user upgrading anvil through Homebrew picks up the
# new path without restarting their shell.
__anvil_run_bin() {
  if [ -n "${ANVIL_RUN_BIN-}" ] && [ -x "${ANVIL_RUN_BIN}" ]; then
    echo "${ANVIL_RUN_BIN}"
    return 0
  fi
  command -v anvil-run 2>/dev/null
}

__anvil_run_dispatch() {
  # $1: tool name (driver_id)
  # $2..: the wrapped command + its args
  local tool="$1"
  shift
  if [ -n "${ANVIL_RUN_DISABLE-}" ]; then
    command "$@"
    return $?
  fi
  local bin
  bin="$(__anvil_run_bin)"
  if [ -z "${bin}" ]; then
    # Launcher not installed — fall through so the user is not
    # blocked. They will lose Anvil's session enforcement until
    # they install it, which is the desired UX over a hard error.
    command "$@"
    return $?
  fi
  "${bin}" --tool "${tool}" -- "$@"
}

claude() {
  __anvil_run_dispatch claude-code claude "$@"
}

codex() {
  __anvil_run_dispatch codex codex "$@"
}

aider() {
  __anvil_run_dispatch aider aider "$@"
}

# Convenience for ad-hoc tools: `anvil-wrap <tool> <cmd> [args...]`
anvil-wrap() {
  if [ "$#" -lt 2 ]; then
    echo "usage: anvil-wrap <tool> <cmd> [args...]" >&2
    return 64
  fi
  local tool="$1"
  shift
  __anvil_run_dispatch "${tool}" "$@"
}
