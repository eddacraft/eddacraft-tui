#!/usr/bin/env bash
# CIB-320 / CIB-431: the Nx graph check must fail on an unnamed inferred
# project and pass on a clean tree. The RED proof runs in an isolated
# temporary copy so concurrent invocations cannot mutate the caller's
# tracked checkout, and leftover paths such as `.nxignore.tmp` stay intact.

set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
script="${repo_root}/scripts/ci/nx-graph-check.sh"
ci_workflow="${repo_root}/.github/workflows/ci.yml"
package_json="${repo_root}/package.json"
nxignore="${repo_root}/.nxignore"
fixture_pkg="${repo_root}/benchmarks/fixtures/devacc/mini-ts-service/package.json"
stale_tmp="${repo_root}/.nxignore.tmp"

tmp_root=$(mktemp -d "${TMPDIR:-/tmp}/nx-graph-check-test.XXXXXX")
sentinel_backup=""
sentinel_created=0

restore_sentinel() {
  if [[ "${sentinel_created}" -eq 1 ]]; then
    rm -f "${stale_tmp}"
  elif [[ -n "${sentinel_backup}" && -f "${sentinel_backup}" ]]; then
    cp -a "${sentinel_backup}" "${stale_tmp}"
  fi
}

cleanup() {
  restore_sentinel
  rm -rf "${tmp_root}"
}
trap cleanup EXIT

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

assert_contains() {
  local file="$1"
  local expected="$2"
  if [[ ! -f "${file}" ]]; then
    fail "expected ${file} to exist"
  fi
  if ! grep -Fq -- "${expected}" "${file}"; then
    fail "expected ${file} to contain: ${expected}"
  fi
}

file_digest() {
  sha256sum -- "$1" | awk '{print $1}'
}

job_block_contains() {
  local job="$1"
  local needle="$2"
  awk -v job="^  ${job}:$" -v needle="${needle}" '
    $0 ~ job { inside = 1; next }
    inside && /^  [a-z]/ { inside = 0 }
    inside && index($0, needle) { found = 1 }
    END { exit (found ? 0 : 1) }
  ' "${ci_workflow}"
}

job_block_excludes() {
  local job="$1"
  local needle="$2"
  if job_block_contains "${job}" "${needle}"; then
    fail "expected ${job} in ${ci_workflow} not to contain: ${needle}"
  fi
}

materialise_template() {
  local template="${tmp_root}/template"
  mkdir -p "${template}"
  git -C "${repo_root}" checkout-index --prefix="${template}/" -a
}

create_isolated_tree() {
  local dest="$1"
  mkdir -p "${dest}"
  cp -a "${tmp_root}/template/." "${dest}/"
  mkdir -p "${dest}/scripts/ci"
  cp -f "${script}" "${dest}/scripts/ci/nx-graph-check.sh"
  chmod +x "${dest}/scripts/ci/nx-graph-check.sh"
  ln -sfn "${repo_root}/node_modules" "${dest}/node_modules"
}

apply_unnamed_fixture() {
  local dest="$1"
  local filtered
  filtered=$(mktemp "${tmp_root}/nxignore.XXXXXX")
  grep -vx 'benchmarks/fixtures' "${dest}/.nxignore" > "${filtered}"
  mv "${filtered}" "${dest}/.nxignore"
  node -e '
    const fs = require("node:fs");
    const path = process.argv[1];
    const pkg = JSON.parse(fs.readFileSync(path, "utf8"));
    delete pkg.name;
    fs.writeFileSync(path, `${JSON.stringify(pkg, null, 2)}\n`);
  ' "${dest}/benchmarks/fixtures/devacc/mini-ts-service/package.json"
}

run_red_proof() {
  local dest="$1"
  create_isolated_tree "${dest}"
  apply_unnamed_fixture "${dest}"
  local red_out red_status=0
  set +e
  red_out=$(bash "${dest}/scripts/ci/nx-graph-check.sh" 2>&1)
  red_status=$?
  set -e
  [[ "${red_status}" -ne 0 ]] || fail "expected nx-graph-check to fail when the unnamed fixture is visible, got:
${red_out}"
  printf '%s\n' "${red_out}" | grep -Fq 'projects in the following directories have no name provided' ||
    fail "expected Nx unnamed-project error text, got:
${red_out}"
}

assert_caller_state() {
  local expected_nxignore="$1"
  local expected_fixture="$2"
  local expected_sentinel="$3"
  local actual_nxignore actual_fixture actual_sentinel
  actual_nxignore=$(file_digest "${nxignore}")
  actual_fixture=$(file_digest "${fixture_pkg}")
  actual_sentinel=$(file_digest "${stale_tmp}")
  [[ "${actual_nxignore}" == "${expected_nxignore}" ]] ||
    fail "caller's .nxignore changed during isolated proof"
  [[ "${actual_fixture}" == "${expected_fixture}" ]] ||
    fail "caller's fixture package.json changed during isolated proof"
  [[ "${actual_sentinel}" == "${expected_sentinel}" ]] ||
    fail "pre-existing ${stale_tmp} changed during isolated proof"
}

# --- contract: local script uses a cold cache ---
[[ -f "${script}" ]] || fail "expected ${script} to exist"
[[ -x "${script}" ]] || fail "expected ${script} to be executable"
assert_contains "${script}" 'NX_DAEMON=false'
assert_contains "${script}" 'NX_WORKSPACE_DATA_DIRECTORY'
assert_contains "${script}" 'NX_CACHE_DIRECTORY'
assert_contains "${script}" 'mktemp -d'
assert_contains "${script}" 'nx show projects'
assert_contains "${package_json}" '"nx:graph:check"'
assert_contains "${package_json}" 'scripts/ci/nx-graph-check.sh'

# --- contract: cleanup is installed after the first mktemp -d ---
awk '
  $0 ~ /mktemp -d/ { count++; mktemp_at[count] = NR }
  $0 ~ /trap cleanup EXIT/ { trap_at = NR }
  END {
    if (count < 2) {
      print "expected two mktemp -d calls" > "/dev/stderr"
      exit 1
    }
    if (trap_at == 0) {
      print "expected trap cleanup EXIT" > "/dev/stderr"
      exit 1
    }
    if (!(mktemp_at[1] < trap_at && trap_at < mktemp_at[2])) {
      print "expected trap cleanup EXIT after the first mktemp -d and before the second" > "/dev/stderr"
      exit 1
    }
  }
' "${script}" || fail "cleanup trap is not installed immediately after the first temp directory is created"

# --- contract: CI job is always-on, named Nx Graph, not path-gated ---
assert_contains "${ci_workflow}" '  nx-graph:'
assert_contains "${ci_workflow}" '    name: Nx Graph'
assert_contains "${ci_workflow}" 'bash scripts/ci/nx-graph-check.sh'
job_block_contains nx-graph 'if: always()' || fail 'expected nx-graph job to use if: always()'
job_block_excludes nx-graph 'detect-changes.outputs'
job_block_excludes nx-graph 'paths:'

# Included in the push merge-gate aggregator.
awk '
  /^  integration-readiness:/ { inside = 1; next }
  inside && /^  [a-z]/ { inside = 0 }
  inside && $0 ~ /- nx-graph$/ { found = 1 }
  END { exit (found ? 0 : 1) }
' "${ci_workflow}" || fail 'expected integration-readiness to depend on nx-graph'
assert_contains "${ci_workflow}" 'NX_GRAPH_RESULT'
assert_contains "${ci_workflow}" 'Nx Graph:${NX_GRAPH_RESULT}'

[[ -f "${nxignore}" ]] || fail "expected ${nxignore} to exist"
[[ -f "${fixture_pkg}" ]] || fail "expected ${fixture_pkg} to exist"

caller_nxignore_digest=$(file_digest "${nxignore}")
caller_fixture_digest=$(file_digest "${fixture_pkg}")

if [[ -e "${stale_tmp}" ]]; then
  sentinel_backup=$(mktemp "${tmp_root}/sentinel-backup.XXXXXX")
  cp -a "${stale_tmp}" "${sentinel_backup}"
else
  sentinel_created=1
fi
printf 'cib-431-sentinel %s\n' "${tmp_root}" > "${stale_tmp}"
sentinel_digest=$(file_digest "${stale_tmp}")

# --- GREEN: clean tree ---
clean_out=$(bash "${script}" 2>&1) || fail "expected clean-tree nx-graph-check to pass, got:
${clean_out}"
assert_caller_state "${caller_nxignore_digest}" "${caller_fixture_digest}" "${sentinel_digest}"

# --- RED: unnamed inferred project must fail with the Nx error text ---
materialise_template
run_red_proof "${tmp_root}/red"
assert_caller_state "${caller_nxignore_digest}" "${caller_fixture_digest}" "${sentinel_digest}"

# --- concurrent isolated proofs must not collide or touch the caller ---
(
  run_red_proof "${tmp_root}/red-a"
) >"${tmp_root}/red-a.log" 2>&1 &
pid_a=$!
(
  run_red_proof "${tmp_root}/red-b"
) >"${tmp_root}/red-b.log" 2>&1 &
pid_b=$!
status_a=0
status_b=0
wait "${pid_a}" || status_a=$?
wait "${pid_b}" || status_b=$?
if [[ "${status_a}" -ne 0 ]]; then
  fail "concurrent proof A failed:
$(cat "${tmp_root}/red-a.log")"
fi
if [[ "${status_b}" -ne 0 ]]; then
  fail "concurrent proof B failed:
$(cat "${tmp_root}/red-b.log")"
fi
assert_caller_state "${caller_nxignore_digest}" "${caller_fixture_digest}" "${sentinel_digest}"

# Restore must return the check to green on the caller's unmodified tree.
restored_out=$(bash "${script}" 2>&1) || fail "expected nx-graph-check to pass after restore, got:
${restored_out}"
assert_caller_state "${caller_nxignore_digest}" "${caller_fixture_digest}" "${sentinel_digest}"

# --- second mktemp failure must still remove the first temp directory ---
real_mktemp=$(command -v mktemp)
shim_dir="${tmp_root}/mktemp-shim"
mkdir -p "${shim_dir}"
first_dir_file="${shim_dir}/first-dir"
cat >"${shim_dir}/mktemp" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "-d" && "$#" -eq 1 ]]; then
  if [[ ! -s "${NX_GRAPH_CHECK_FIRST_DIR_FILE}" ]]; then
    dir=$("${NX_GRAPH_CHECK_REAL_MKTEMP}" -d)
    printf '%s\n' "${dir}" > "${NX_GRAPH_CHECK_FIRST_DIR_FILE}"
    printf '%s\n' "${dir}"
    exit 0
  fi
  echo "mktemp: simulated failure" >&2
  exit 1
fi
exec "${NX_GRAPH_CHECK_REAL_MKTEMP}" "$@"
EOF
chmod +x "${shim_dir}/mktemp"

set +e
NX_GRAPH_CHECK_REAL_MKTEMP="${real_mktemp}" \
  NX_GRAPH_CHECK_FIRST_DIR_FILE="${first_dir_file}" \
  PATH="${shim_dir}:${PATH}" \
  bash "${script}" >/dev/null 2>"${tmp_root}/mktemp-fail.err"
mktemp_status=$?
set -e
[[ "${mktemp_status}" -ne 0 ]] || fail "expected nx-graph-check to fail when the second mktemp -d fails"
[[ -s "${first_dir_file}" ]] || fail "first mktemp -d did not record a directory"
first_dir=$(cat "${first_dir_file}")
[[ ! -e "${first_dir}" ]] || fail "first temp directory leaked after second mktemp -d failed: ${first_dir}"

echo 'nx-graph-check contract and RED/GREEN proof passed'
