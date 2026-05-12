#!/usr/bin/env bash
# Fixture tests for scripts/docs/docs-check.mjs and its surface scripts.
#
# Each case builds a minimal sandbox repo under $TMPDIR, runs the orchestrator
# (or a single surface) against it, and asserts on exit code + output. The
# fixtures stay tiny on purpose — these tests lock the contract (output format,
# baseline behaviour, labelled summary), not the rules of every validator.

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${script_dir}/../.." && pwd)"
orchestrator="${script_dir}/docs-check.mjs"
metadata_script="${script_dir}/check-metadata.mjs"
tags_script="${script_dir}/check-tags.mjs"
links_script="${script_dir}/check-links.mjs"

tmp_root=$(mktemp -d)
trap 'rm -rf "${tmp_root}"' EXIT

failures=0
pass() { printf '  ok: %s\n' "$1"; }
fail() { printf '  FAIL: %s\n' "$1"; failures=$((failures + 1)); }

# Run a script via the live monorepo so its imports resolve against the real
# node_modules. The sandbox provides --root for input paths only.
run_surface() {
  local script="$1"
  shift
  node "${script}" "$@"
}

# Case 1: orchestrator surfaces the seven expected labels in summary order.
echo "case 1: orchestrator emits all seven surface labels"
out="$(cd "${repo_root}" && node "${orchestrator}" 2>&1 || true)"
for surface in metadata tags links aps adr index-freshness asbuilt-paths; do
  if ! grep -qE "^  (pass|FAIL) ${surface}$" <<<"${out}"; then
    fail "summary missing surface: ${surface}"
    break
  fi
done
if grep -qE "^  (pass|FAIL) asbuilt-paths$" <<<"${out}"; then
  pass "all seven surfaces present in summary"
fi

# Case 2: stubs always pass and print pending-task note.
echo "case 2: stubs no-op cleanly"
out="$(node "${script_dir}/check-index-freshness.mjs" 2>&1)"
if echo "${out}" | grep -q "pending DOCGOV-007"; then
  pass "index-freshness stub prints DOCGOV-007 pending note"
else
  fail "index-freshness stub note missing; got: ${out}"
fi
out="$(node "${script_dir}/check-asbuilt-paths.mjs" 2>&1)"
if echo "${out}" | grep -q "pending DOCGOV-006"; then
  pass "asbuilt-paths stub prints DOCGOV-006 pending note"
else
  fail "asbuilt-paths stub note missing; got: ${out}"
fi

# Case 3: baseline absorbs current errors so the live repo passes.
echo "case 3: baseline file absorbs current errors"
out="$(cd "${repo_root}" && node "${orchestrator}" 2>&1 || true)"
if echo "${out}" | grep -qE "^\[docs-check\] 7/7 surfaces passed"; then
  pass "live repo passes all seven surfaces under baseline"
else
  fail "live repo expected 7/7 passed; got tail: $(echo "${out}" | tail -3)"
fi

# Case 4: --no-baseline reveals the baselined corpus errors.
echo "case 4: --no-baseline surfaces underlying errors"
out="$(cd "${repo_root}" && node "${orchestrator}" --no-baseline 2>&1 || true)"
if echo "${out}" | grep -qE "FAIL metadata"; then
  pass "without baseline, metadata surface fails as expected"
else
  fail "expected FAIL metadata without baseline; tail: $(echo "${out}" | tail -5)"
fi

# Case 5: each surface emits findings in [<surface>] <severity>: <file>:<line> — <message> format.
echo "case 5: surface findings honour the labelled-output contract"
out="$(cd "${repo_root}" && node "${metadata_script}" --no-baseline 2>&1 || true)"
out="$(printf '%s\n' "${out}" | head -5)"
if echo "${out}" | grep -qE "^\[metadata\] (ERROR|WARN): [^:]+:[0-9]+ — "; then
  pass "metadata findings match labelled contract"
else
  fail "metadata findings broke contract; got: ${out}"
fi
out="$(cd "${repo_root}" && node "${links_script}" --no-baseline 2>&1 || true)"
out="$(printf '%s\n' "${out}" | head -5)"
if echo "${out}" | grep -qE "^\[links\] (ERROR|WARN): [^:]+:[0-9]+ — "; then
  pass "links findings match labelled contract"
else
  fail "links findings broke contract; got: ${out}"
fi

# Case 6: --json round-trips through JSON.parse.
echo "case 6: surface --json output is valid JSON"
json_tmp="${tmp_root}/metadata.json"
(cd "${repo_root}" && node "${metadata_script}" --no-baseline --json) >"${json_tmp}" 2>/dev/null || true
if node -e "JSON.parse(require('node:fs').readFileSync(process.argv[1],'utf8'))" "${json_tmp}" 2>/dev/null; then
  pass "metadata --json parses cleanly"
else
  fail "metadata --json failed JSON.parse"
fi

# Case 7: summary line includes counts.
echo "case 7: surface summary lines include counts"
out="$(cd "${repo_root}" && node "${tags_script}" --no-baseline 2>&1 || true)"
out="$(printf '%s\n' "${out}" | tail -1)"
if echo "${out}" | grep -qE "^\[tags\] summary: [0-9]+ errors, [0-9]+ warnings, [0-9]+ files checked$"; then
  pass "tags summary line matches contract"
else
  fail "tags summary line broke contract; got: ${out}"
fi

# Case 8: orchestrator exits 1 when any surface fails.
echo "case 8: orchestrator exits non-zero when surfaces fail"
set +e
(cd "${repo_root}" && node "${orchestrator}" --no-baseline >/dev/null 2>&1)
status=$?
set -e
if [[ "${status}" -ne 0 ]]; then
  pass "orchestrator exits non-zero with --no-baseline (current corpus has errors)"
else
  fail "orchestrator should exit non-zero with --no-baseline; got ${status}"
fi

# Case 9: orchestrator exits 0 when baseline absorbs everything.
echo "case 9: orchestrator exits 0 under live baseline"
set +e
(cd "${repo_root}" && node "${orchestrator}" >/dev/null 2>&1)
status=$?
set -e
if [[ "${status}" -eq 0 ]]; then
  pass "orchestrator exits 0 under live baseline"
else
  fail "orchestrator should exit 0 under live baseline; got ${status}"
fi

if [[ "${failures}" -gt 0 ]]; then
  echo "${failures} test case(s) failed"
  exit 1
fi
echo "all cases passed"
