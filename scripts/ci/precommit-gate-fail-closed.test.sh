#!/usr/bin/env bash
# CIB-277: the pre-commit gate must never be silently absent.
#
# Two failure modes were reproduced in the field, both in a fresh worktree
# before `pnpm install` finished:
#
#   1. Silent bypass  — `core.hooksPath` points at `.husky/_`, which husky
#      generates and gitignores. With no `.husky/_/pre-commit`, Git runs no
#      hook and prints nothing. The commit succeeds, exit 0, zero output.
#   2. Opaque failure — a `pnpm` that is on PATH but cannot execute dies with
#      a raw Node/corepack stack trace naming neither the gate nor a remedy.
#
# These tests build a throwaway repository containing exactly the `.husky`
# files Git tracks, so they fail if the shims are ever re-ignored. That
# coupling is the point: the fix is a `.gitignore` narrowing, and a test that
# copied the working tree instead would pass whether or not the shims ship.

set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)

tmp_root=$(mktemp -d)
cleanup() { rm -rf "${tmp_root}"; }
trap cleanup EXIT

# Husky's runner sources `${XDG_CONFIG_HOME:-$HOME/.config}/husky/init.sh`
# before every hook. A developer who keeps one — to put a Node or pnpm on PATH
# for non-interactive shells, say — has it prepend a REAL pnpm ahead of the
# fake shims these probes install, so the hook runs the real gate and probe 3
# can never observe the output it asserts on.
#
# Point XDG_CONFIG_HOME at an empty directory so the fixture exercises this
# repo's hook and nothing else. Without it the suite passes on CI (which has no
# such file) and fails on a configured workstation — the worst way round, since
# the failure then looks like it belongs to whatever change is under test.
export XDG_CONFIG_HOME="${tmp_root}/xdg"
mkdir -p "${XDG_CONFIG_HOME}"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

# A PATH holding only what Git and the hook need to reach their first
# decision, and deliberately no package manager.
#
# Inheriting the caller's PATH would not test what it looks like it tests: the
# host has a working `npx`, so the hook rightly falls back to it and the
# no-toolchain branch never runs. Naming the binaries explicitly is what makes
# "no runner available" true rather than assumed.
minimal_path() {
  local dir="${tmp_root}/minimal-bin"
  if [ ! -d "${dir}" ]; then
    mkdir -p "${dir}"
    local tool resolved
    for tool in git env sh dirname basename; do
      resolved=$(command -v "${tool}") ||
        fail "${tool} is required to run these probes but is not on PATH"
      ln -s "${resolved}" "${dir}/${tool}"
    done
    for tool in pnpm npx node; do
      if [ -x "${dir}/${tool}" ] || [ -L "${dir}/${tool}" ]; then
        fail "minimal PATH must not provide ${tool}"
      fi
    done
  fi
  echo "${dir}"
}

# Materialise a repo standing in for a fresh clone or worktree, before any
# install has run.
#
# The file list comes from `git ls-files` in the repository under test, so the
# fixture contains exactly what Git would deliver on checkout — that is the
# coupling that makes re-ignoring the shims fail this suite. Inside the
# fixture the copies are working-tree files; only `seed.txt` is committed,
# because the probes need a HEAD to commit against and nothing more.
new_fixture_repo() {
  local dir="$1"
  mkdir -p "${dir}"
  git -C "${dir}" init --quiet
  git -C "${dir}" config core.hooksPath .husky/_
  git -C "${dir}" config user.email test@example.com
  git -C "${dir}" config user.name test

  local tracked
  tracked=$(git -C "${repo_root}" ls-files -- .husky)
  [ -n "${tracked}" ] || fail "no .husky paths are tracked; expected at least .husky/pre-commit"

  local path
  while IFS= read -r path; do
    mkdir -p "${dir}/$(dirname "${path}")"
    cp "${repo_root}/${path}" "${dir}/${path}"
    # Git records only the executable bit; mirror it so the fixture behaves
    # the way a real checkout would.
    if [ "$(git -C "${repo_root}" ls-files --stage -- "${path}" | cut -c1-6)" = "100755" ]; then
      chmod +x "${dir}/${path}"
    fi
  done <<<"${tracked}"

  echo 'seed' >"${dir}/seed.txt"
  git -C "${dir}" add seed.txt
  git -C "${dir}" -c core.hooksPath=/var/empty commit --quiet -m 'seed'
}

# A staged file that the gate would reject if it ran: deliberately malformed
# JSON, matching the probe recorded on CIB-277.
stage_malformed_json() {
  local dir="$1"
  printf '{"a":   1,\n\n   "b":2}\n' >"${dir}/malformed.json"
  git -C "${dir}" add malformed.json
}

# ── Probe 1: the gate must exist before any install ──────────────────

probe_gate_is_present_without_install() {
  local dir="${tmp_root}/no-install"
  new_fixture_repo "${dir}"

  [ -f "${dir}/.husky/_/pre-commit" ] ||
    fail "no .husky/_/pre-commit in a fresh checkout — Git runs no hook and the commit is silently unchecked (CIB-277 probe 1)"

  stage_malformed_json "${dir}"

  # No node_modules and no package runner at all: the gate cannot do its work,
  # so it must refuse rather than wave the commit through.
  local out status=0
  out=$(cd "${dir}" && PATH="$(minimal_path)" git commit -m 'should be refused' 2>&1) || status=$?

  [ "${status}" -ne 0 ] ||
    fail "commit succeeded with no usable toolchain — this is the silent bypass CIB-277 filed (output: ${out})"

  grep -qi 'pre-commit' <<<"${out}" ||
    fail "refusal does not name the gate; a contributor cannot tell what blocked them: ${out}"

  grep -qiE 'pnpm|npx|node' <<<"${out}" ||
    fail "refusal names no cause or remedy: ${out}"
}

# ── Probe 2: a broken pnpm must produce an actionable message ────────

probe_broken_pnpm_names_the_gate_and_a_remedy() {
  local dir="${tmp_root}/broken-pnpm"
  new_fixture_repo "${dir}"

  # `command -v pnpm` succeeds for this one; running it does not. That is the
  # exact shape of the corepack failure recorded on CIB-277.
  local shim="${tmp_root}/broken-bin"
  mkdir -p "${shim}"
  cat >"${shim}/pnpm" <<'BROKEN'
#!/bin/sh
echo "TypeError [ERR_VM_DYNAMIC_IMPORT_CALLBACK_MISSING]: A dynamic import callback was not specified." >&2
echo "    at importModuleDynamicallyCallback (node:internal/modules/esm/utils:272:9)" >&2
exit 1
BROKEN
  chmod +x "${shim}/pnpm"

  stage_malformed_json "${dir}"

  # Broken pnpm, no npx to fall back to — the branch that used to emit a bare
  # Node trace.
  local out status=0
  out=$(cd "${dir}" && PATH="${shim}:$(minimal_path)" git commit -m 'should be refused' 2>&1) || status=$?

  [ "${status}" -ne 0 ] ||
    fail "commit succeeded while pnpm was unusable — the gate never ran (output: ${out})"

  grep -qi 'pre-commit' <<<"${out}" ||
    fail "message does not name the gate: ${out}"

  # The defect here was never the failure — the old hook failed too. It was
  # the illegibility of it. So these assertions must distinguish a written
  # explanation from a raw crash, or they pass against the very code that
  # prompted the item. Loose greps do not: the pre-fix output already
  # satisfied "names the gate" (husky prints "pre-commit script failed") and
  # "mentions node" (the trace says node:internal/...).
  #
  # Note the hook deliberately quotes the tool's own stderr under "The tool
  # reported:", because the underlying error is often more specific than
  # anything the hook can infer. So the presence of the raw trace is NOT
  # evidence of the old behaviour — the structured explanation around it is
  # what distinguishes them, and that is what these assert.
  grep -q 'could not run' <<<"${out}" ||
    fail "no refusal framing, so this reads as an unrelated crash: ${out}"

  grep -q 'Cause:' <<<"${out}" ||
    fail "message states no cause: ${out}"

  grep -q 'Fix:' <<<"${out}" ||
    fail "message offers no remedy: ${out}"
}

# ── Probe 3: a working toolchain still gates ─────────────────────────

probe_working_toolchain_still_runs_the_gate() {
  local dir="${tmp_root}/working"
  new_fixture_repo "${dir}"

  # Stand in for a healthy pnpm. The readiness probe
  # (`pnpm exec lint-staged --version`) must SUCCEED here, or the hook exits
  # through its fail-closed branch and never reaches the gate — which is the
  # whole thing this probe exists to observe. Only the bare
  # `pnpm exec lint-staged` fails, standing in for a real rejection.
  local shim="${tmp_root}/working-bin"
  mkdir -p "${shim}"
  cat >"${shim}/pnpm" <<'WORKING'
#!/bin/sh
[ "$1" = "--version" ] && { echo "11.9.0"; exit 0; }
if [ "$1" = "exec" ]; then
  shift
  case "$*" in
    'lint-staged --version') echo "17.0.7"; exit 0 ;;
    'lint-staged') echo "lint-staged: malformed.json is not formatted" >&2; exit 1 ;;
  esac
fi
exit 0
WORKING
  chmod +x "${shim}/pnpm"

  stage_malformed_json "${dir}"

  local out status=0
  out=$(cd "${dir}" && PATH="${shim}:$(minimal_path)" git commit -m 'should be refused' 2>&1) || status=$?

  [ "${status}" -ne 0 ] ||
    fail "a healthy toolchain reporting a lint-staged failure still let the commit through: ${out}"

  # Assert on the gate's OWN output, not on the word "lint-staged" — the
  # fail-closed message contains that word too ("lint-staged is not
  # runnable"), so grepping for it passes even when the gate never ran. An
  # earlier draft of this probe did exactly that, and the entire gate body
  # could be deleted with the suite still green.
  grep -q 'is not formatted' <<<"${out}" ||
    fail "the gate's own output is missing, so it never ran: ${out}"

  if grep -q 'could not run' <<<"${out}"; then
    fail "took the fail-closed path despite a healthy toolchain: ${out}"
  fi
}

probe_gate_is_present_without_install
probe_broken_pnpm_names_the_gate_and_a_remedy
probe_working_toolchain_still_runs_the_gate

echo 'pre-commit gate fail-closed probes passed'
