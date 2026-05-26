#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
CHECK=(node "$ROOT/scripts/aps/drift-check.mjs")

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

assert_json_has_code() {
  local json="$1"
  local code="$2"
  printf '%s' "$json" | node -e '
const doc = JSON.parse(require("node:fs").readFileSync(0, "utf8"));
const code = process.argv[1];
if (!doc.advisory || doc.enforcement !== "none") throw new Error("expected warning-mode output");
if (!doc.findings.some((finding) => finding.code === code)) {
  throw new Error(`missing finding code: ${code}\n${JSON.stringify(doc, null, 2)}`);
}
' "$code"
}

# #1438 follow-up: assert the message text on load-bearing findings so
# a silent rename (refactor that changes the human-readable message)
# is caught. The `code` field is the machine-readable surface, but the
# message is what operators read in the CI log — keep both stable.
assert_json_has_message_substring() {
  local json="$1"
  local code="$2"
  local substring="$3"
  printf '%s' "$json" | node -e '
const doc = JSON.parse(require("node:fs").readFileSync(0, "utf8"));
const [code, substring] = process.argv.slice(1);
const finding = doc.findings.find((f) => f.code === code);
if (!finding) {
  throw new Error(`no finding with code: ${code}\n${JSON.stringify(doc, null, 2)}`);
}
if (!finding.message || !finding.message.includes(substring)) {
  throw new Error(`finding ${code} message did not include substring "${substring}"; got: ${finding.message}`);
}
' "$code" "$substring"
}

write_module() {
  local path="$1"
  mkdir -p "$(dirname "$path")"
  # CICD-011 council follow-up: FIX-005b exercises the b-suffix code path
  # (drift-check.mjs headingPattern + apsWorkItemPattern admit `\d{3}[a-z]?`).
  cat > "$path" <<'EOF'
# Fixture Module

| ID | Owner | Status | Progress |
| --- | --- | --- | --- |
| FIX | — | In Progress | 2/5 |

### FIX-001: Complete item

- **Status:** Complete
- **Validation:** `pnpm test`
- **Files:** `src/known.ts`

### FIX-002: Merged item

- **Status:** Merged
- **Validation:** `pnpm test`
- **Files:** `src/merged.ts`

### FIX-003: Shipped item

- **Status:** Released/Shipped
- **Validation:** `pnpm test`
- **Files:** `src/shipped.ts`

### FIX-004: Ready item

- **Status:** Ready
- **Validation:** `pnpm test`
- **Files:** `src/ready.ts`

### FIX-005b: Suffixed item

- **Status:** Ready
- **Validation:** `pnpm test`
- **Files:** `src/suffixed.ts`
EOF
}

mkdir -p "$tmp/plans/modules"
write_module "$tmp/plans/modules/fixture.aps.md"

# #1769: lifecycle fixture exercises the broader done-state set
# (`Done`, `Complete`, `Merged`, `Released/Shipped`) plus the trailing-
# prose forms that real modules already use:
#   - `Complete — merged 2026-04-29 via PR #...` (em-dash separator)
#   - `Done — landed via PR #...` (em-dash separator on Done)
#   - `Merged 2026-05-17 — ...` (date BEFORE em-dash — surfaced by
#     PR #1771 review against `plans/modules/multilayer-protection-v2`)
# Header progress matches the count of done items, so this module must
# NOT produce an `aps-progress-mismatch` finding. A negative test below
# locks that behaviour.
cat > "$tmp/plans/modules/lifecycle.aps.md" <<'EOF'
# Lifecycle Module

| ID | Owner | Status | Progress |
| --- | --- | --- | --- |
| LIFE | — | In Progress | 7/8 |

### LIFE-001: Canonical Done

- **Status:** Done
- **Files:** `src/a.ts`

### LIFE-002: Legacy Complete alias

- **Status:** Complete
- **Validation:** `pnpm test` — validation passed.
- **Files:** `src/b.ts`

### LIFE-003: Merged terminal

- **Status:** Merged
- **Files:** `src/c.ts`

### LIFE-004: Released/Shipped terminal

- **Status:** Released/Shipped
- **Files:** `src/d.ts`

### LIFE-005: Trailing-prose Complete

- **Status:** Complete — merged 2026-04-29 via PR #100 (`feat/LIFE-005`).
- **Validation:** `pnpm test` — validation passed.
- **Files:** `src/e.ts`

### LIFE-006: Trailing-prose Done

- **Status:** Done — landed via PR #101 (`feat/LIFE-006`).
- **Files:** `src/f.ts`

### LIFE-007: Date-then-dash Merged

- **Status:** Merged 2026-05-17 — all umbrella-required sub-tasks landed.
- **Files:** `src/g.ts`

### LIFE-008: Still in progress

- **Status:** In Progress (release-council pending)
- **Files:** `src/h.ts`
EOF

cat > "$tmp/plans/index.aps.md" <<'EOF'
# Fixture Index

| [fixture](./modules/fixture.aps.md) | FIX | 1/4 | Fixture module. |
| [lifecycle](./modules/lifecycle.aps.md) | LIFE | 7/8 | Lifecycle fixture. |
EOF
cat > "$tmp/package.json" <<'EOF'
{"version":"1.2.3"}
EOF

printf '%s\n' 'src/unknown.ts' > "$tmp/changed-files"
cat > "$tmp/candidate.json" <<'EOF'
{
  "lifecycleState": "candidate",
  "version": "v1.2.3",
  "source": {"tag": "v1.2.3"},
  "aps": {"items": []}
}
EOF
candidate_json="$("${CHECK[@]}" --root "$tmp" --changed-files "$tmp/changed-files" --release-record "$tmp/candidate.json" --json)"
assert_json_has_code "$candidate_json" 'aps-progress-mismatch'
assert_json_has_code "$candidate_json" 'aps-index-progress-mismatch'
assert_json_has_code "$candidate_json" 'aps-complete-without-validation-evidence'
assert_json_has_code "$candidate_json" 'changed-file-without-aps-reference'
assert_json_has_code "$candidate_json" 'candidate-missing-merged-aps-item'

# Regression sentinel: shipped-aps-without-release-record MUST NOT fire under
# a `candidate` record. Released/Shipped items reference a past published
# release the candidate record knows nothing about — checking them against
# the candidate's items would always produce false positives. The
# published-only gate landed in #1971.
if printf '%s' "$candidate_json" | node -e '
const doc = JSON.parse(require("node:fs").readFileSync(0, "utf8"));
if (doc.findings.some((f) => f.code === "shipped-aps-without-release-record")) {
  throw new Error("shipped-aps-without-release-record fired under a candidate record");
}
'; then :; else echo "shipped-aps gating regression: fired under candidate record" >&2; exit 1; fi

# #1438 follow-up: lock the message text on load-bearing findings so
# silent renames are caught. `changed-file-without-aps-reference` is
# the one operators most often act on, and `aps-progress-mismatch`'s
# `/N` count is the regression sentinel for the b-suffix fix.
assert_json_has_message_substring "$candidate_json" 'changed-file-without-aps-reference' 'src/unknown.ts'
assert_json_has_message_substring "$candidate_json" 'aps-progress-mismatch' '/5'

# #1769: the LIFE fixture's 5/6 header matches its 5 done items
# (`Done`, `Complete`, `Merged`, `Released/Shipped`, and trailing-prose
# `Complete — merged ...`), so `aps-progress-mismatch` must NOT fire for
# the LIFE module. Regression sentinel for the done-state broadening:
# a regression that drops `Done`/`Merged`/`Released/Shipped` from the
# count would surface here as a spurious LIFE mismatch.
if printf '%s' "$candidate_json" | node -e '
const doc = JSON.parse(require("node:fs").readFileSync(0, "utf8"));
const life = doc.findings.find((f) => f.code === "aps-progress-mismatch" && f.module === "LIFE");
if (life) {
  throw new Error("LIFE module emitted spurious aps-progress-mismatch: " + life.message);
}
'; then :; else echo "done-state broadening regression: LIFE flagged as mismatched" >&2; exit 1; fi

# #1769: lock the canonical aliases individually. Build a fixture per
# state so a regression that drops any one state surfaces with the
# specific module name in the failure. State is passed to the inline
# node script via the `STATE` env var rather than bash-interpolated into
# the single-quoted JS — easier to read and immune to future state
# values containing quotes.
for state in Done Complete Merged 'Released/Shipped'; do
  state_tmp="$(mktemp -d)"
  mkdir -p "$state_tmp/plans/modules"
  state_slug="$(printf '%s' "$state" | tr '[:upper:]/' '[:lower:]-')"
  state_id="$(printf '%s' "$state_slug" | tr -d '-' | tr '[:lower:]' '[:upper:]')"
  cat > "$state_tmp/plans/modules/${state_slug}.aps.md" <<EOF
# ${state} Module

| ID | Owner | Status | Progress |
| --- | --- | --- | --- |
| ${state_id} | — | In Progress | 1/1 |

### ${state_id}-001: Single done item

- **Status:** ${state}
- **Files:** \`src/x.ts\`
EOF
  state_json="$("${CHECK[@]}" --root "$state_tmp" --json)"
  if printf '%s' "$state_json" | STATE="$state" node -e '
const doc = JSON.parse(require("node:fs").readFileSync(0, "utf8"));
const state = process.env.STATE;
const mismatch = doc.findings.find((f) => f.code === "aps-progress-mismatch");
if (mismatch) {
  throw new Error(`expected no mismatch for Status: ${state}, got: ${mismatch.message}`);
}
'; then :; else echo "done-state regression: '${state}' did not count toward progressDone" >&2; rm -rf "$state_tmp"; exit 1; fi
  rm -rf "$state_tmp"
done

cat > "$tmp/published.json" <<'EOF'
{
  "lifecycleState": "published",
  "version": "v1.2.4",
  "source": {"tag": "v1.2.5"},
  "aps": {"items": [{"id":"FIX-003"}]},
  "artifacts": [{"name":"anvil.tar.gz","url":"https://example.test/anvil.tar.gz"}]
}
EOF
published_json="$("${CHECK[@]}" --root "$tmp" --release-record "$tmp/published.json" --json)"
assert_json_has_code "$published_json" 'release-version-tag-mismatch'
assert_json_has_code "$published_json" 'package-version-tag-mismatch'
assert_json_has_code "$published_json" 'release-artifact-missing-integrity'
# #1971: shipped-aps-without-release-record fires under a `published` record
# for Released/Shipped items not listed in the record's `aps.items`. The
# fixture's published.json lists FIX-003 but not LIFE-004, so LIFE-004's
# Released/Shipped status should be flagged here.
assert_json_has_code "$published_json" 'shipped-aps-without-release-record'
assert_json_has_message_substring "$published_json" 'shipped-aps-without-release-record' 'LIFE-004'

# ── CICD-011: PR metadata drift ──────────────────────────────────
# A PR with no APS reference and no `Unplanned-work:` opt-out flags
# the `pr-missing-aps-reference` warning.
pr_missing_json="$("${CHECK[@]}" --root "$tmp" --pr-title 'chore: bump deps' --json)"
assert_json_has_code "$pr_missing_json" 'pr-missing-aps-reference'

# A PR title that names an APS work item resolves the warning. The
# fixture module declares FIX-001..FIX-005b, so referencing FIX-001 is
# valid; no `pr-missing-aps-reference` or `pr-aps-reference-unknown`
# should appear.
pr_ok_json="$("${CHECK[@]}" --root "$tmp" --pr-title 'feat(fix): cover FIX-001 follow-ups' --json)"
if printf '%s' "$pr_ok_json" | node -e '
const doc = JSON.parse(require("node:fs").readFileSync(0, "utf8"));
if (doc.findings.some((f) => f.code === "pr-missing-aps-reference" || f.code === "pr-aps-reference-unknown")) {
  throw new Error("did not expect PR-metadata drift on a title that references a known APS item");
}
'; then :; else echo "pr-metadata false-positive on known APS reference" >&2; exit 1; fi

# CICD-011 council follow-up: PR referencing the b-suffix item must
# resolve as known — guards against drift-check regexes losing the
# `[a-z]?` admission after `\d{3}`. The fixture's FIX-005b is the
# extractable analogue of real-world RCLI3-016b / RCLI3-017b.
pr_suffix_json="$("${CHECK[@]}" --root "$tmp" --pr-title 'feat: wrap FIX-005b case' --json)"
if printf '%s' "$pr_suffix_json" | node -e '
const doc = JSON.parse(require("node:fs").readFileSync(0, "utf8"));
if (doc.findings.some((f) => f.code === "pr-missing-aps-reference" || f.code === "pr-aps-reference-unknown")) {
  throw new Error("did not expect PR-metadata drift on a title that references a known b-suffix APS item");
}
'; then :; else echo "pr-metadata false-positive on b-suffix APS reference" >&2; exit 1; fi

# extractModule must count the b-suffix item: the fixture declares 5
# items (FIX-001..005b). The progress mismatch message must mention "/5"
# so a regex regression that drops the b-suffix item surfaces here.
if ! printf '%s' "$candidate_json" | node -e '
const doc = JSON.parse(require("node:fs").readFileSync(0, "utf8"));
const finding = doc.findings.find((f) => f.code === "aps-progress-mismatch");
if (!finding) throw new Error("expected aps-progress-mismatch finding");
if (!/\/5/.test(finding.message)) {
  throw new Error("aps-progress-mismatch message did not reflect 5 fixture items: " + finding.message);
}
'; then echo "extractModule did not count the b-suffix item" >&2; exit 1; fi

# A PR that references a KNOWN prefix with an unknown ID
# (e.g. `FIX-999` against the `FIX` fixture module) flags
# `pr-aps-reference-unknown` — the operator typed an APS-shaped reference
# that doesn't resolve to any declared item under that prefix.
pr_unknown_json="$("${CHECK[@]}" --root "$tmp" --pr-title 'feat: FIX-999 something else' --json)"
assert_json_has_code "$pr_unknown_json" 'pr-aps-reference-unknown'

# #1438 follow-up: prefix-unknown tokens (e.g. `NOPE-999`,
# `HTTP-404`, `EC2-123` against a fixture that only declares `FIX`)
# are treated as non-APS noise and fall through to
# `pr-missing-aps-reference` rather than `pr-aps-reference-unknown`.
# This stops common phrases like "fix HTTP-404 errors" from
# misleading operators with a fake unknown-reference warning.
pr_noise_json="$("${CHECK[@]}" --root "$tmp" --pr-title 'feat: fix HTTP-404 errors' --json)"
assert_json_has_code "$pr_noise_json" 'pr-missing-aps-reference'
if printf '%s' "$pr_noise_json" | node -e '
const doc = JSON.parse(require("node:fs").readFileSync(0, "utf8"));
if (doc.findings.some((f) => f.code === "pr-aps-reference-unknown")) {
  throw new Error("non-APS token with unknown prefix must not fire pr-aps-reference-unknown");
}
'; then :; else echo "noise-token classification broke: HTTP-404 mislabeled as unknown-reference" >&2; exit 1; fi

# #1438 follow-up: `pre-FIX-001` must not match `FIX-001` via the
# trailing word-boundary on `pre-`. Negative lookbehind `(?<![\w-])`
# rejects the hyphen-preceded form. A PR title with only this string
# should be classified as missing (no real APS reference).
pr_prehyphen_json="$("${CHECK[@]}" --root "$tmp" --pr-title 'docs: ref pre-FIX-001 inline' --json)"
assert_json_has_code "$pr_prehyphen_json" 'pr-missing-aps-reference'
if printf '%s' "$pr_prehyphen_json" | node -e '
const doc = JSON.parse(require("node:fs").readFileSync(0, "utf8"));
if (doc.findings.some((f) => f.code === "pr-aps-reference-unknown")) {
  throw new Error("hyphen-preceded prose like pre-FIX-001 must not match the APS pattern");
}
'; then :; else echo "negative lookbehind regression: pre-FIX-001 leaked through" >&2; exit 1; fi

# PR #1439 council follow-up: scan ALL APS-shaped tokens, not just the
# first match. A PR like `addresses HTTP-404 in FIX-001 path` mentions
# a non-APS token first and a known APS item second; the policy is
# "reference at least one APS work item anywhere", so this must be
# silent (no missing-reference, no unknown-reference). Previously a
# `.match()` keyed on the first token only and false-positived as
# unknown-reference for HTTP-404.
pr_mixed_json="$("${CHECK[@]}" --root "$tmp" --pr-title 'addresses HTTP-404 in FIX-001 path' --json)"
if printf '%s' "$pr_mixed_json" | node -e '
const doc = JSON.parse(require("node:fs").readFileSync(0, "utf8"));
if (doc.findings.some((f) => f.code === "pr-missing-aps-reference" || f.code === "pr-aps-reference-unknown")) {
  throw new Error("did not expect PR-metadata drift when a known APS item appears alongside unknown tokens");
}
'; then :; else echo "pr-metadata scan-all-matches missed a known reference next to an unknown token" >&2; exit 1; fi

# A PR with `Unplanned-work:` opt-out in the body suppresses the warning.
# Value must be at least 4 non-whitespace characters (#1438 follow-up).
printf 'Unplanned-work: production hotfix\n' > "$tmp/pr-body.txt"
pr_unplanned_json="$("${CHECK[@]}" --root "$tmp" --pr-title 'fix: prod regression' --pr-body-file "$tmp/pr-body.txt" --json)"
if printf '%s' "$pr_unplanned_json" | node -e '
const doc = JSON.parse(require("node:fs").readFileSync(0, "utf8"));
if (doc.findings.some((f) => f.code === "pr-missing-aps-reference")) {
  throw new Error("did not expect pr-missing-aps-reference when Unplanned-work: is declared");
}
'; then :; else echo "pr-metadata false-positive on Unplanned-work opt-out" >&2; exit 1; fi

# #1438 follow-up: trivial opt-out values (< 4 chars) must NOT
# satisfy the opt-out. `Unplanned-work: x` is just dismissive boilerplate;
# require a real justification token like `hotfix` or `incident-1234`.
printf 'Unplanned-work: x\n' > "$tmp/pr-body-trivial.txt"
pr_trivial_optout_json="$("${CHECK[@]}" --root "$tmp" --pr-title 'fix: ad-hoc' --pr-body-file "$tmp/pr-body-trivial.txt" --json)"
assert_json_has_code "$pr_trivial_optout_json" 'pr-missing-aps-reference'

# #1438 follow-up: case-sensitive header — prose like `This is
# unplanned-work: see thread` (lowercase, narrative) must NOT count
# as an opt-out. Only the exact `Unplanned-work:` form (capitalised
# marker, line-anchored) qualifies.
printf 'This is unplanned-work: see linked thread\n' > "$tmp/pr-body-lowercase.txt"
pr_prose_optout_json="$("${CHECK[@]}" --root "$tmp" --pr-title 'docs: thread context' --pr-body-file "$tmp/pr-body-lowercase.txt" --json)"
assert_json_has_code "$pr_prose_optout_json" 'pr-missing-aps-reference'

# PR #1442 follow-up: when emitting `::warning::` workflow commands
# the message body must be URL-encoded for `%`, `\r`, `\n` (and the
# title for `,`, `:`) — the GitHub Actions runner URL-decodes the
# parameter after parsing, so literal `%` characters or stray
# `%XX` sequences in finding messages would corrupt the annotation.
# Synthesise a finding by putting `%` in the changed-file path (which
# lands verbatim in the `changed-file-without-aps-reference`
# message), then assert the emitted annotation URL-encodes it as
# `%25`. We must enable GITHUB_ACTIONS at the env level so the
# annotation branch actually runs.
printf '%s\n' 'src/has-percent-50%-here.ts' > "$tmp/changed-files-pct"
escape_out="$(GITHUB_ACTIONS=true node "$ROOT/scripts/aps/drift-check.mjs" --root "$tmp" --changed-files "$tmp/changed-files-pct" 2>&1 | grep '^::warning' | grep 'has-percent' || true)"
case "$escape_out" in
  *'%25'*) ;;
  *)
    echo "expected ::warning annotation to URL-encode literal % as %25" >&2
    echo "got: $escape_out" >&2
    exit 1
    ;;
esac
case "$escape_out" in
  *' 50% '*)
    echo "::warning annotation must not contain a literal '%' followed by non-hex chars" >&2
    echo "got: $escape_out" >&2
    exit 1
    ;;
esac

# CICD-011 council follow-up: when plans/modules/ is missing or empty,
# the PR-aps-reference-unknown check is degraded — drift-check must
# emit the explicit `pr-aps-check-degraded` advisory rather than
# silently skipping. Tmp root with no plans/modules/ directory exercises
# the empty-knownApsItems path.
degraded_tmp="$(mktemp -d)"
# Replace the existing trap with one that cleans both tmp dirs (a bare
# `trap ... EXIT` would clobber the first trap and leak the original).
trap 'rm -rf "$tmp" "$degraded_tmp"' EXIT
mkdir -p "$degraded_tmp/plans"
cat > "$degraded_tmp/package.json" <<'EOF'
{"version":"1.2.3"}
EOF
degraded_json="$("${CHECK[@]}" --root "$degraded_tmp" --pr-title 'feat: NOPE-999 case' --json)"
assert_json_has_code "$degraded_json" 'pr-aps-check-degraded'

# CICD-011 cycle-2 council: in degraded mode, `pr-missing-aps-reference`
# and `pr-aps-reference-unknown` must short-circuit so the degraded
# advisory stands alone. Without the short-circuit, a PR with no APS
# reference at all (empty index + no reference) would fire BOTH
# `pr-aps-check-degraded` ("check disabled") and
# `pr-missing-aps-reference` ("no reference found") simultaneously,
# giving operators contradictory signal.
degraded_noref_json="$("${CHECK[@]}" --root "$degraded_tmp" --pr-title 'chore: bump deps no aps reference here' --json)"
assert_json_has_code "$degraded_noref_json" 'pr-aps-check-degraded'
if printf '%s' "$degraded_noref_json" | node -e '
const doc = JSON.parse(require("node:fs").readFileSync(0, "utf8"));
if (doc.findings.some((f) => f.code === "pr-missing-aps-reference" || f.code === "pr-aps-reference-unknown")) {
  throw new Error("did not expect pr-missing-aps-reference or pr-aps-reference-unknown when the index is degraded");
}
'; then :; else echo "degraded short-circuit failed: PR-reference checks fired alongside the degraded advisory" >&2; exit 1; fi

echo "drift-check.test.sh: ok"
