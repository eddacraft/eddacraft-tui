#!/usr/bin/env bash
# Contract tests for scripts/docs/docs-check.mjs and its surface scripts.
#
# These tests run against the *live* repository on purpose — the orchestrator's
# job is to drive the real surface scripts against the real corpus and apply
# the real baseline, so testing it in a sandboxed clone would mostly retest the
# sandbox infrastructure. What we lock here is the contract: labelled-output
# format, summary line shape, baseline absorption, --no-baseline behaviour,
# --json round-trip, and orchestrator exit codes. A regression in validator
# rules is caught by the per-surface unit tests (e.g. @eddacraft/anvil-docs-meta
# vitest cases) and by the baselined snapshot of the live corpus.
#
# tmp_root is used for per-case temp files (e.g. captured JSON output) and is
# unconditionally cleaned up on exit.

# Deliberate: no `pipefail`. The test cases use `echo | grep -q` and
# `printf | head -N` pipelines where the downstream command (head, grep -q)
# legitimately closes stdin early on large outputs, causing the upstream to
# exit 141 (SIGPIPE). With pipefail, those benign exits would cascade and
# abort the whole script. Each test case has its own explicit guard.
set -eu

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${script_dir}/../.." && pwd)"
orchestrator="${script_dir}/docs-check.mjs"
metadata_script="${script_dir}/check-metadata.mjs"
tags_script="${script_dir}/check-tags.mjs"
links_script="${script_dir}/check-links.mjs"
index_script="${script_dir}/check-index-freshness.mjs"
index_generator="${script_dir}/docs-index.mjs"

tmp_root=$(mktemp -d)
trap 'rm -rf "${tmp_root}"' EXIT

failures=0
pass() { printf '  ok: %s\n' "$1"; }
fail() { printf '  FAIL: %s\n' "$1"; failures=$((failures + 1)); }

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

# Case 2: index-freshness and asbuilt-paths real surfaces both run cleanly.
echo "case 2: index-freshness and asbuilt-paths surfaces run cleanly"
out="$(cd "${repo_root}" && node "${index_script}" 2>&1 || true)"
if echo "${out}" | grep -qE "^\[index-freshness\] summary: [0-9]+ errors, [0-9]+ warnings, [0-9]+ files checked$"; then
  pass "index-freshness real surface prints summary"
else
  fail "index-freshness summary missing; got: ${out}"
fi
set +e
(cd "${repo_root}" && node "${index_script}" >/dev/null 2>&1)
status=$?
set -e
if [[ "${status}" -eq 0 ]]; then
  pass "index-freshness exits 0 when live indexes are fresh"
else
  fail "index-freshness should exit 0 for fresh live indexes; got ${status}"
fi
out="$(node "${script_dir}/check-asbuilt-paths.mjs" 2>&1)"
if echo "${out}" | grep -qE "^\[asbuilt-paths\] summary: [0-9]+ errors, [0-9]+ warnings, [0-9]+ files checked$"; then
  pass "asbuilt-paths real surface prints summary"
else
  fail "asbuilt-paths summary missing; got: ${out}"
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
# asbuilt-paths is baselineable, so --update-baseline depends on its --json contract.
asbuilt_script="${script_dir}/check-asbuilt-paths.mjs"
asbuilt_json_tmp="${tmp_root}/asbuilt-paths.json"
(cd "${repo_root}" && node "${asbuilt_script}" --no-baseline --json) >"${asbuilt_json_tmp}" 2>/dev/null || true
if node -e "JSON.parse(require('node:fs').readFileSync(process.argv[1],'utf8'))" "${asbuilt_json_tmp}" 2>/dev/null; then
  pass "asbuilt-paths --json parses cleanly"
else
  fail "asbuilt-paths --json failed JSON.parse"
fi
json_tmp="${tmp_root}/index-freshness.json"
(cd "${repo_root}" && node "${index_script}" --json) >"${json_tmp}" 2>/dev/null || true
if node -e "JSON.parse(require('node:fs').readFileSync(process.argv[1],'utf8'))" "${json_tmp}" 2>/dev/null; then
  pass "index-freshness --json parses cleanly"
else
  fail "index-freshness --json failed JSON.parse"
fi

# Case 6b: generated-index checker detects missing and stale files in a fixture root.
echo "case 6b: docs:index detects missing, fresh, and stale generated indexes"
fixture_root="${tmp_root}/index-fixture"
mkdir -p "${fixture_root}/docs/governance"
cat >"${fixture_root}/docs/README.md" <<'EOF'
# Fixture README

| Type | Authority | Owner | Status | Freshness |
| --- | --- | --- | --- | --- |
| README | Authoritative | Fixtures | Live | Test fixture |

| Upstream | Downstream |
| --- | --- |
| scripts/docs/docs-index.mjs | docs/indexes/README.md |
EOF
cat >"${fixture_root}/docs/example.md" <<'EOF'
# Fixture Guide

| Type | Authority | Owner | Status | Freshness |
| --- | --- | --- | --- | --- |
| Guide | Authoritative | Fixtures | Live | Test fixture |

| Upstream | Downstream |
| --- | --- |
| scripts/docs/docs-index.mjs | docs/indexes/by-tag.md |

**Tags:** agent
EOF
cat >"${fixture_root}/docs/governance/tags-catalogue.md" <<'EOF'
# Fixture Tags Catalogue

| Type | Authority | Owner | Status | Freshness |
| --- | --- | --- | --- | --- |
| Guide | Authoritative | Fixtures | Live | Test fixture |

| Upstream | Downstream |
| --- | --- |
| scripts/docs/docs-index.mjs | docs/indexes/by-tag.md |

## Catalogue

| Tag | Meaning |
| --- | --- |
| `agent` | Fixture tag. |

`not-approved-example`
EOF
set +e
node "${index_generator}" --root "${fixture_root}" --check >/dev/null 2>&1
status=$?
set -e
if [[ "${status}" -ne 0 ]]; then
  pass "docs:index:check fixture fails before indexes exist"
else
  fail "docs:index:check fixture should fail before indexes exist"
fi
node "${index_generator}" --root "${fixture_root}" >/dev/null 2>&1
set +e
node "${index_generator}" --root "${fixture_root}" --check >/dev/null 2>&1
status=$?
set -e
if [[ "${status}" -eq 0 ]]; then
  pass "docs:index:check fixture passes after generation"
else
  fail "docs:index:check fixture should pass after generation; got ${status}"
fi
if grep -q "Fixture README" "${fixture_root}/docs/indexes/by-type.md" && grep -q "## agent" "${fixture_root}/docs/indexes/by-tag.md"; then
  pass "docs:index fixture includes README metadata and approved tag grouping"
else
  fail "docs:index fixture omitted README metadata or approved tag grouping"
fi
printf '\nmanual edit\n' >>"${fixture_root}/docs/indexes/by-type.md"
set +e
node "${index_generator}" --root "${fixture_root}" --check >/dev/null 2>&1
status=$?
set -e
if [[ "${status}" -ne 0 ]]; then
  pass "docs:index:check fixture fails on stale generated index"
else
  fail "docs:index:check fixture should fail on stale generated index"
fi
parse_error_root="${tmp_root}/index-parse-error-fixture"
mkdir -p "${parse_error_root}/docs"
cat >"${parse_error_root}/docs/bad.md" <<'EOF'
# Bad Governed Doc

| Type | Authority | Owner | Status | Freshness |
| --- | --- | --- | --- | --- |
| Guide | Authoritative | Fixtures | Live | Test fixture |
EOF
set +e
node "${index_generator}" --root "${parse_error_root}" --check >/dev/null 2>&1
status=$?
set -e
if [[ "${status}" -ne 0 ]]; then
  pass "docs:index:check fixture fails on governed parse errors"
else
  fail "docs:index:check fixture should fail on governed parse errors"
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
