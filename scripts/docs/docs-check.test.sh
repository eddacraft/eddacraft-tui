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
public_script="${script_dir}/check-public-docs.mjs"
public_command_script="${script_dir}/check-anvil-public-commands.mjs"
aps_command_script="${script_dir}/check-aps-public-commands.mjs"
index_script="${script_dir}/check-index-freshness.mjs"
index_generator="${script_dir}/docs-index.mjs"

tmp_root=$(mktemp -d)
trap 'rm -rf "${tmp_root}"' EXIT

failures=0
pass() { printf '  ok: %s\n' "$1"; }
fail() { printf '  FAIL: %s\n' "$1"; failures=$((failures + 1)); }

# Case 1: orchestrator surfaces the eleven expected labels in summary order.
echo "case 1: orchestrator emits all eleven surface labels"
out="$(cd "${repo_root}" && node "${orchestrator}" 2>&1 || true)"
for surface in metadata tags links public-docs aps adr index-freshness asbuilt-paths docs-owed release-plan retired-claims; do
  if ! grep -qE "^  (pass|FAIL|ERROR \(tooling\))[[:space:]]+${surface}$" <<<"${out}"; then
    fail "summary missing surface: ${surface}"
    break
  fi
done
if grep -qE "^  (pass|FAIL|ERROR \(tooling\))[[:space:]]+retired-claims$" <<<"${out}"; then
  pass "all eleven surfaces present in summary"
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
if echo "${out}" | grep -qE "^\[docs-check\] 11/11 surfaces passed"; then
  pass "live repo passes all eleven surfaces under baseline"
else
  fail "live repo expected 11/11 passed; got tail: $(echo "${out}" | tail -3)"
fi

# Case 4: --no-baseline reveals the baselined corpus errors. The metadata surface
# is fully backfilled (DOCGOV-011), so assert that *some* baselineable surface
# with retained corpus debt — links (docs-site absolute links), tags, or
# asbuilt-paths — fails without the baseline.
echo "case 4: --no-baseline surfaces underlying errors"
out="$(cd "${repo_root}" && node "${orchestrator}" --no-baseline 2>&1 || true)"
if echo "${out}" | grep -qE "^  FAIL[[:space:]]+(metadata|tags|links|asbuilt-paths)$"; then
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
if grep -qE "Unknown option '--no-baseline'|ERR_PARSE_ARGS_UNKNOWN_OPTION" <<<"${out}"; then
  fail "--no-baseline misrouted to index-freshness; got: $(grep -im1E 'unknown|ERR_PARSE' <<<"${out}")"
elif grep -qE "^  (pass|FAIL|ERROR \(tooling\))[[:space:]]+index-freshness$" <<<"${out}"; then
  pass "--no-baseline run reaches index-freshness without an unknown-option crash"
else
  fail "index-freshness surface missing from --no-baseline summary; tail: $(tail -5 <<<"${out}")"
fi

# Case 13 (DOCSYNC-028): the public-doc boundary rejects internal leakage,
# product-name casing drift, hidden pages, and duplicated canonical install
# procedures while accepting a complete lowercase fixture.
echo "case 13: public-doc boundary enforces the newcomer trust contract"
public_root="${tmp_root}/public-docs-fixture"
mkdir -p "${public_root}/docs/public/anvil/guides" "${public_root}/apps/docs-site/sidebars" \
  "${public_root}/crates/anvil-cli/src"
printf 'fn main() {}\n' >"${public_root}/crates/anvil-cli/src/main.rs"
cat >"${public_root}/docs/public/anvil/quickstart.md" <<'EOF'
---
id: quickstart
title: Install anvil
description: Install anvil and verify the binary.
owner: DOCSYNC
upstream:
  - crates/anvil-cli/src/main.rs
verified_against: 0.9.4-beta
---

# Install anvil

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/eddacraft/anvil/releases/latest/download/eddacraft-anvil-installer.sh | sh
```
EOF
cat >"${public_root}/docs/public/anvil/guides/check-code.md" <<'EOF'
---
id: check-code
title: Check code
description: Run an anvil check.
owner: DOCSYNC
upstream:
  - crates/anvil-cli/src/main.rs
verified_against: 0.9.4-beta
---

# Check code

Continue after the canonical install task.
EOF
cat >"${public_root}/apps/docs-site/sidebars/anvil.ts" <<'EOF'
export default {
  anvilSidebar: [{
    type: 'category',
    label: 'internal',
    items: ['quickstart', { type: 'doc', id: 'guides/check-code', label: 'Check code' }],
  }],
};
EOF
set +e
out="$(node "${public_script}" --root "${public_root}" --skip-generated 2>&1)"
status=$?
set -e
if [[ "${status}" -eq 0 ]] && echo "${out}" | grep -qE "^\[public-docs\] summary: 0 errors, 3 files checked$"; then
  pass "complete lowercase public fixture passes"
else
  fail "valid public fixture should pass (status ${status}); got: ${out}"
fi
set +e
out="$(node "${public_script}" --root "${public_root}" 2>&1)"
status=$?
set -e
if [[ "${status}" -ne 0 ]] && echo "${out}" | grep -q "generated public reference checker is missing"; then
  pass "live-mode public boundary fails closed when the generator is missing"
else
  fail "public boundary should reject a missing generator (status ${status}); got: ${out}"
fi
cat >"${public_root}/docs/public/anvil/guides/internal.md" <<'EOF'
---
id: internal
title: anvil internals
description: Internal implementation notes.
owner: DOCSYNC
upstream:
  - crates/anvil-cli/src/main.rs
verified_against: 0.9.4-beta
---

# anvil internals

Read `/plans/index.aps.md`, then reinstall:

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/eddacraft/anvil/releases/latest/download/eddacraft-anvil-installer.sh | sh
```
EOF
cat >>"${public_root}/apps/docs-site/sidebars/anvil.ts" <<'EOF'
export const productLabel = 'Anvil';
export const editUrl = 'https://github.com/eddacraft/anvil-001';
export const omittedPage = 'internal';
EOF
set +e
out="$(node "${public_script}" --root "${public_root}" --skip-generated 2>&1)"
status=$?
set -e
if [[ "${status}" -ne 0 ]] \
  && echo "${out}" | grep -q "internal repository reference" \
  && echo "${out}" | grep -q "product name must be lowercase" \
  && echo "${out}" | grep -q "not present in the anvil sidebar" \
  && echo "${out}" | grep -q "duplicates the canonical install procedure"; then
  pass "internal leakage, casing, navigation, and duplicated setup fail together"
else
  fail "public boundary missed one or more defects (status ${status}); got: ${out}"
fi

# Case 13b (DOCSYNC-028): command truth includes inline YAML run steps, not only
# command lines that begin at the start of a fence.
echo "case 13b: public command truth checks inline YAML run steps"
command_root="${tmp_root}/public-command-fixture"
mkdir -p "${command_root}/docs/public/anvil"
cat >"${command_root}/docs/public/anvil/github.md" <<'EOF'
```yaml
run: anvil not-a-real-command
```
EOF
fake_anvil="${command_root}/fake-anvil"
cat >"${fake_anvil}" <<'EOF'
#!/usr/bin/env bash
if [[ "${1:-}" == "not-a-real-command" ]]; then
  echo "unknown command" >&2
  exit 2
fi
exit 0
EOF
chmod +x "${fake_anvil}"
set +e
out="$(node "${public_command_script}" --root "${command_root}" --anvil-bin "${fake_anvil}" 2>&1)"
status=$?
set -e
if [[ "${status}" -ne 0 ]] && echo "${out}" | grep -q "anvil not-a-real-command"; then
  pass "inline YAML run command is extracted and rejected"
else
  fail "public command truth skipped an inline YAML run step (status ${status}); got: ${out}"
fi

cat >"${command_root}/docs/public/anvil/help.md" <<'EOF'
```bash
anvil drift compare --help
```
EOF
cat >"${fake_anvil}" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$*" >>"${ANVIL_ARGS_FILE}"
exit 0
EOF
chmod +x "${fake_anvil}"
args_file="${command_root}/anvil-args"
ANVIL_ARGS_FILE="${args_file}" node "${public_command_script}" --root "${command_root}" --anvil-bin "${fake_anvil}" >/dev/null 2>&1
if grep -qx "drift compare --help" "${args_file}" && ! grep -q -- "--help --help" "${args_file}"; then
  pass "existing help flags are probed without duplication"
else
  fail "public command truth duplicated an existing help flag; got: $(tr '\n' ';' <"${args_file}")"
fi

# Case 13c (DOCFRESH-008): public pages carry governance frontmatter.
# In-tree product sections (anvil, start-here, beta, edda-stack) require
# the full triple. Copied sections (kindling, aps) require owner plus
# verified_against as the imported product version; optional upstream
# must resolve if declared. Attested copies are counted visibly.
echo "case 13c: public pages declare governance frontmatter"
gov_root="${tmp_root}/public-governance-fixture"
mkdir -p "${gov_root}/docs/public/anvil" "${gov_root}/docs/public/kindling" \
  "${gov_root}/docs/public/edda-stack" "${gov_root}/apps/docs-site/sidebars" \
  "${gov_root}/crates/anvil-cli/src" "${gov_root}/packages/edda-stack"
printf 'fn main() {}\n' >"${gov_root}/crates/anvil-cli/src/main.rs"
printf '# edda-stack\n' >"${gov_root}/packages/edda-stack/README.md"
cat >"${gov_root}/apps/docs-site/sidebars/anvil.ts" <<'EOF'
export default {
  anvilSidebar: ['governed'],
};
EOF
cat >"${gov_root}/docs/public/anvil/governed.md" <<'EOF'
---
id: governed
title: Governed page
description: A fully governed anvil page.
owner: DOCSYNC
upstream:
  - crates/anvil-cli/src/main.rs
verified_against: 0.9.4-beta
---

# Governed page
EOF
cat >"${gov_root}/docs/public/edda-stack/overview.md" <<'EOF'
---
id: overview
title: edda-stack overview
description: In-tree product section page.
owner: DOCSYNC
upstream:
  - packages/edda-stack/README.md
verified_against: 0.9.4-beta
---

# edda-stack overview
EOF
cat >"${gov_root}/docs/public/kindling/index.md" <<'EOF'
---
id: index
title: kindling overview
description: Copied-section page attested to an imported product version.
owner: DOCSYNC
verified_against: 0.2.0
public_unlisted: true
---

# kindling overview
EOF
set +e
out="$(node "${public_script}" --root "${gov_root}" --skip-generated 2>&1)"
status=$?
set -e
if [[ "${status}" -eq 0 ]] \
  && echo "${out}" | grep -qE "^\[public-docs\] summary: 0 errors, [0-9]+ files checked$" \
  && echo "${out}" | grep -q "1 copied-section page(s) attested against an imported product version"; then
  pass "full triple passes in-tree including edda-stack; attested copy is counted visibly"
else
  fail "governed fixture should pass with a visible copied-section count (status ${status}); got: ${out}"
fi
cat >"${gov_root}/docs/public/kindling/index.md" <<'EOF'
---
id: index
title: kindling overview
description: Copied-section page without a version pin.
owner: DOCSYNC
public_unlisted: true
---

# kindling overview
EOF
set +e
out="$(node "${public_script}" --root "${gov_root}" --skip-generated 2>&1)"
status=$?
set -e
if [[ "${status}" -ne 0 ]] \
  && echo "${out}" | grep -q "missing governance frontmatter: verified_against"; then
  pass "copied-section page cannot omit verified_against"
else
  fail "copied section should fail without verified_against (status ${status}); got: ${out}"
fi
cat >"${gov_root}/docs/public/kindling/index.md" <<'EOF'
---
id: index
title: kindling overview
description: Copied-section page with a dead optional upstream.
owner: DOCSYNC
upstream:
  - packages/kindling-integration/README.md
verified_against: 0.2.0
public_unlisted: true
---

# kindling overview
EOF
set +e
out="$(node "${public_script}" --root "${gov_root}" --skip-generated 2>&1)"
status=$?
set -e
if [[ "${status}" -ne 0 ]] \
  && echo "${out}" | grep -q "upstream path does not exist: packages/kindling-integration/README.md"; then
  pass "copied-section optional upstream must resolve when declared"
else
  fail "dead optional upstream on a copied page should fail (status ${status}); got: ${out}"
fi
cat >"${gov_root}/docs/public/anvil/governed.md" <<'EOF'
---
id: governed
title: Governed page
description: A fully governed anvil page.
upstream:
  - crates/anvil-cli/src/does-not-exist.rs
verified_against: v0.9.4-beta
---

# Governed page
EOF
cat >"${gov_root}/docs/public/kindling/index.md" <<'EOF'
---
id: index
title: kindling overview
description: Copied-section page attested to an imported product version.
owner: DOCSYNC
verified_against: 0.2.0
public_unlisted: true
---

# kindling overview
EOF
set +e
out="$(node "${public_script}" --root "${gov_root}" --skip-generated 2>&1)"
status=$?
set -e
if [[ "${status}" -ne 0 ]] \
  && echo "${out}" | grep -q "missing governance frontmatter: owner" \
  && echo "${out}" | grep -q "upstream path does not exist: crates/anvil-cli/src/does-not-exist.rs" \
  && echo "${out}" | grep -q "verified_against must be a bare product version"; then
  pass "missing owner, dead upstream path, and v-prefixed version fail together"
else
  fail "governance validation missed a defect (status ${status}); got: ${out}"
fi
cat >"${gov_root}/docs/public/anvil/governed.md" <<'EOF'
---
id: governed
title: Governed page
description: A fully governed anvil page.
owner: DOCSYNC
---

# Governed page
EOF
cat >"${gov_root}/docs/public/edda-stack/overview.md" <<'EOF'
---
id: overview
title: edda-stack overview
description: In-tree product section page.
owner: DOCSYNC
---

# edda-stack overview
EOF
set +e
out="$(node "${public_script}" --root "${gov_root}" --skip-generated 2>&1)"
status=$?
set -e
if [[ "${status}" -ne 0 ]] \
  && echo "${out}" | grep -q "docs/public/anvil/governed.md:1 — missing governance frontmatter: upstream" \
  && echo "${out}" | grep -q "docs/public/anvil/governed.md:1 — missing governance frontmatter: verified_against" \
  && echo "${out}" | grep -q "docs/public/edda-stack/overview.md:1 — missing governance frontmatter: upstream" \
  && echo "${out}" | grep -q "docs/public/edda-stack/overview.md:1 — missing governance frontmatter: verified_against"; then
  pass "in-tree section page, including edda-stack, cannot omit upstream or verified_against"
else
  fail "in-tree governance requirements not enforced (status ${status}); got: ${out}"
fi
# Restore the in-tree fixtures so later leakage assertions stay at two errors.
cat >"${gov_root}/docs/public/edda-stack/overview.md" <<'EOF'
---
id: overview
title: edda-stack overview
description: In-tree product section page.
owner: DOCSYNC
upstream:
  - packages/edda-stack/README.md
verified_against: 0.9.4-beta
---

# edda-stack overview
EOF
# Only the governance keys are exempt from the leakage scan — frontmatter
# title and description render into built HTML (page <title>, meta tags) and
# stay under the newcomer trust contract.
cat >"${gov_root}/docs/public/anvil/governed.md" <<'EOF'
---
id: governed
title: Anvil internals
description: See crates/anvil-cli/src/main.rs for details.
owner: DOCSYNC
upstream:
  - crates/anvil-cli/src/main.rs
verified_against: 0.9.4-beta
---

# Governed page
EOF
cat >"${gov_root}/docs/public/kindling/index.md" <<'EOF'
---
id: index
title: kindling overview
description: Copied-section page attested to an imported product version.
owner: DOCSYNC
verified_against: 0.2.0
public_unlisted: true
---

# kindling overview
EOF
set +e
out="$(node "${public_script}" --root "${gov_root}" --skip-generated 2>&1)"
status=$?
set -e
if [[ "${status}" -ne 0 ]] \
  && echo "${out}" | grep -q "product name must be lowercase" \
  && echo "${out}" | grep -q "internal repository reference" \
  && echo "${out}" | grep -qE "^\[public-docs\] summary: 2 errors, [0-9]+ files checked$"; then
  pass "rendered frontmatter (title, description) stays scanned; governance keys stay exempt"
else
  fail "frontmatter leakage boundary wrong (status ${status}); got: ${out}"
fi

# Case 14 (DOCSYNC-028): release-tag reference generation handles every Clap
# variant shape, ignores post-release source, and rejects hand-edited output.
echo "case 14: generated anvil references detect stale output"
generator="${script_dir}/generate-anvil-public-reference.mjs"
generated_root="${tmp_root}/generated-reference-fixture"
mkdir -p \
  "${generated_root}/patterns/compiled" \
  "${generated_root}/crates/anvil-cli/src" \
  "${generated_root}/crates/anvil-cli/src/commands" \
  "${generated_root}/crates/anvil-cli/src/activation" \
  "${generated_root}/crates/anvil-kernel/src/parser"
cat >"${generated_root}/patterns/compiled/registry.json" <<'EOF'
{
  "families": [{ "id": "rust-reliability" }],
  "patterns": [{
    "id": "RS-001",
    "family": "rust-reliability",
    "title": "Example rule",
    "severity": "warning",
    "file_extensions": [".rs"],
    "enabled": true
  }]
}
EOF
cat >"${generated_root}/crates/anvil-cli/src/main.rs" <<'EOF'
pub const EXIT_OK: u8 = 0;
pub const EXIT_ERROR: u8 = 1;
pub const EXIT_AUTH_REQUIRED: u8 = 3;

enum Commands {
    /// Check one file.
    #[command(name = "run-check")]
    Check(CheckArgs),
    /// Show status.
    Status,
    /// Configure a path.
    Configure {
        path: String,
    },
    /// Hidden runtime command.
    #[command(hide = true)]
    Hidden,
}

/// Canonical stable name for a command.
EOF
cat >"${generated_root}/crates/anvil-cli/src/activation/agent_registry.rs" <<'EOF'
pub enum AgentClientId {
    Cursor,
    ClaudeCode,
}

// Registry table form used by the live generator (display_name wins).
const CLIENTS: &[AgentClient] = &[
    AgentClient {
        display_name: "Cursor",
    },
    AgentClient {
        display_name: "Claude Code",
    },
];
EOF
cat >"${generated_root}/crates/anvil-cli/src/commands/start.rs" <<'EOF'
/// CLI args for anvil start.
pub struct StartArgs {
    /// Run a read-only activation probe.
    #[arg(long)]
    pub verify: bool,
    /// Skip MCP install.
    #[arg(long = "no-mcp")]
    pub no_mcp: bool,
}

/// Sentinel so the public-reference parser can close the struct body.
pub fn start_args_fixture_end() {}
EOF
cat >"${generated_root}/crates/anvil-kernel/src/parser/languages.rs" <<'EOF'
pub fn from_path(path: &Path) -> Option<Self> {
    match path.extension()?.to_str()? {
        "rs" => Some(Self::Rust),
        _ => None,
    }
}
EOF
cat >"${generated_root}/dist-workspace.toml" <<'EOF'
targets = ["x86_64-unknown-linux-gnu"]
EOF
mkdir -p "${generated_root}/docs/public/anvil/releases"
cat >"${generated_root}/docs/public/anvil/releases/changelog.md" <<'EOF'
# Current release notes

## 1.2.3-beta
EOF
git -C "${generated_root}" init -q
git -C "${generated_root}" config user.name "docs fixture"
git -C "${generated_root}" config user.email "docs-fixture@example.invalid"
git -C "${generated_root}" add .
git -C "${generated_root}" commit -qm "fixture release"
git -C "${generated_root}" tag v1.2.3-beta
cat >"${generated_root}/crates/anvil-cli/src/main.rs" <<'EOF'
enum Commands {
    /// Not in the public release.
    Unreleased,
}

/// Canonical stable name for a command.
EOF
node "${generator}" --root "${generated_root}" >/dev/null
set +e
node "${generator}" --root "${generated_root}" --check >/dev/null 2>&1
fresh_status=$?
generated_cli="${generated_root}/docs/public/anvil/reference/cli.md"
if grep -q 'anvil run-check' "${generated_cli}" \
  && grep -q 'anvil status' "${generated_cli}" \
  && grep -q 'anvil configure' "${generated_cli}" \
  && ! grep -qE 'anvil (hidden|unreleased)' "${generated_cli}"; then
  shape_status=0
else
  shape_status=1
fi
printf '\nmanual edit\n' >>"${generated_root}/docs/public/anvil/reference/cli.md"
node "${generator}" --root "${generated_root}" --check >/dev/null 2>&1
stale_status=$?
set -e
if [[ "${fresh_status}" -eq 0 && "${shape_status}" -eq 0 && "${stale_status}" -ne 0 ]]; then
  pass "release-tag generation covers Clap variants and rejects a hand edit"
else
  fail "generated reference contract failed (fresh ${fresh_status}, shapes ${shape_status}, stale ${stale_status})"
fi

# A resolved release tag is the source boundary for every product input. Make
# the next tag without dist-workspace.toml, then restore that file only in the
# workspace: generation must fail instead of silently mixing the two trees.
cat >"${generated_root}/docs/public/anvil/releases/changelog.md" <<'EOF'
# Current release notes

## 1.2.4-beta
EOF
git -C "${generated_root}" rm -q dist-workspace.toml
git -C "${generated_root}" add docs/public/anvil/releases/changelog.md
git -C "${generated_root}" commit -qm "fixture release missing one product input"
git -C "${generated_root}" tag v1.2.4-beta
cat >"${generated_root}/dist-workspace.toml" <<'EOF'
targets = ["x86_64-unknown-linux-gnu"]
EOF
set +e
missing_tag_out="$(node "${generator}" --root "${generated_root}" 2>&1)"
missing_tag_status=$?
set -e
if [[ "${missing_tag_status}" -ne 0 ]] \
  && grep -q 'v1.2.4-beta' <<<"${missing_tag_out}" \
  && grep -q 'dist-workspace.toml' <<<"${missing_tag_out}"; then
  pass "resolved release tag fails closed when a tagged product input is missing"
else
  fail "resolved tag mixed a workspace input (status ${missing_tag_status}); got: ${missing_tag_out}"
fi

# Pre-tag prepare PRs deliberately use one all-workspace source mode. Restore a
# valid workspace CLI, name the next release without creating its tag, and add
# a same-named branch that Git must not mistake for the tag. Require one
# ref-level fallback diagnostic rather than one decision per file.
git -C "${generated_root}" show v1.2.3-beta:crates/anvil-cli/src/main.rs \
  >"${generated_root}/crates/anvil-cli/src/main.rs"
cat >"${generated_root}/docs/public/anvil/releases/changelog.md" <<'EOF'
# Current release notes

## 1.2.5-beta
EOF
git -C "${generated_root}" branch v1.2.5-beta v1.2.3-beta
set +e
pretag_out="$(node "${generator}" --root "${generated_root}" 2>&1)"
pretag_status=$?
set -e
pretag_fallback_count="$(grep -c 'using workspace tree' <<<"${pretag_out}" || true)"
if [[ "${pretag_status}" -eq 0 ]] \
  && [[ "${pretag_fallback_count}" -eq 1 ]] \
  && grep -q 'v1.2.5-beta' <<<"${pretag_out}" \
  && grep -q 'anvil run-check' "${generated_root}/docs/public/anvil/reference/cli.md"; then
  pass "unresolved pre-tag release uses one all-workspace fallback mode"
else
  fail "pre-tag workspace fallback contract failed (status ${pretag_status}, fallbacks ${pretag_fallback_count}); got: ${pretag_out}"
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

# CIB-307: valid JSON is still unsafe baseline input when the surface process
# could not run successfully. A tooling-failed surface must leave every byte of
# the known-good baseline untouched and explain why its JSON was rejected.
cat >"${bl_root}/tooling-json-surface.mjs" <<'EOF'
console.log(JSON.stringify({
  surface: 'tooling-json',
  findings: [],
  summary: { errors: 0, warnings: 0, filesChecked: 0 },
}));
process.exit(2);
EOF
cat >"${bl_root}/surfaces-tooling-json.json" <<'EOF'
[
  {
    "name": "tooling-json",
    "script": "tooling-json-surface.mjs",
    "baselineable": true
  }
]
EOF
baseline_before_tooling="${bl_root}/baseline-before-tooling.json"
cp "${baseline_file}" "${baseline_before_tooling}"
set +e
out="$(cd "${repo_root}" && node "${orchestrator}" --update-baseline --root "${bl_root}" --surfaces "${bl_root}/surfaces-tooling-json.json" 2>&1)"
status=$?
set -e
if [[ "${status}" -ne 0 ]] && echo "${out}" | grep -qE "^\[docs-check\] tooling-json: ERROR \(tooling\)"; then
  pass "--update-baseline rejects valid JSON from a tooling-failed surface"
else
  fail "--update-baseline accepted tooling-failed JSON or omitted its diagnostic (status ${status}); got: ${out}"
fi
if cmp -s "${baseline_before_tooling}" "${baseline_file}"; then
  pass "--update-baseline preserves the baseline byte-for-byte on tooling failure"
else
  fail "--update-baseline changed the baseline after a tooling-failed surface"
fi

# A tooling failure's stdout is not trustworthy enough to parse. Non-JSON
# output must retain the tooling verdict instead of producing a parse error.
cat >"${bl_root}/tooling-text-surface.mjs" <<'EOF'
console.log('tooling startup failed before JSON initialisation');
process.exit(2);
EOF
cat >"${bl_root}/surfaces-tooling-text.json" <<'EOF'
[
  {
    "name": "tooling-text",
    "script": "tooling-text-surface.mjs",
    "baselineable": true
  }
]
EOF
set +e
out="$(cd "${repo_root}" && node "${orchestrator}" --update-baseline --root "${bl_root}" --surfaces "${bl_root}/surfaces-tooling-text.json" 2>&1)"
status=$?
set -e
if [[ "${status}" -ne 0 ]] \
  && echo "${out}" | grep -qE "^\[docs-check\] tooling-text: ERROR \(tooling\)" \
  && ! echo "${out}" | grep -q "JSON parse failed"; then
  pass "--update-baseline checks tooling verdict before parsing stdout"
else
  fail "--update-baseline misclassified tooling-failed text output (status ${status}); got: ${out}"
fi

# Guard the exit taxonomy from CIB-278: valid JSON from an ordinary content
# failure remains acceptable regeneration input.
cat >"${bl_root}/content-json-surface.mjs" <<'EOF'
console.log(JSON.stringify({
  surface: 'content-json',
  findings: [{ severity: 'ERROR', file: 'docs/content.md', message: 'content debt' }],
  summary: { errors: 1, warnings: 0, filesChecked: 1 },
}));
process.exit(1);
EOF
cat >"${bl_root}/surfaces-content-json.json" <<'EOF'
[
  { "name": "content-json", "script": "content-json-surface.mjs", "baselineable": true }
]
EOF
set +e
(cd "${repo_root}" && node "${orchestrator}" --update-baseline --root "${bl_root}" --surfaces "${bl_root}/surfaces-content-json.json" >/dev/null 2>&1)
status=$?
set -e
if [[ "${status}" -eq 0 ]] && node -e "const b=require(process.argv[1]); process.exit(b['content-json']?.['docs/content.md']?.includes('content debt') ? 0 : 1)" "${baseline_file}"; then
  pass "--update-baseline accepts valid JSON from a content-failed surface"
else
  fail "--update-baseline rejected valid content-failure JSON or failed to write it (status ${status})"
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

# Case 15 (DOCSYNC-029): the public-doc boundary applies the newcomer trust
# contract to APS, including casing, internal references, and complete sidebar
# discovery.
echo "case 15: APS public docs enforce the newcomer trust contract"
aps_public_root="${tmp_root}/aps-public-docs-fixture"
mkdir -p "${aps_public_root}/docs/public/aps/guides" "${aps_public_root}/apps/docs-site/sidebars"
cat >"${aps_public_root}/docs/public/aps/getting-started.md" <<'EOF'
---
id: getting-started
title: Create your first APS plan
description: Install APS and validate a first plan.
owner: DOCSYNC
verified_against: 0.6.0
---

# Create your first APS plan

Install APS, create a plan, and run `aps lint`.
EOF
cat >"${aps_public_root}/docs/public/aps/guides/agents.md" <<'EOF'
---
id: agents
title: Work with an AI agent
description: Give an AI agent a bounded APS work item.
owner: DOCSYNC
verified_against: 0.6.0
---

# Work with an AI agent

Start a ready item before asking an agent to implement it.
EOF
cat >"${aps_public_root}/apps/docs-site/sidebars/aps.ts" <<'EOF'
export default {
  apsSidebar: ['getting-started', { type: 'category', label: 'Guides', items: ['guides/agents'] }],
};
EOF
set +e
out="$(node "${public_script}" --root "${aps_public_root}" --skip-generated 2>&1)"
status=$?
set -e
if [[ "${status}" -eq 0 ]] && echo "${out}" | grep -qE "^\[public-docs\] summary: 0 errors, [0-9]+ files checked$"; then
  pass "complete lowercase APS fixture passes"
else
  fail "valid APS fixture should pass (status ${status}); got: ${out}"
fi
cat >"${aps_public_root}/docs/public/aps/guides/internal.md" <<'EOF'
---
id: internal
title: APS internals
description: Internal implementation notes.
owner: DOCSYNC
verified_against: 0.6.0
---

# APS internals

Read `/cli/src/main.rs` in the Anvil repository.
EOF
set +e
out="$(node "${public_script}" --root "${aps_public_root}" --skip-generated 2>&1)"
status=$?
set -e
if [[ "${status}" -ne 0 ]] \
  && echo "${out}" | grep -q "internal repository reference" \
  && echo "${out}" | grep -q "product name must be lowercase" \
  && echo "${out}" | grep -q "not present in the APS sidebar"; then
  pass "APS internal leakage, casing, and hidden pages fail together"
else
  fail "APS public boundary missed one or more defects (status ${status}); got: ${out}"
fi

# Case 16 (DOCSYNC-029): fenced APS commands are checked against a pinned
# upstream CLI contract rather than accepted as plausible prose.
echo "case 16: APS command examples follow the pinned CLI contract"
aps_command_root="${tmp_root}/aps-command-fixture"
mkdir -p "${aps_command_root}/docs/public/aps"
cat >"${aps_command_root}/docs/public/aps/commands.md" <<'EOF'
```bash
aps lint plans
aps --strict lint plans
aps next --package core
aps complete AUTH-003 --learning "Captured the retry rule"
```
EOF
set +e
out="$(node "${aps_command_script}" --root "${aps_command_root}" 2>&1)"
status=$?
set -e
if [[ "${status}" -eq 0 ]] && echo "${out}" | grep -qE "^\[aps-public-commands\] 4/4 fenced APS commands match"; then
  pass "valid APS commands pass"
else
  fail "valid APS commands should pass (status ${status}); got: ${out}"
fi
cat >"${aps_command_root}/docs/public/aps/commands.md" <<'EOF'
```bash
aps upgrade --apply
aps --strict archived
aps update --global
aps init --scope nested
```
EOF
set +e
out="$(node "${aps_command_script}" --root "${aps_command_root}" 2>&1)"
status=$?
set -e
if [[ "${status}" -ne 0 ]] \
  && echo "${out}" | grep -q "unknown command 'upgrade'" \
  && echo "${out}" | grep -q "unknown command 'archived'" \
  && echo "${out}" | grep -q "update does not accept --global" \
  && echo "${out}" | grep -q "init does not accept --scope"; then
  pass "removed APS commands and flags fail together"
else
  fail "APS command boundary missed removed syntax (status ${status}); got: ${out}"
fi

# Case 17 (CIB-278): a surface that could not RUN must not be rendered as a
# content defect. Exit code 2 is already this repo's "cannot run" convention —
# scripts/aps/drift-check.mjs and scripts/docs/adr-integrity.sh both reserve it
# for usage/environment errors — so the orchestrator must map it to a distinct
# tooling verdict rather than collapsing every non-zero status into FAIL.
echo "case 17: orchestrator separates tooling failure from content failure"
taxonomy_root="${tmp_root}/taxonomy-fixture"
mkdir -p "${taxonomy_root}/stubs"
cat >"${taxonomy_root}/stubs/ok.mjs" <<'EOF'
console.log('[ok-surface] summary: 0 errors, 0 warnings, 0 files checked');
process.exit(0);
EOF
cat >"${taxonomy_root}/stubs/content.mjs" <<'EOF'
console.error('[content-surface] ERROR: docs/thing.md:1 — a real content defect');
process.exit(1);
EOF
cat >"${taxonomy_root}/stubs/tooling.mjs" <<'EOF'
console.error('[tooling-surface] tooling failure: could not run the checker');
process.exit(2);
EOF

# 17a: exit 2 renders as a distinct tooling verdict, never as FAIL.
cat >"${taxonomy_root}/surfaces-tooling.json" <<'EOF'
[
  { "name": "ok-surface", "script": "stubs/ok.mjs", "baselineable": false },
  { "name": "tooling-surface", "script": "stubs/tooling.mjs", "baselineable": false }
]
EOF
set +e
out="$(node "${orchestrator}" --root "${taxonomy_root}" --surfaces "${taxonomy_root}/surfaces-tooling.json" 2>&1)"
status=$?
set -e
if echo "${out}" | grep -qE "^  FAIL[[:space:]]+tooling-surface$"; then
  fail "exit-2 surface rendered as a content FAIL; got: $(echo "${out}" | grep tooling-surface | head -2)"
elif echo "${out}" | grep -qE "^  ERROR \(tooling\)[[:space:]]+tooling-surface$"; then
  pass "exit-2 surface renders as ERROR (tooling), not FAIL"
else
  fail "exit-2 surface missing a tooling verdict; got: $(echo "${out}" | tail -8)"
fi

# 17b: the tooling failure names itself and states it is not a content defect,
# so a contributor is not sent hunting through their docs change.
if echo "${out}" | grep -qE "^\[docs-check\] .*tooling.*" \
  && echo "${out}" | grep -q "tooling-surface" \
  && echo "${out}" | grep -qiE "not a (docs )?content (defect|failure)"; then
  pass "tooling failure is called out with an actionable, non-content-defect line"
else
  fail "tooling failure lacks an actionable summary line; got: $(echo "${out}" | tail -6)"
fi

# 17c: a tooling failure still exits non-zero (it must stay loud), and uses the
# reserved code 2 rather than the content-failure code 1.
if [[ "${status}" -eq 2 ]]; then
  pass "tooling-only run exits 2 (loud, and distinct from content failure)"
else
  fail "tooling-only run should exit 2; got ${status}"
fi

# 17d: a genuine content failure is still FAIL and still exits 1 — the fix must
# not blunt real docs signal.
cat >"${taxonomy_root}/surfaces-content.json" <<'EOF'
[
  { "name": "ok-surface", "script": "stubs/ok.mjs", "baselineable": false },
  { "name": "content-surface", "script": "stubs/content.mjs", "baselineable": false }
]
EOF
set +e
out="$(node "${orchestrator}" --root "${taxonomy_root}" --surfaces "${taxonomy_root}/surfaces-content.json" 2>&1)"
status=$?
set -e
if echo "${out}" | grep -qE "^  FAIL[[:space:]]+content-surface$" && [[ "${status}" -eq 1 ]]; then
  pass "content failure still renders FAIL and exits 1"
else
  fail "content failure regressed (status ${status}); got: $(echo "${out}" | tail -8)"
fi

# 17e: a content failure outranks a tooling failure in the process exit code, so
# a broken tool can never mask a real docs defect.
cat >"${taxonomy_root}/surfaces-both.json" <<'EOF'
[
  { "name": "content-surface", "script": "stubs/content.mjs", "baselineable": false },
  { "name": "tooling-surface", "script": "stubs/tooling.mjs", "baselineable": false }
]
EOF
set +e
(node "${orchestrator}" --root "${taxonomy_root}" --surfaces "${taxonomy_root}/surfaces-both.json" >/dev/null 2>&1)
status=$?
set -e
if [[ "${status}" -eq 1 ]]; then
  pass "content failure outranks tooling failure in the exit code"
else
  fail "mixed run should exit 1 (content wins); got ${status}"
fi

# Case 18 (CIB-278): the aps and adr delegates must invoke their underlying
# checks directly instead of re-entering the package manager. A broken `pnpm`
# on PATH is the exact field condition that made both surfaces report a content
# FAIL when the corpus was clean; after the fix pnpm is not on the path at all.
echo "case 18: aps and adr surfaces do not re-enter the package manager"
shim_dir="${tmp_root}/broken-pm"
mkdir -p "${shim_dir}"
cat >"${shim_dir}/pnpm" <<'EOF'
#!/bin/sh
echo "ERR_VM_DYNAMIC_IMPORT_CALLBACK_MISSING" >&2
exit 1
EOF
chmod +x "${shim_dir}/pnpm"
for surface in aps adr; do
  set +e
  out="$(cd "${repo_root}" && PATH="${shim_dir}:${PATH}" node "${script_dir}/check-${surface}.mjs" 2>&1)"
  status=$?
  set -e
  if echo "${out}" | grep -q "ERR_VM_DYNAMIC_IMPORT_CALLBACK_MISSING"; then
    fail "${surface} surface still re-enters pnpm; got: ${out}"
  elif [[ "${status}" -eq 0 ]]; then
    pass "${surface} surface passes against the live corpus with pnpm unusable"
  else
    fail "${surface} surface should pass on a clean corpus (status ${status}); got: ${out}"
  fi
done
set +e
out="$(cd "${repo_root}" && PATH="${shim_dir}:${PATH}" node "${orchestrator}" 2>&1)"
status=$?
set -e
if echo "${out}" | grep -qE "^  FAIL[[:space:]]+(aps|adr)$"; then
  fail "broken pnpm still misreported as a content FAIL; got: $(echo "${out}" | tail -14)"
elif [[ "${status}" -eq 0 ]] && echo "${out}" | grep -qE "surfaces passed; 0 failed"; then
  pass "orchestrator is fully green with pnpm unusable"
else
  fail "orchestrator not green with pnpm unusable (status ${status}); got: $(echo "${out}" | tail -14)"
fi

# Case 19 (CIB-278): honouring exit code 2 downstream only works if every surface
# means the same thing by it. A missing governed document is a CONTENT defect and
# must exit 1 — if it exited 2 the orchestrator would tell the contributor who
# just deleted it that their docs change was not the cause, which is the original
# misattribution inverted. These lock the two sites that used to exit 2.
echo "case 19: a missing governed document is a content defect, not a tooling failure"
release_plan_root="${tmp_root}/release-plan-missing"
mkdir -p "${release_plan_root}"
set +e
out="$(node "${script_dir}/check-release-plan.mjs" --root "${release_plan_root}" 2>&1)"
status=$?
set -e
if [[ "${status}" -eq 1 ]]; then
  pass "missing RELEASE-PLAN.md exits 1 (content), not 2 (tooling)"
else
  fail "missing RELEASE-PLAN.md should exit 1; got ${status}: ${out}"
fi

# adr-integrity.sh roots itself at dirname(BASH_SOURCE)/../.., so drive it from a
# scratch tree with the real script copied in.
adr_root="${tmp_root}/adr-missing"
mkdir -p "${adr_root}/scripts/docs" "${adr_root}/plans/decisions"
cp "${script_dir}/adr-integrity.sh" "${adr_root}/scripts/docs/adr-integrity.sh"
set +e
out="$(bash "${adr_root}/scripts/docs/adr-integrity.sh" 2>&1)"
status=$?
set -e
if [[ "${status}" -eq 1 ]]; then
  pass "missing DECISION-LOG.md exits 1 (content), not 2 (tooling)"
else
  fail "missing DECISION-LOG.md should exit 1; got ${status}: ${out}"
fi
rm -rf "${adr_root}/plans/decisions"
set +e
out="$(bash "${adr_root}/scripts/docs/adr-integrity.sh" 2>&1)"
status=$?
set -e
if [[ "${status}" -eq 1 ]]; then
  pass "missing plans/decisions exits 1 (content), not 2 (tooling)"
else
  fail "missing plans/decisions should exit 1; got ${status}: ${out}"
fi

# And end-to-end: the orchestrator must render that as FAIL, never ERROR (tooling).
orchestrator_root="${tmp_root}/release-plan-orchestrated"
mkdir -p "${orchestrator_root}"
ln -s "${script_dir}" "${orchestrator_root}/scripts-docs"
cat >"${orchestrator_root}/surfaces.json" <<'EOF'
[
  { "name": "release-plan", "script": "scripts-docs/check-release-plan.mjs", "baselineable": false }
]
EOF
set +e
out="$(node "${orchestrator}" --root "${orchestrator_root}" --surfaces "${orchestrator_root}/surfaces.json" 2>&1)"
status=$?
set -e
if echo "${out}" | grep -qE "^  ERROR \(tooling\)[[:space:]]+release-plan$"; then
  fail "missing RELEASE-PLAN.md wrongly blamed on tooling; got: $(echo "${out}" | tail -6)"
elif echo "${out}" | grep -qE "^  FAIL[[:space:]]+release-plan$" && [[ "${status}" -eq 1 ]]; then
  pass "orchestrator renders a missing governed doc as FAIL and exits 1"
else
  fail "missing RELEASE-PLAN.md not rendered as FAIL (status ${status}); got: $(echo "${out}" | tail -6)"
fi

# --- docs-owed surface (DOCFRESH-001, ADR-119) -------------------------------
#
# These lock the three properties that make the surface safe to register but not
# yet safe to gate. Each one guards a mistake that would be invisible in normal
# output: a gate switched on too early, a ratchet that cannot hold, and an
# unreadable corpus reported as a clean one.

owed_script="${script_dir}/check-docs-owed.mjs"

# Case A: report-only under docs:check. The surface finds real errors in the live corpus
# with --no-baseline, and must STILL exit 0. Gating cannot switch on before
# DOCFRESH-002 lands the granularity split, because severity is currently
# derived from the confidence class alone and would fail on the directory
# upstreams ADR-119 D2 requires to stay advisory.
echo "case A: docs-owed reports without gating"
set +e
out="$(cd "${repo_root}" && node "${owed_script}" --no-baseline --limit 0 2>&1)"
status=$?
set -e
if [[ "${status}" -ne 0 ]]; then
  fail "docs-owed must exit 0 while report-only; got ${status}"
elif ! echo "${out}" | grep -qE "^\[docs-owed\] summary \[corpus\]: [0-9]+ owed \([0-9]+ gating, [0-9]+ advisory by granularity\), [0-9]+ review, [0-9]+ baselined, [0-9]+ checked,"; then
  fail "docs-owed summary line shape changed; got: $(echo "${out}" | tail -2)"
else
  pass "docs-owed reports findings and exits 0 (docs:check gating deferred to DOCFRESH-003)"
fi

# Case B: --fail-on-owed is the opt-in that proves the gate works before it is
# switched on. Without it the same corpus exits 0; with it, exit 1.
# Guarded on the corpus actually having owed findings. Asserting exit 1
# unconditionally would turn this green test red the day the backlog is burned
# down — at which point exit 0 is the *correct* answer for an empty corpus.
echo "case B: docs-owed --fail-on-owed opts into failure"
owed_now="$(cd "${repo_root}" && node "${owed_script}" --no-baseline --limit 0 2>/dev/null |
  sed -n 's/^\[docs-owed\] summary \[corpus\]: \([0-9]*\) owed .*/\1/p')"
if [[ -z "${owed_now}" ]]; then
  fail "could not read owed count from docs-owed summary"
elif [[ "${owed_now}" -eq 0 ]]; then
  pass "skipped: corpus has 0 owed findings, so exit 0 is correct for --fail-on-owed"
else
  set +e
  (cd "${repo_root}" && node "${owed_script}" --no-baseline --limit 0 --fail-on-owed >/dev/null 2>&1)
  gated=$?
  set -e
  if [[ "${gated}" -eq 1 ]]; then
    pass "--fail-on-owed exits 1 on unbaselined owed findings"
  else
    fail "--fail-on-owed expected exit 1 with ${owed_now} owed; got ${gated}"
  fi
fi

# Case C: the ratchet holds. The baseline matches findings by exact message, so
# a message carrying a commit date would un-baseline itself the moment its
# upstream took one more commit and the absorbed backlog would reappear as fresh
# errors on an unrelated pull request. Assert no finding message contains a date.
echo "case C: docs-owed fingerprints are date-free"
json_out="${tmp_root}/docs-owed.json"
if ! (cd "${repo_root}" && node "${owed_script}" --no-baseline --json >"${json_out}" 2>/dev/null); then
  fail "docs-owed --json did not run"
elif ! node -e '
    const j = JSON.parse(require("node:fs").readFileSync(process.argv[1], "utf8"));
    // An empty corpus is a pass, not a failure: there is nothing that could
    // carry a volatile fingerprint. Asserting findings exist would make this
    // case a time bomb once the backlog is cleared.
    const dated = j.findings.filter((f) => /\d{4}-\d{2}-\d{2}/.test(f.message));
    if (dated.length) {
      console.error("volatile message: " + dated[0].message);
      process.exit(1);
    }
    // Every moved upstream must be an absorption candidate, or a document with
    // several upstreams drifts out of its own baseline entry when the
    // newest-moved path changes.
    const short = j.findings.filter((f) => (f.fingerprints || []).length !== f.movedUpstream.length);
    if (short.length) {
      console.error("fingerprint/movedUpstream mismatch: " + short[0].file);
      process.exit(1);
    }
  ' "${json_out}"; then
  fail "docs-owed messages must not embed dates (baseline fingerprints must be stable)"
else
  pass "docs-owed finding messages carry no date"
fi

# Case D: an unreadable document is a tooling failure, not a clean corpus. Only
# a metadata ParseError may be absorbed as "no governance metadata"; an I/O
# error means the surface never read what it is about to report on.
echo "case D: docs-owed exits 2 when it cannot read the corpus"
unreadable="${repo_root}/docs/testing/benchmark-results.md"
if [[ -r "${unreadable}" ]]; then
  perms="$(stat -c '%a' "${unreadable}")"
  chmod 000 "${unreadable}"
  set +e
  (cd "${repo_root}" && node "${owed_script}" --no-baseline >/dev/null 2>&1)
  unread_status=$?
  set -e
  chmod "${perms}" "${unreadable}"
  if [[ "${unread_status}" -eq 2 ]]; then
    pass "unreadable governed doc exits 2 (tooling), not 0 (clean)"
  else
    fail "unreadable governed doc expected exit 2; got ${unread_status}"
  fi
else
  pass "skipped: fixture doc not readable to begin with"
fi

# Case E: the ADR-119 D2 granularity split (DOCFRESH-002).
#
# A directory-only upstream must stay advisory no matter how far the review date
# has slipped — that is the whole point of the split, and it is what stops the
# gate firing on `crates/anvil-cli`. Built as a purpose-made fixture repo rather
# than asserted against the live corpus, so the case keeps its meaning as the
# backlog changes.
echo "case E: directory upstreams stay advisory, file upstreams gate"
d2_root="${tmp_root}/d2"
mkdir -p "${d2_root}/docs/guides" "${d2_root}/crates/widget/src"
(
  cd "${d2_root}"
  git init -q .
  git config user.email t@example.com
  git config user.name t
  cat >docs/guides/dir-only.md <<'DOC'
# Directory Only

| Type  | Authority     | Owner | Status | Freshness                              |
| ----- | ------------- | ----- | ------ | -------------------------------------- |
| Guide | Authoritative | TEST  | Live   | Last reviewed 2001-01-01 against `crates/widget` |

| Upstream      | Downstream |
| ------------- | ---------- |
| `crates/widget` | none    |

Body.
DOC
  cat >docs/guides/file-level.md <<'DOC'
# File Level

| Type  | Authority     | Owner | Status | Freshness                                             |
| ----- | ------------- | ----- | ------ | ----------------------------------------------------- |
| Guide | Authoritative | TEST  | Live   | Last reviewed 2001-01-01 against `crates/widget/src/thing.rs` |

| Upstream              | Downstream |
| --------------------- | ---------- |
| `crates/widget/src/thing.rs` | none |

Body.
DOC
  echo "pub const A: u8 = 1;" >crates/widget/src/thing.rs
  git add -A
  git commit -qm "fixture: docs and source"
) >/dev/null 2>&1

d2_json="${tmp_root}/d2.json"
if ! (cd "${repo_root}" && node "${owed_script}" --root "${d2_root}" --no-baseline --json >"${d2_json}" 2>/dev/null); then
  fail "docs-owed did not run against the D2 fixture"
elif ! node -e '
    const j = JSON.parse(require("node:fs").readFileSync(process.argv[1], "utf8"));
    const dir = j.findings.find((f) => f.file.endsWith("dir-only.md"));
    const file = j.findings.find((f) => f.file.endsWith("file-level.md"));
    if (!dir) { console.error("directory-upstream fixture produced no finding at all"); process.exit(1); }
    if (!file) { console.error("file-upstream fixture produced no finding"); process.exit(1); }
    if (dir.severity !== "WARN" || dir.posture !== "advisory-granularity") {
      console.error("directory upstream must be advisory; got " + dir.severity + "/" + dir.posture);
      process.exit(1);
    }
    if (file.severity !== "ERROR" || file.posture !== "gating") {
      console.error("file upstream must gate; got " + file.severity + "/" + file.posture);
      process.exit(1);
    }
  ' "${d2_json}"; then
  fail "D2 granularity split not honoured"
else
  pass "directory upstream advisory (WARN), file upstream gating (ERROR), both reported"
fi

# Case F: an advisory-only corpus must not trip the gate. A directory upstream
# that has moved is real staleness, but it can never turn the check red.
echo "case F: advisory-only findings never fail the gate"
rm -f "${d2_root}/docs/guides/file-level.md"
(cd "${d2_root}" && git add -A && git commit -qm "drop the file-level doc") >/dev/null 2>&1
set +e
(cd "${repo_root}" && node "${owed_script}" --root "${d2_root}" --no-baseline --fail-on-owed >/dev/null 2>&1)
d2_status=$?
set -e
if [[ "${d2_status}" -eq 0 ]]; then
  pass "advisory-only corpus exits 0 under --fail-on-owed"
else
  fail "advisory-only corpus must not fail the gate; got ${d2_status}"
fi

# Case G: diff-mode glob matching agrees with git's own pathspec.
#
# Corpus mode asks git (`git log -- <glob>`); diff mode matches changed paths in
# JS. If those disagree, a glob upstream is reported in one mode and silently
# missed in the other. Rather than assert a reading of git's globbing, this
# derives the expectation from git itself: whatever `git ls-files -- <glob>`
# matches is what the JS matcher must match.
#
# The non-obvious part, which the first implementation got wrong: in git's
# DEFAULT pathspec `*` crosses `/`. Segment-aware globbing is `:(glob)` magic,
# which this surface does not use.
echo "case G: diff-mode glob matching agrees with git pathspec"
g_root="${tmp_root}/glob"
mkdir -p "${g_root}/scripts/release/_test"
(
  cd "${g_root}"
  git init -q .
  git config user.email t@example.com
  git config user.name t
  touch scripts/release/prepare.sh scripts/release/_test/prepare.test.sh
  git add -A
  git commit -qm "fixture: release scripts"
) >/dev/null 2>&1

# Both sides go through the same `sort`: JS and the shell disagree on where `_`
# falls, and an ordering difference is not a matching difference.
git_matches="$(cd "${g_root}" && git ls-files -- 'scripts/release/*' | sort | tr '\n' ' ')"
all_files="$(cd "${g_root}" && git ls-files | tr '\n' ' ')"
js_matches="$(node -e '
  // Same construction as globPathspec() in check-docs-owed.mjs.
  const re = new RegExp("^" + "scripts/release/*".replace(/[.+^${}()|[\]\\]/g, "\\$&").replace(/\*+/g, ".*") + "$");
  const files = process.argv[1].trim().split(/\s+/).filter(Boolean);
  console.log(files.filter((p) => re.test(p)).join("\n"));
' "${all_files}" | sort | tr '\n' ' ')"
if [[ "${git_matches}" == "${js_matches}" ]]; then
  pass "glob matcher agrees with git ls-files, nested paths included"
else
  fail "glob matcher disagrees with git; git=[${git_matches}] js=[${js_matches}]"
fi

# Case H (DOCFRESH-007): ANVIL_DOCS_VERSION must match the newest public
# changelog heading. A fixture mismatch fails; a fixture match and the live
# repository both pass.
echo "case H: docs version pin matches newest changelog heading"
pin_script="${script_dir}/check-docs-version-pin.mjs"
pin_root="${tmp_root}/docs-version-pin"
mkdir -p "${pin_root}/.github/workflows" "${pin_root}/docs/public/anvil/releases"
cat >"${pin_root}/.github/workflows/ci.yml" <<'EOF'
      - name: Install the public-doc command truth binary
        env:
          ANVIL_DOCS_VERSION: 0.9.1-beta
EOF
cat >"${pin_root}/docs/public/anvil/releases/changelog.md" <<'EOF'
## 0.9.4-beta — 10 August 2026 — Clearer install advice

## 0.9.1-beta — 2 August 2026 — Daily Path Polish
EOF
set +e
out="$(node "${pin_script}" --root "${pin_root}" 2>&1)"
status=$?
set -e
if [[ "${status}" -ne 0 ]] \
  && echo "${out}" | grep -q "ANVIL_DOCS_VERSION is 0.9.1-beta" \
  && echo "${out}" | grep -q "newest changelog heading is 0.9.4-beta"; then
  pass "mismatch between pin and newest heading fails"
else
  fail "expected mismatch to fail (status ${status}); got: ${out}"
fi

cat >"${pin_root}/.github/workflows/ci.yml" <<'EOF'
      - name: Install the public-doc command truth binary
        env:
          ANVIL_DOCS_VERSION: 0.9.4-beta
EOF
set +e
out="$(node "${pin_script}" --root "${pin_root}" 2>&1)"
status=$?
set -e
if [[ "${status}" -eq 0 ]] && echo "${out}" | grep -q "0.9.4-beta matches newest changelog heading"; then
  pass "matching pin and newest heading pass"
else
  fail "expected match to pass (status ${status}); got: ${out}"
fi

# Cut window: newest heading is untagged; pin may stay on previous published release.
cat >"${pin_root}/.github/workflows/ci.yml" <<'EOF'
      - name: Install the public-doc command truth binary
        env:
          ANVIL_DOCS_VERSION: 0.9.4-beta
EOF
cat >"${pin_root}/docs/public/anvil/releases/changelog.md" <<'EOF'
## 0.9.5-beta — 16 August 2026 — MCP live-heal

## 0.9.4-beta — 10 August 2026 — Clearer install advice
EOF
git -C "${pin_root}" init -q
git -C "${pin_root}" config user.email "docs-check@example.com"
git -C "${pin_root}" config user.name "docs-check"
git -C "${pin_root}" add .github docs
git -C "${pin_root}" commit -q -m "fixture"
git -C "${pin_root}" tag v0.9.4-beta
set +e
out="$(node "${pin_script}" --root "${pin_root}" 2>&1)"
status=$?
set -e
if [[ "${status}" -eq 0 ]] && echo "${out}" | grep -q "0.9.4-beta matches newest published changelog heading"; then
  pass "cut-window pin may stay on previous published release"
else
  fail "expected cut-window pin to pass (status ${status}); got: ${out}"
fi

set +e
out="$(cd "${repo_root}" && node "${pin_script}" 2>&1)"
status=$?
set -e
if [[ "${status}" -eq 0 ]]; then
  pass "live pin matches expected published changelog heading"
else
  fail "live pin disagrees with changelog; got: ${out}"
fi

# CLAWFIX-001: a baseline entry suppresses only one concrete tag finding.
echo "case I: tag baselines are consumed one-for-one"
tag_root="${tmp_root}/tag-baseline"
mkdir -p "${tag_root}/docs/governance" "${tag_root}/plans/modules"
cat >"${tag_root}/docs/governance/tags-catalogue.md" <<'EOF'
# Tags

## Catalogue

| Tag | Meaning |
| --- | --- |
| `approved` | Fixture tag |
EOF
cat >"${tag_root}/plans/modules/example.aps.md" <<'EOF'
# Example

- **Tags:** Bad
- **Tags:** Bad
EOF
cat >"${tag_root}/docs/governance/docs-check.baseline.json" <<'EOF'
{
  "tags": {
    "plans/modules/example.aps.md": [
      "malformed tag \"Bad\" (expected lowercase kebab-case, e.g. \"agent\" or \"cross-platform\")"
    ]
  }
}
EOF
set +e
tag_json="$(node "${tags_script}" --root "${tag_root}" --json 2>/dev/null)"
status=$?
set -e
if [[ "${status}" -eq 1 ]] && node -e '
  const report = JSON.parse(process.argv[1]);
  process.exit(report.summary.errors === 1 && report.summary.warnings === 1 ? 0 : 1);
' "${tag_json}"; then
  pass "one baseline entry leaves the duplicate tag violation unsuppressed"
else
  fail "duplicate tag violation was over-suppressed (status ${status}): ${tag_json}"
fi

# CLAWFIX-001: an unreadable tracked path means the corpus could not be checked.
echo "case J: retired-claims fails tooling on unreadable tracked files"
unreadable_root="${tmp_root}/retired-unreadable"
mkdir -p "${unreadable_root}"
git -C "${unreadable_root}" init -q
git -C "${unreadable_root}" config user.email "docs-check@example.com"
git -C "${unreadable_root}" config user.name "docs-check"
printf '%s\n' "tracked then removed" >"${unreadable_root}/gone.txt"
git -C "${unreadable_root}" add gone.txt
rm "${unreadable_root}/gone.txt"
set +e
out="$(node "${script_dir}/check-retired-claims.mjs" --root "${unreadable_root}" 2>&1)"
status=$?
set -e
if [[ "${status}" -eq 2 ]] && echo "${out}" | grep -q "gone.txt"; then
  pass "unreadable tracked file exits 2 and names the path"
else
  fail "unreadable tracked file should be tooling failure (status ${status}): ${out}"
fi

# CLAWFIX-001: a survivor allowance identifies its local context, not a count.
echo "case K: retired-claim survivor fingerprint rejects an in-file move"
retired_root="${tmp_root}/retired-fingerprint"
checker_root="${tmp_root}/retired-checker"
mkdir -p "${retired_root}" "${checker_root}"
cp "${script_dir}/check-retired-claims.mjs" "${checker_root}/check-retired-claims.mjs"
fingerprint="$(node --input-type=module -e '
  import { createHash } from "node:crypto";
  const material = ["claim.txt", "before original", "retired fixture phrase", "after original"].join("\0");
  process.stdout.write(createHash("sha256").update(material).digest("hex"));
')"
cat >"${checker_root}/retired-claims.mjs" <<EOF
export const RETIRED_CLAIMS = [{
  phrase: 'retired fixture phrase',
  retiredBy: 'CLAWFIX-001',
  baseline: [{
    path: 'claim.txt',
    occurrences: 1,
    fingerprints: ['${fingerprint}'],
    owner: 'CLAWFIX-001'
  }]
}];
export const EXCLUDED_PREFIXES = [];
export const EXCLUDED_FILES = [];
export const EXCLUDED_EXACT_BASENAMES = [];
export const EXCLUDED_EXTENSIONS = [];
export const LINE_MARKER = 'retired-claim-ok:';
EOF
git -C "${retired_root}" init -q
git -C "${retired_root}" config user.email "docs-check@example.com"
git -C "${retired_root}" config user.name "docs-check"
cat >"${retired_root}/claim.txt" <<'EOF'
before original
retired fixture phrase
after original
EOF
git -C "${retired_root}" add claim.txt
set +e
node "${checker_root}/check-retired-claims.mjs" --root "${retired_root}" >/dev/null 2>&1
initial_status=$?
set -e
cat >"${retired_root}/claim.txt" <<'EOF'
before original
replacement text
after original
new section
retired fixture phrase
tail
EOF
set +e
out="$(node "${checker_root}/check-retired-claims.mjs" --root "${retired_root}" 2>&1)"
moved_status=$?
set -e
if [[ "${initial_status}" -eq 0 && "${moved_status}" -eq 1 ]]   && echo "${out}" | grep -q "fingerprint"; then
  pass "matching survivor passes and moved survivor fails"
else
  fail "survivor identity was count-only (initial ${initial_status}, moved ${moved_status}): ${out}"
fi

# CLAWFIX-001: reject an escaping glob before asking globby to expand it.
echo "case L: asbuilt-paths rejects repository-escaping globs"
asbuilt_root="${tmp_root}/asbuilt-containment"
outside_root="${tmp_root}/outside"
mkdir -p "${asbuilt_root}/docs/guides" "${outside_root}"
printf '%s\n' "outside" >"${outside_root}/match.txt"
cat >"${asbuilt_root}/docs/guides/example.md" <<'EOF'
# Example guide

| Type | Authority | Owner | Status | Freshness |
| --- | --- | --- | --- | --- |
| Guide | Authoritative | Test | Live | Last reviewed 2026-08-18 |

| Upstream | Downstream |
| --- | --- |
| `docs/../../outside/**` | Fixture |
EOF
set +e
out="$(node "${asbuilt_script}" --root "${asbuilt_root}" --no-baseline 2>&1)"
status=$?
set -e
if [[ "${status}" -eq 1 ]] && echo "${out}" | grep -q "escapes repository root"; then
  pass "escaping glob is rejected before expansion"
else
  fail "escaping glob should fail containment (status ${status}): ${out}"
fi

# CLAWFIX-001 Council advisory: glob expansion must not traverse a repository
# symlink into a host directory outside the checkout.
echo "case M: asbuilt-paths does not follow symlinked glob roots"
symlink_root="${tmp_root}/asbuilt-symlink-containment"
mkdir -p "${symlink_root}/docs/guides"
ln -s "${outside_root}" "${symlink_root}/docs/linked"
cat >"${symlink_root}/docs/guides/example.md" <<'EOF'
# Example guide

| Type | Authority | Owner | Status | Freshness |
| --- | --- | --- | --- | --- |
| Guide | Authoritative | Test | Live | Last reviewed 2026-08-18 |

| Upstream | Downstream |
| --- | --- |
| `docs/linked/**` | Fixture |
EOF
set +e
out="$(node "${asbuilt_script}" --root "${symlink_root}" --no-baseline 2>&1)"
status=$?
set -e
if [[ "${status}" -eq 1 ]] && echo "${out}" | grep -q "glob traverses symlink"; then
  pass "symlinked glob root is not traversed"
else
  fail "symlinked glob escaped containment (status ${status}): ${out}"
fi

# CLAWFIX-001 Council advisory: static source references must not use a
# repository symlink as a host-path existence oracle.
echo "case N: asbuilt-paths rejects static paths through repository symlinks"
cat >"${symlink_root}/docs/guides/example.md" <<'EOF'
# Example guide

| Type | Authority | Owner | Status | Freshness |
| --- | --- | --- | --- | --- |
| Guide | Authoritative | Test | Live | Last reviewed 2026-08-18 |

| Upstream | Downstream |
| --- | --- |
| `docs/linked/match.txt` | Fixture |
EOF
set +e
out="$(node "${asbuilt_script}" --root "${symlink_root}" --no-baseline 2>&1)"
status=$?
set -e
if [[ "${status}" -eq 1 ]] && echo "${out}" | grep -q "path traverses symlink"; then
  pass "static path through symlink is rejected"
else
  fail "static path exposed host existence (status ${status}): ${out}"
fi

# CLAWFIX-001 adversarial review: a missing component cancelled by `..` must
# not stop symlink inspection before the lexically resolved target.
echo "case O: asbuilt-paths normalises static paths before symlink inspection"
cat >"${symlink_root}/docs/guides/example.md" <<'EOF'
# Example guide

| Type | Authority | Owner | Status | Freshness |
| --- | --- | --- | --- | --- |
| Guide | Authoritative | Test | Live | Last reviewed 2026-08-18 |

| Upstream | Downstream |
| --- | --- |
| `docs/missing/../linked/match.txt` | Fixture |
EOF
set +e
out="$(node "${asbuilt_script}" --root "${symlink_root}" --no-baseline 2>&1)"
status=$?
set -e
if [[ "${status}" -eq 1 ]] && echo "${out}" | grep -q "path traverses symlink"; then
  pass "lexically cancelled prefix cannot bypass symlink rejection"
else
  fail "cancelled prefix bypassed symlink rejection (status ${status}): ${out}"
fi


if [[ "${failures}" -gt 0 ]]; then
  echo "${failures} test case(s) failed"
  exit 1
fi
echo "all cases passed"
