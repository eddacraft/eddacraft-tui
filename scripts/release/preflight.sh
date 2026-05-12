#!/usr/bin/env bash
set -uo pipefail

readonly DEFAULT_REPO="eddacraft/anvil-001"
readonly CARGO_HAKARI_VERSION="0.9.37"
readonly CARGO_DENY_VERSION="0.19.4"
readonly STEP_TIMEOUT="${ANVIL_RELEASE_STEP_TIMEOUT:-600}"
# pnpm test runs `nx run-many -t test` with `test → ^build`, so every node
# project rebuilds its upstream graph before vitest runs. Cold-cache wall
# time on the current workspace is ~20 min; the default STEP_TIMEOUT (10
# min) trips it as a false positive. Per-gate override here, configurable.
readonly PNPM_TEST_TIMEOUT="${ANVIL_RELEASE_PNPM_TEST_TIMEOUT:-1800}"

json=false
base=""
head=""
repo="$DEFAULT_REPO"
parse_error=""

declare -a GATE_IDS=()
declare -a GATE_NAMES=()
declare -a GATE_COMMANDS=()
declare -a GATE_STATUS=()
declare -a GATE_EXIT_CODES=()
declare -a GATE_DURATIONS=()

reserved_exit=0

hakari_expected="$CARGO_HAKARI_VERSION"
hakari_installed=""
hakari_status="unknown"
deny_expected="$CARGO_DENY_VERSION"
deny_installed=""
deny_status="unknown"

usage() {
  cat <<'USAGE'
Usage: bash scripts/release/preflight.sh [--json] [--base <ref>] [--head <ref>] [--repo <owner/name>]

Runs deterministic local release-readiness gates without network or gh calls.

Options:
  --json              Emit exactly one JSON object to stdout.
  --base <ref>        Comparison base ref recorded in output.
  --head <ref>        Comparison head ref recorded in output.
  --repo <owner/name> Repository owner/name. Defaults to eddacraft/anvil-001.
  -h, --help          Show this help.

Environment:
  ANVIL_RELEASE_STEP_TIMEOUT       Default per-gate timeout in seconds (default 600).
  ANVIL_RELEASE_PNPM_TEST_TIMEOUT  pnpm-test gate timeout in seconds (default 1800).

Test fixture mode:
  ANVIL_RELEASE_PREFLIGHT_FIXTURE=pass|fail|version-mismatch|missing-tool
  ANVIL_RELEASE_PREFLIGHT_FIXTURE_FAILURES=cargo-test,pnpm-lint
USAGE
}

timestamp() {
  date -u +"%Y-%m-%dT%H:%M:%SZ"
}

json_string() {
  JSON_VALUE="$1" node -e 'process.stdout.write(JSON.stringify(process.env.JSON_VALUE ?? ""))'
}

json_nullable_string() {
  if [[ -z "$1" ]]; then
    printf 'null'
  else
    json_string "$1"
  fi
}

tool_version() {
  local binary="$1"
  shift
  if ! command -v "$binary" >/dev/null 2>&1; then
    printf 'null'
    return 0
  fi
  local output
  if output=$("$binary" "$@" 2>/dev/null); then
    json_string "$output"
  else
    printf 'null'
  fi
}

fixture_has_failure() {
  local gate_id="$1"
  local failures=",${ANVIL_RELEASE_PREFLIGHT_FIXTURE_FAILURES:-},"
  [[ "$failures" == *",${gate_id},"* ]]
}

require_release_tool_pins_synced() {
  local workflow=".github/workflows/rust.yml"
  local workflow_hakari=""
  local workflow_deny=""

  if [[ -f "$workflow" ]]; then
    workflow_hakari=$(awk -F': ' '/CARGO_HAKARI_VERSION:/ {print $2; exit}' "$workflow" | tr -d "'\"")
    workflow_deny=$(awk -F': ' '/CARGO_DENY_VERSION:/ {print $2; exit}' "$workflow" | tr -d "'\"")
  fi

  if [[ "$workflow_hakari" != "$CARGO_HAKARI_VERSION" ]]; then
    return 1
  fi
  if [[ "$workflow_deny" != "$CARGO_DENY_VERSION" ]]; then
    return 1
  fi
}

require_cargo_tool_version() {
  local binary="$1"
  local subcommand="$2"
  local expected="$3"
  local output=""
  local installed=""

  if ! command -v "$binary" >/dev/null 2>&1; then
    if [[ "$binary" == "cargo-hakari" ]]; then
      hakari_status="missing"
    else
      deny_status="missing"
    fi
    return 127
  fi

  if ! output=$(cargo "$subcommand" --version 2>&1); then
    if [[ "$binary" == "cargo-hakari" ]]; then
      hakari_status="probe-failed"
    else
      deny_status="probe-failed"
    fi
    return 126
  fi

  installed=$(awk '{print $2}' <<<"$output")
  if [[ "$binary" == "cargo-hakari" ]]; then
    hakari_installed="$installed"
    hakari_status="ok"
  else
    deny_installed="$installed"
    deny_status="ok"
  fi

  if [[ "$installed" != "$expected" ]]; then
    if [[ "$binary" == "cargo-hakari" ]]; then
      hakari_status="mismatch"
    else
      deny_status="mismatch"
    fi
    return 1
  fi
}

run_real_command() {
  local effective_timeout="${ANVIL_RELEASE_GATE_TIMEOUT:-$STEP_TIMEOUT}"
  if declare -F "$1" >/dev/null 2>&1; then
    "$@"
  elif ! command -v "$1" >/dev/null 2>&1; then
    return 127
  elif [[ "$effective_timeout" != "0" ]] && command -v timeout >/dev/null 2>&1; then
    timeout "$effective_timeout" "$@"
  else
    "$@"
  fi
}

record_gate() {
  local gate_id="$1"
  local gate_name="$2"
  local command_label="$3"
  local status="$4"
  local exit_code="$5"
  local duration="$6"

  GATE_IDS+=("$gate_id")
  GATE_NAMES+=("$gate_name")
  GATE_COMMANDS+=("$command_label")
  GATE_STATUS+=("$status")
  GATE_EXIT_CODES+=("$exit_code")
  GATE_DURATIONS+=("$duration")
}

run_gate() {
  local gate_id="$1"
  local gate_name="$2"
  local command_label="$3"
  shift 3

  local start=$SECONDS
  local rc=0
  local status="passed"
  local fixture="${ANVIL_RELEASE_PREFLIGHT_FIXTURE:-}"

  if [[ -n "$fixture" ]]; then
    if [[ "$fixture" == "version-mismatch" && ( "$gate_id" == "hakari-version" || "$gate_id" == "deny-version" ) ]]; then
      rc=1
      status="failed"
      if [[ "$gate_id" == "hakari-version" ]]; then
        hakari_installed="0.0.0"
        hakari_status="mismatch"
      else
        deny_installed="0.0.0"
        deny_status="mismatch"
      fi
    elif [[ "$fixture" == "missing-tool" && "$gate_id" == "pnpm-test" ]]; then
      rc=127
      status="failed"
      if (( reserved_exit == 0 )); then
        reserved_exit=127
      fi
    elif [[ "$fixture" == "fail" ]] && fixture_has_failure "$gate_id"; then
      rc=1
      status="failed"
    elif [[ "$fixture" != "pass" && "$fixture" != "fail" && "$fixture" != "version-mismatch" && "$fixture" != "missing-tool" ]]; then
      rc=1
      status="failed"
    fi
  else
    run_real_command "$@" >/dev/null 2>&1 || rc=$?
    if (( rc != 0 )); then
      status="failed"
      if (( rc == 126 || rc == 127 )) && (( reserved_exit == 0 )); then
        reserved_exit="$rc"
      fi
    fi
  fi

  record_gate "$gate_id" "$gate_name" "$command_label" "$status" "$rc" "$((SECONDS - start))"
}

run_gates() {
  if [[ "${ANVIL_RELEASE_PREFLIGHT_FIXTURE:-}" == "pass" || "${ANVIL_RELEASE_PREFLIGHT_FIXTURE:-}" == "fail" ]]; then
    hakari_installed="$CARGO_HAKARI_VERSION"
    hakari_status="ok"
    deny_installed="$CARGO_DENY_VERSION"
    deny_status="ok"
  fi

  run_gate "cargo-fmt" "cargo fmt" "cargo fmt --all --check" cargo fmt --all --check
  run_gate "release-pins" "release pins" "check .github/workflows/rust.yml release tool pins" require_release_tool_pins_synced
  run_gate "hakari-version" "hakari version" "cargo hakari --version" require_cargo_tool_version cargo-hakari hakari "$CARGO_HAKARI_VERSION"
  run_gate "cargo-hakari" "cargo hakari" "cargo hakari verify" cargo hakari verify
  run_gate "deny-version" "deny version" "cargo deny --version" require_cargo_tool_version cargo-deny deny "$CARGO_DENY_VERSION"
  run_gate "cargo-deny" "cargo deny" "cargo deny check" cargo deny check
  run_gate "cargo-clippy" "cargo clippy" "cargo clippy --workspace --all-targets -- -D warnings" cargo clippy --workspace --all-targets -- -D warnings
  run_gate "cargo-test" "cargo test" "cargo test --workspace" cargo test --workspace
  run_gate "pnpm-format" "pnpm format" "pnpm format:check" pnpm format:check
  run_gate "pnpm-lint" "pnpm lint" "pnpm lint:check" pnpm lint:check
  run_gate "pnpm-typecheck" "pnpm typecheck" "pnpm typecheck" pnpm typecheck
  ANVIL_RELEASE_GATE_TIMEOUT="$PNPM_TEST_TIMEOUT" run_gate "pnpm-test" "pnpm test" "pnpm test" pnpm test
}

failed_gate_count() {
  local count=0
  local i
  for i in "${!GATE_STATUS[@]}"; do
    if [[ "${GATE_STATUS[$i]}" != "passed" ]]; then
      count=$((count + 1))
    fi
  done
  printf '%s' "$count"
}

emit_human() {
  local failed_count="$1"

  echo "Preflight summary"
  printf "  %-18s %-8s %s\n" "gate" "status" "duration"
  printf "  %-18s %-8s %s\n" "------------------" "--------" "--------"

  local i
  for i in "${!GATE_IDS[@]}"; do
    printf "  %-18s %-8s %ss\n" "${GATE_NAMES[$i]}" "${GATE_STATUS[$i]}" "${GATE_DURATIONS[$i]}"
  done

  if (( failed_count == 0 )); then
    echo "All preflight gates passed."
  else
    echo "$failed_count preflight gate(s) failed. Address before release."
  fi
}

emit_json() {
  local started_at="$1"
  local ended_at="$2"
  local failed_count="$3"
  local status="success"
  if (( failed_count > 0 )); then
    status="failed"
  fi

  printf '{'
  printf '"schemaVersion":"1.0.0",'
  printf '"command":"preflight",'
  printf '"phase":"preflight",'
  printf '"mode":"compatibility",'
  printf '"status":%s,' "$(json_string "$status")"
  printf '"startedAt":%s,' "$(json_string "$started_at")"
  printf '"endedAt":%s,' "$(json_string "$ended_at")"
  printf '"repository":%s,' "$(json_string "$repo")"
  printf '"inputs":{"base":%s,"head":%s,"version":null,"sourceSha":null,"trackingIssue":null},' "$(json_nullable_string "$base")" "$(json_nullable_string "$head")"
  printf '"trackingIssue":{"repository":%s,"number":null,"url":null,"metadataCommentUrl":null},' "$(json_string "$repo")"
  printf '"releaseRecord":{"lifecycleState":"candidate","recordUrl":null,"sha256":null},'
  printf '"data":{"failedGateCount":%s,"passedGateCount":%s,"toolVersions":{' "$failed_count" "$((${#GATE_IDS[@]} - failed_count))"
  printf '"git":%s,' "$(tool_version git --version)"
  printf '"gh":%s,' "$(tool_version gh --version)"
  printf '"cargo":%s,' "$(tool_version cargo --version)"
  printf '"node":%s,' "$(tool_version node --version)"
  printf '"opa":%s,' "$(tool_version opa version)"
  printf '"pnpm":%s,' "$(tool_version pnpm --version)"
  printf '"cargoHakari":{"expected":%s,"installed":%s,"status":%s},' "$(json_string "$hakari_expected")" "$(json_nullable_string "$hakari_installed")" "$(json_string "$hakari_status")"
  printf '"cargoDeny":{"expected":%s,"installed":%s,"status":%s}' "$(json_string "$deny_expected")" "$(json_nullable_string "$deny_installed")" "$(json_string "$deny_status")"
  printf '},"gates":['

  local i
  local first=true
  for i in "${!GATE_IDS[@]}"; do
    if [[ "$first" == true ]]; then
      first=false
    else
      printf ','
    fi
    local gate_status="fail"
    if [[ "${GATE_STATUS[$i]}" == "passed" ]]; then
      gate_status="pass"
    fi
    printf '{"id":%s,"name":%s,"status":%s,"command":%s,"exitCode":%s,"durationMs":%s}' \
      "$(json_string "${GATE_IDS[$i]}")" \
      "$(json_string "${GATE_NAMES[$i]}")" \
      "$(json_string "$gate_status")" \
      "$(json_string "${GATE_COMMANDS[$i]}")" \
      "${GATE_EXIT_CODES[$i]}" \
      "$((GATE_DURATIONS[$i] * 1000))"
  done
  printf ']},'
  printf '"warnings":[],'
  printf '"failures":['

  first=true
  for i in "${!GATE_IDS[@]}"; do
    if [[ "${GATE_STATUS[$i]}" == "passed" ]]; then
      continue
    fi
    if [[ "$first" == true ]]; then
      first=false
    else
      printf ','
    fi
    local failure_code="validation-failed"
    if (( GATE_EXIT_CODES[$i] == 126 || GATE_EXIT_CODES[$i] == 127 )); then
      failure_code="infra-failed"
    fi
    printf '{"code":%s,"message":%s,"retryable":true,"recovery":"fix-and-rerun","evidence":{"command":%s,"url":null,"path":null,"gate":%s,"exitCode":%s}}' \
      "$(json_string "$failure_code")" \
      "$(json_string "${GATE_NAMES[$i]} failed")" \
      "$(json_string "${GATE_COMMANDS[$i]}")" \
      "$(json_string "${GATE_IDS[$i]}")" \
      "${GATE_EXIT_CODES[$i]}"
  done
  printf '],'
  if (( failed_count == 0 )); then
    printf '"next":{"command":%s,"reason":%s}' "$(json_string "prepare")" "$(json_string "Preflight passed; prepare the release candidate next.")"
  else
    printf '"next":{"command":%s,"reason":%s}' "$(json_string "preflight")" "$(json_string "Fix failed gates before preparing the release candidate.")"
  fi
  printf '}\n'
}

emit_invalid_json() {
  local started_at="$1"
  local ended_at="$2"
  local message="$3"

  printf '{'
  printf '"schemaVersion":"1.0.0","command":"preflight","phase":"preflight","mode":"compatibility","status":"failed",'
  printf '"startedAt":%s,"endedAt":%s,"repository":%s,' "$(json_string "$started_at")" "$(json_string "$ended_at")" "$(json_string "$repo")"
  printf '"inputs":{"base":%s,"head":%s,"version":null,"sourceSha":null,"trackingIssue":null},' "$(json_nullable_string "$base")" "$(json_nullable_string "$head")"
  printf '"trackingIssue":{"repository":%s,"number":null,"url":null,"metadataCommentUrl":null},' "$(json_string "$repo")"
  printf '"releaseRecord":{"lifecycleState":null,"recordUrl":null,"sha256":null},'
  printf '"data":{"failedGateCount":0,"passedGateCount":0,"toolVersions":{},"gates":[]},"warnings":[],'
  printf '"failures":[{"code":"invalid-input","message":%s,"retryable":false,"recovery":"correct-usage","evidence":{"command":"scripts/release/preflight.sh","url":null,"path":null}}],' "$(json_string "$message")"
  printf '"next":{"command":"preflight","reason":"Fix command arguments and rerun preflight."}}\n'
}

parse_args() {
  while (($# > 0)); do
    case "$1" in
      --json)
        json=true
        shift
        ;;
      --base)
        base="${2:-}"
        [[ -n "$base" ]] || { parse_error="--base requires a value"; return 129; }
        shift 2
        ;;
      --head)
        head="${2:-}"
        [[ -n "$head" ]] || { parse_error="--head requires a value"; return 129; }
        shift 2
        ;;
      --repo)
        repo="${2:-}"
        [[ -n "$repo" ]] || { parse_error="--repo requires a value"; return 129; }
        shift 2
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      *)
        parse_error="unknown argument: $1"
        echo "preflight: ${parse_error}" >&2
        return 129
        ;;
    esac
  done
}

prescan_json_arg() {
  local arg
  for arg in "$@"; do
    if [[ "$arg" == "--json" ]]; then
      json=true
      return 0
    fi
  done
}

main() {
  local started_at
  local ended_at
  started_at=$(timestamp)

  prescan_json_arg "$@"

  if ! parse_args "$@"; then
    ended_at=$(timestamp)
    if [[ "$json" == true ]]; then
      emit_invalid_json "$started_at" "$ended_at" "${parse_error:-invalid arguments}"
    fi
    exit 129
  fi

  local failed_count
  local exit_code

  run_gates
  ended_at=$(timestamp)
  failed_count=$(failed_gate_count)
  exit_code="${reserved_exit:-0}"
  if (( exit_code == 0 )); then
    exit_code="$failed_count"
    if (( exit_code > 125 )); then
      exit_code=125
    fi
  fi

  if [[ "$json" == true ]]; then
    emit_json "$started_at" "$ended_at" "$failed_count"
  else
    emit_human "$failed_count"
  fi

  exit "$exit_code"
}

main "$@"
