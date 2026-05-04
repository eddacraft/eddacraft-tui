#!/usr/bin/env bash
# Preflight-only release script.
#
# Runs the deterministic checks that must pass before a release can start:
# formatting, linting, typechecking, and tests on both the Rust workspace
# and the TypeScript workspace. Every check runs to completion — a failure
# in one step does not short-circuit later steps. The exit code is the
# number of failed steps (0 on a clean pass).
#
# No prompts. No git operations. No GitHub calls. No handoff artefacts.
#
# Usage: ./scripts/release.sh
#
# Next step after a clean pass: invoke `/release` in Claude Code for the
# judgment half (version pick, branch strategy, tag, workflow monitor,
# changelog review, comms, cleanup).

set -u

readonly CYAN='\033[0;36m'
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[0;33m'
readonly BOLD='\033[1m'
readonly NC='\033[0m'

# Optional per-step timeout (seconds). 0 disables. Env override:
# ANVIL_RELEASE_STEP_TIMEOUT=0 ./scripts/release.sh
readonly STEP_TIMEOUT="${ANVIL_RELEASE_STEP_TIMEOUT:-600}"
# Keep these aligned with CARGO_HAKARI_VERSION and CARGO_DENY_VERSION in
# .github/workflows/rust.yml; require_release_tool_pins_synced enforces it.
readonly CARGO_HAKARI_VERSION="0.9.37"
readonly CARGO_DENY_VERSION="0.19.4"

declare -a RESULT_NAMES
declare -a RESULT_STATUS
declare -a RESULT_DURATIONS

require_cargo_tool_version() {
  local binary="$1"
  local subcommand="$2"
  local expected="$3"

  if ! command -v "${binary}" >/dev/null 2>&1; then
    echo -e "  ${RED}Missing ${binary}.${NC} Install with: cargo install ${binary} --locked --version ${expected}"
    return 1
  fi

  local output
  if ! output=$(cargo "${subcommand}" --version 2>&1); then
    echo -e "  ${RED}${binary} version probe failed.${NC}"
    echo -e "  Output: ${output}"
    echo -e "  Reinstall with: cargo install ${binary} --locked --version ${expected}"
    return 1
  fi

  local installed
  installed=$(awk '{print $2}' <<<"${output}")
  if [[ "${installed}" != "${expected}" ]]; then
    echo -e "  ${RED}${binary} version mismatch:${NC} installed=${installed} expected=${expected}"
    echo -e "  Install with: cargo install ${binary} --locked --version ${expected}"
    return 1
  fi

  echo "  ${binary} ${installed} installed"
}

require_release_tool_pins_synced() {
  local workflow=".github/workflows/rust.yml"
  local workflow_hakari
  local workflow_deny

  workflow_hakari=$(awk -F': ' '/CARGO_HAKARI_VERSION:/ {print $2; exit}' "${workflow}")
  workflow_deny=$(awk -F': ' '/CARGO_DENY_VERSION:/ {print $2; exit}' "${workflow}")

  if [[ "${workflow_hakari}" != "${CARGO_HAKARI_VERSION}" ]]; then
    echo -e "  ${RED}cargo-hakari pin drift:${NC} script=${CARGO_HAKARI_VERSION} rust.yml=${workflow_hakari}"
    return 1
  fi
  if [[ "${workflow_deny}" != "${CARGO_DENY_VERSION}" ]]; then
    echo -e "  ${RED}cargo-deny pin drift:${NC} script=${CARGO_DENY_VERSION} rust.yml=${workflow_deny}"
    return 1
  fi
}

run_check() {
  local name="$1"
  shift

  echo -e "\n${CYAN}▶${NC} ${BOLD}${name}${NC}"
  local start=${SECONDS}
  local rc=0

  if declare -F "$1" >/dev/null 2>&1; then
    "$@" || rc=$?
  elif [[ "${STEP_TIMEOUT}" != "0" ]] && command -v timeout >/dev/null 2>&1; then
    timeout "${STEP_TIMEOUT}" "$@" || rc=$?
  else
    "$@" || rc=$?
  fi

  local duration=$((SECONDS - start))
  local status
  if (( rc == 0 )); then
    status="PASS"
    echo -e "  ${GREEN}✓${NC} ${name} (${duration}s)"
  elif (( rc == 124 )); then
    status="TIMEOUT"
    echo -e "  ${RED}✗${NC} ${name} (timed out after ${STEP_TIMEOUT}s)"
  else
    status="FAIL"
    echo -e "  ${RED}✗${NC} ${name} (exit ${rc}, ${duration}s)"
  fi

  RESULT_NAMES+=("${name}")
  RESULT_STATUS+=("${status}")
  RESULT_DURATIONS+=("${duration}s")
}

print_summary() {
  echo
  echo -e "${BOLD}Preflight summary${NC}"
  printf "  %-20s %-8s %s\n" "step" "result" "duration"
  printf "  %-20s %-8s %s\n" "--------------------" "--------" "--------"

  local fail_count=0
  local i
  for i in "${!RESULT_NAMES[@]}"; do
    local name="${RESULT_NAMES[${i}]}"
    local status="${RESULT_STATUS[${i}]}"
    local duration="${RESULT_DURATIONS[${i}]}"
    local colour="${GREEN}"
    if [[ "${status}" != "PASS" ]]; then
      colour="${RED}"
      fail_count=$((fail_count + 1))
    fi
    printf "  %-20s ${colour}%-8s${NC} %s\n" "${name}" "${status}" "${duration}"
  done
  echo

  if (( fail_count == 0 )); then
    echo -e "${GREEN}All preflight checks passed.${NC}"
    echo -e "Next: invoke ${BOLD}/release${NC} in Claude Code."
    return 0
  fi

  echo -e "${RED}${fail_count} check(s) failed — address before tagging.${NC}"
  return "${fail_count}"
}

main() {
  echo -e "${BOLD}━━━ Anvil release preflight ━━━${NC}"
  echo -e "${YELLOW}Deterministic checks only. Judgment steps live in /release.${NC}"

  run_check "cargo fmt"      cargo fmt --all --check
  run_check "release pins"   require_release_tool_pins_synced
  run_check "hakari version" require_cargo_tool_version cargo-hakari hakari "${CARGO_HAKARI_VERSION}"
  run_check "cargo hakari"   cargo hakari verify
  run_check "deny version"   require_cargo_tool_version cargo-deny deny "${CARGO_DENY_VERSION}"
  run_check "cargo deny"     cargo deny check
  run_check "cargo clippy"   cargo clippy --workspace --all-targets -- -D warnings
  run_check "cargo test"     cargo test --workspace
  run_check "pnpm format"    pnpm format:check
  run_check "pnpm lint"      pnpm lint:check
  run_check "pnpm typecheck" pnpm typecheck
  run_check "pnpm test"      pnpm test

  print_summary
  exit $?
}

main "$@"
