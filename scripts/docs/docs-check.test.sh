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

# Case 1: orchestrator surfaces the eight expected labels in summary order.
echo "case 1: orchestrator emits all eight surface labels"
out="$(cd "${repo_root}" && node "${orchestrator}" 2>&1 || true)"
for surface in metadata tags links aps adr index-freshness asbuilt-paths release-plan; do
  if ! grep -qE "^  (pass|FAIL) ${surface}$" <<<"${out}"; then
    fail "summary missing surface: ${surface}"
    break
  fi
done
if grep -qE "^  (pass|FAIL) release-plan$" <<<"${out}"; then
  pass "all eight surfaces present in summary"
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
if echo "${out}" | grep -qE "^\[docs-check\] 8/8 surfaces passed"; then
  pass "live repo passes all eight surfaces under baseline"
else
  fail "live repo expected 8/8 passed; got tail: $(echo "${out}" | tail -3)"
fi

# Case 4: --no-baseline reveals the baselined corpus errors. The metadata surface
# is fully backfilled (DOCGOV-011), so assert that *some* baselineable surface
# with retained corpus debt — links (docs-site absolute links), tags, or
# asbuilt-paths — fails without the baseline.
echo "case 4: --no-baseline surfaces underlying errors"
out="$(cd "${repo_root}" && node "${orchestrator}" --no-baseline 2>&1 || true)"
if echo "${out}" | grep -qE "FAIL (metadata|tags|links|asbuilt-paths)"; then
  pass "without baseline, a baselineable surface with corpus debt fails as expected"
else
  fail "expected a baselineable surface to FAIL without baseline; tail: $(echo "${out}" | tail -5)"
fi

# Case 5: the labelled-output contract [<surface>] <severity>: <file>:<line> — <message>.
# The metadata surface is now fully backfilled (DOCGOV-011 emptied its bucket),
# so it must emit no findings even without a baseline; the labelled-format
# contract is exercised against the links surface, which retains corpus debt.
echo "case 5: surface findings honour the labelled-output contract"
out="$(cd "${repo_root}" && node "${metadata_script}" --no-baseline 2>&1 || true)"
if echo "${out}" | grep -qE "^\[metadata\] (ERROR|WARN): "; then
  fail "metadata surface should be clean post-DOCGOV-011; got: $(printf '%s\n' "${out}" | head -3)"
else
  pass "metadata surface is fully backfilled (no findings without baseline)"
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

# Case 10 (DOCGOV-012 defect 2): --no-baseline must NOT be forwarded to a
# non-baselineable surface (index-freshness → docs-index.mjs), whose strict
# parseArgs rejects the unknown flag and would crash the surface.
echo "case 10: --no-baseline does not crash the index-freshness surface"
out="$(cd "${repo_root}" && node "${orchestrator}" --no-baseline 2>&1 || true)"
if echo "${out}" | grep -qE "Unknown option '--no-baseline'|ERR_PARSE_ARGS_UNKNOWN_OPTION"; then
  fail "--no-baseline misrouted to index-freshness; got: $(echo "${out}" | grep -iE 'unknown|ERR_PARSE' | head -1)"
elif echo "${out}" | grep -qE "^  (pass|FAIL) index-freshness$"; then
  pass "--no-baseline run reaches index-freshness without an unknown-option crash"
else
  fail "index-freshness surface missing from --no-baseline summary; tail: $(echo "${out}" | tail -5)"
fi

# Case 11 (DOCGOV-012 defect 1): --update-baseline must NOT overwrite the
# tracked baseline when a baselineable surface fails to emit valid JSON. Uses
# the --root / --surfaces test seam with stub surface scripts so the live
# corpus and tracked baseline are never touched.
echo "case 11: --update-baseline preserves the baseline on a partial/failed run"
bl_root="${tmp_root}/baseline-fixture"
mkdir -p "${bl_root}/docs/governance"
cat >"${bl_root}/good-surface.mjs" <<'EOF'
console.log(JSON.stringify({
  surface: 'good',
  findings: [{ severity: 'ERROR', file: 'docs/x.md', message: 'boom' }],
  summary: { errors: 1, warnings: 0, filesChecked: 1 },
}));
EOF
cat >"${bl_root}/bad-surface.mjs" <<'EOF'
console.log('this is not json {{{');
process.exit(1);
EOF
cat >"${bl_root}/surfaces.json" <<'EOF'
[
  { "name": "good", "script": "good-surface.mjs", "baselineable": true },
  { "name": "bad", "script": "bad-surface.mjs", "baselineable": true }
]
EOF
baseline_file="${bl_root}/docs/governance/docs-check.baseline.json"
cat >"${baseline_file}" <<'EOF'
{
  "good": { "docs/x.md": ["boom"] },
  "bad": { "docs/y.md": ["preexisting bad entry"] }
}
EOF
before_hash="$(node -e "process.stdout.write(require('node:fs').readFileSync(process.argv[1],'utf8'))" "${baseline_file}")"
set +e
(cd "${repo_root}" && node "${orchestrator}" --update-baseline --root "${bl_root}" --surfaces "${bl_root}/surfaces.json" >/dev/null 2>&1)
status=$?
set -e
if [[ "${status}" -ne 0 ]]; then
  pass "--update-baseline exits non-zero when a baselineable surface fails"
else
  fail "--update-baseline should exit non-zero on surface failure; got ${status}"
fi
after_hash="$(node -e "process.stdout.write(require('node:fs').readFileSync(process.argv[1],'utf8'))" "${baseline_file}")"
if [[ "${before_hash}" == "${after_hash}" ]]; then
  pass "--update-baseline left the existing baseline unchanged on failure"
else
  fail "--update-baseline overwrote the baseline despite a surface failure"
fi
# Happy path with the same seam: a fully-successful regeneration DOES write.
cat >"${bl_root}/surfaces-ok.json" <<'EOF'
[
  { "name": "good", "script": "good-surface.mjs", "baselineable": true }
]
EOF
set +e
(cd "${repo_root}" && node "${orchestrator}" --update-baseline --root "${bl_root}" --surfaces "${bl_root}/surfaces-ok.json" >/dev/null 2>&1)
status=$?
set -e
if [[ "${status}" -eq 0 ]] && node -e "const b=require(process.argv[1]); process.exit(b.good && b.good['docs/x.md'] && b.bad ? 0 : 1)" "${baseline_file}"; then
  pass "--update-baseline writes on full success and carries forward untouched keys"
else
  fail "--update-baseline happy path failed to write or dropped a carried-forward key"
fi

# Case 12 (DOCGOV-012 defect 3): a malformed percent escape in a link must
# produce a labelled ERROR finding and a non-zero exit, never an uncaught
# URIError that aborts the whole surface.
echo "case 12: check-links handles malformed percent escapes gracefully"
link_root="${tmp_root}/link-fixture"
mkdir -p "${link_root}/docs"
cat >"${link_root}/docs/bad.md" <<'EOF'
# Bad Link Doc

See [broken](./foo%zz.md) for details, and [anchor](#sec%) too.
EOF
set +e
out="$(cd "${repo_root}" && node "${links_script}" --root "${link_root}" --no-baseline 2>&1)"
status=$?
set -e
if echo "${out}" | grep -qiE "URIError|URI malformed"; then
  fail "check-links crashed on malformed percent escape; got: $(echo "${out}" | head -2)"
elif echo "${out}" | grep -qE "^\[links\] ERROR: docs/bad\.md:[0-9]+ — malformed link " && [[ "${status}" -ne 0 ]]; then
  pass "check-links emits a labelled ERROR and exits non-zero on malformed percent escape"
else
  fail "check-links did not emit a labelled malformed-link ERROR (status ${status}); got: $(echo "${out}" | head -3)"
fi

if [[ "${failures}" -gt 0 ]]; then
  echo "${failures} test case(s) failed"
  exit 1
fi
echo "all cases passed"
