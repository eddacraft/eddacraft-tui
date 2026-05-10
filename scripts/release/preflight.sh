#!/usr/bin/env bash
set -uo pipefail

readonly DEFAULT_REPO="eddacraft/anvil-001"
readonly CARGO_HAKARI_VERSION="0.9.37"
readonly CARGO_DENY_VERSION="0.19.4"
readonly STEP_TIMEOUT="${ANVIL_RELEASE_STEP_TIMEOUT:-600}"

json=false
base=""
head=""
repo="$DEFAULT_REPO"

declare -a GATE_IDS=()
declare -a GATE_NAMES=()
declare -a GATE_COMMANDS=()
declare -a GATE_STATUS=()
declare -a GATE_EXIT_CODES=()
declare -a GATE_DURATIONS=()

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

Test fixture mode:
  ANVIL_RELEASE_PREFLIGHT_FIXTURE=pass|fail|version-mismatch
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
    return 1
  fi

  if ! output=$(cargo "$subcommand" --version 2>&1); then
    if [[ "$binary" == "cargo-hakari" ]]; then
      hakari_status="probe-failed"
    else
      deny_status="probe-failed"
    fi
    return 1
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
  if declare -F "$1" >/dev/null 2>&1; then
    "$@"
  elif [[ "$STEP_TIMEOUT" != "0" ]] && command -v timeout >/dev/null 2>&1; then
    timeout "$STEP_TIMEOUT" "$@"
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
    elif [[ "$fixture" == "fail" ]] && fixture_has_failure "$gate_id"; then
      rc=1
      status="failed"
    elif [[ "$fixture" != "pass" && "$fixture" != "fail" && "$fixture" != "version-mismatch" ]]; then
      rc=1
      status="failed"
    fi
  else
    run_real_command "$@" >/dev/null 2>&1 || rc=$?
    if (( rc != 0 )); then
      status="failed"
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
  run_gate "pnpm-test" "pnpm test" "pnpm test" pnpm test
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
    printf '{"id":%s,"name":%s,"status":%s,"command":%s,"exitCode":%s,"durationSeconds":%s}' \
      "$(json_string "${GATE_IDS[$i]}")" \
      "$(json_string "${GATE_NAMES[$i]}")" \
      "$(json_string "${GATE_STATUS[$i]}")" \
      "$(json_string "${GATE_COMMANDS[$i]}")" \
      "${GATE_EXIT_CODES[$i]}" \
      "${GATE_DURATIONS[$i]}"
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
    printf '{"code":"validation-failed","message":%s,"retryable":true,"recovery":"fix-and-rerun","evidence":{"command":%s,"url":null,"path":null,"gate":%s,"exitCode":%s}}' \
      "$(json_string "${GATE_NAMES[$i]} failed")" \
      "$(json_string "${GATE_COMMANDS[$i]}")" \
      "$(json_string "${GATE_IDS[$i]}")" \
      "${GATE_EXIT_CODES[$i]}"
  done
  printf '],'
  printf '"next":{"command":%s,"reason":%s}' "$(json_nullable_string "prepare")" "$(json_string "Preflight passed; prepare the release candidate next.")"
  printf '}\n'
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
        [[ -n "$base" ]] || return 129
        shift 2
        ;;
      --head)
        head="${2:-}"
        [[ -n "$head" ]] || return 129
        shift 2
        ;;
      --repo)
        repo="${2:-}"
        [[ -n "$repo" ]] || return 129
        shift 2
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      *)
        echo "preflight: unknown argument: $1" >&2
        return 129
        ;;
    esac
  done
}

main() {
  if ! parse_args "$@"; then
    exit 129
  fi

  local started_at
  local ended_at
  local failed_count
  local exit_code

  started_at=$(timestamp)
  run_gates
  ended_at=$(timestamp)
  failed_count=$(failed_gate_count)
  exit_code="$failed_count"
  if (( exit_code > 125 )); then
    exit_code=125
  fi

  if [[ "$json" == true ]]; then
    emit_json "$started_at" "$ended_at" "$failed_count"
  else
    emit_human "$failed_count"
  fi

  exit "$exit_code"
}

main "$@"
