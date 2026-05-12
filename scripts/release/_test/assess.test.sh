#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
HARNESS="$ROOT/scripts/release/_test/harness.sh"
ASSESS="$ROOT/scripts/release/assess.sh"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

assert_contains() {
  local haystack="$1"
  local needle="$2"
  if [[ "$haystack" != *"$needle"* ]]; then
    echo "expected output to contain: $needle" >&2
    echo "actual output:" >&2
    echo "$haystack" >&2
    exit 1
  fi
}

init_repo() {
  local repo="$1"
  mkdir -p "$repo/plans/archive/modules" "$repo/crates/anvil-cli/src"
  git -C "$repo" init -q
  git -C "$repo" config user.email relorch@example.invalid
  git -C "$repo" config user.name "RELORCH Test"
  printf '%s\n' '# Fixture' >"$repo/README.md"
  git -C "$repo" add README.md
  git -C "$repo" commit -q -m "chore: initial fixture"
  git -C "$repo" tag v0.6.1-beta
}

success_repo="$tmp/success-repo"
init_repo "$success_repo"
printf '%s\n' 'fn main() {}' >"$success_repo/crates/anvil-cli/src/main.rs"
printf '%s\n' 'RELORCH-003 assessment text' >"$success_repo/plans/archive/modules/release-orchestration.aps.md"
git -C "$success_repo" add crates/anvil-cli/src/main.rs plans/archive/modules/release-orchestration.aps.md
git -C "$success_repo" commit -q -m "feat: implement RELORCH-003 assessment"

bash "$HARNESS" run-contract \
  --name assess-success \
  --expected-exit 0 \
  --expected-command assess \
  -- bash -c 'cd "$1" && bash "$2" --json --base v0.6.1-beta --head HEAD --repo eddacraft/anvil-001' _ "$success_repo" "$ASSESS"

(cd "$success_repo" && bash "$ASSESS" --json --base v0.6.1-beta --head HEAD) >"$tmp/success.json"
node - "$tmp/success.json" "$(git -C "$success_repo" rev-parse HEAD)" <<'NODE'
const fs = require('node:fs');
const [path, expectedSha] = process.argv.slice(2);
const doc = JSON.parse(fs.readFileSync(path, 'utf8'));
const failures = [];
function expect(condition, message) {
  if (!condition) failures.push(message);
}
expect(doc.status === 'success', `expected success, got ${doc.status}`);
expect(doc.data.candidateVersion === 'v0.7.0-beta', `unexpected candidateVersion ${doc.data.candidateVersion}`);
expect(doc.data.releaseType === 'beta', `unexpected releaseType ${doc.data.releaseType}`);
expect(doc.data.recommendedStrategy === 'direct', `unexpected strategy ${doc.data.recommendedStrategy}`);
expect(doc.data.previousTag === 'v0.6.1-beta', `unexpected previousTag ${doc.data.previousTag}`);
expect(doc.data.sourceSha === expectedSha, `unexpected sourceSha ${doc.data.sourceSha}`);
expect(doc.data.changedPaths.includes('crates/anvil-cli/src/main.rs'), 'missing changed source path');
expect(doc.data.apsItems.includes('RELORCH-003'), 'missing APS item');
expect(doc.data.releaseWarranted === true, 'release should be warranted');
if (failures.length > 0) {
  for (const failure of failures) console.error(failure);
  process.exit(1);
}
NODE

noop_repo="$tmp/noop-repo"
init_repo "$noop_repo"
(cd "$noop_repo" && bash "$ASSESS" --json --base HEAD --head HEAD) >"$tmp/noop.json"
bash "$HARNESS" run-contract \
  --name assess-noop \
  --expected-exit 0 \
  --expected-command assess \
  -- bash -c 'cd "$1" && bash "$2" --json --base HEAD --head HEAD' _ "$noop_repo" "$ASSESS"
node - "$tmp/noop.json" <<'NODE'
const fs = require('node:fs');
const doc = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
if (doc.status !== 'noop' || doc.data.releaseWarranted !== false) {
  console.error(`expected noop with releaseWarranted=false, got ${doc.status}`);
  process.exit(1);
}
NODE

invalid_output="$(bash "$ASSESS" --json --unknown 2>/dev/null || true)"
assert_contains "$invalid_output" '"status":"failed"'
invalid_human_output="$(bash "$ASSESS" --unknown 2>&1 >/dev/null || true)"
assert_contains "$invalid_human_output" 'assess: unknown argument: --unknown'
assert_contains "$invalid_human_output" 'Usage: assess.sh'
if bash "$ASSESS" --json --unknown >/dev/null 2>&1; then
  echo "expected invalid argument to exit non-zero" >&2
  exit 1
fi
rc=0
bash "$ASSESS" --json --unknown >/dev/null 2>&1 || rc=$?
if [[ "$rc" != "129" ]]; then
  echo "expected invalid argument exit 129, got $rc" >&2
  exit 1
fi
bash "$HARNESS" run-contract \
  --name assess-invalid-args \
  --expected-exit 129 \
  --expected-command assess \
  -- bash "$ASSESS" --json --unknown

help_output="$(bash "$ASSESS" --help)"
assert_contains "$help_output" 'Usage: assess.sh'

target_repo="$tmp/target-repo"
init_repo "$target_repo"
printf '%s\n' 'target mode' >"$target_repo/target.txt"
git -C "$target_repo" add target.txt
git -C "$target_repo" commit -q -m "feat: target mode RELORCH-003"
target_sha="$(git -C "$target_repo" rev-parse HEAD)"
(cd "$target_repo" && bash "$ASSESS" --json --base v0.6.1-beta --source-sha "$target_sha") >"$tmp/target.json"
node - "$tmp/target.json" "$target_sha" <<'NODE'
const fs = require('node:fs');
const [path, expectedSha] = process.argv.slice(2);
const doc = JSON.parse(fs.readFileSync(path, 'utf8'));
if (doc.mode !== 'target') throw new Error(`expected target mode, got ${doc.mode}`);
if (doc.inputs.sourceSha !== expectedSha) throw new Error(`unexpected input sourceSha ${doc.inputs.sourceSha}`);
if (doc.inputs.head !== null) throw new Error(`expected target mode head input to be null, got ${doc.inputs.head}`);
if (doc.data.sourceSha !== expectedSha) throw new Error(`unexpected data sourceSha ${doc.data.sourceSha}`);
NODE

if bash -c 'cd "$1" && bash "$2" --json --base v0.6.1-beta --source-sha HEAD >/dev/null 2>&1' _ "$target_repo" "$ASSESS"; then
  echo "expected symbolic --source-sha to fail" >&2
  exit 1
fi

target_noop_output="$(cd "$target_repo" && bash "$ASSESS" --base "$target_sha" --source-sha "$target_sha")"
assert_contains "$target_noop_output" "$target_sha has no changed paths"

# PR #(B): version-detection regression. `git describe --tags` returns
# the most recent reachable tag; the cutover work added a marker tag
# `dev-retired-2026-05-11` that previously got picked as `previousTag`
# and short-circuited the version-bump regex (falling back to the
# default `v0.1.0-beta`). The fix narrows the describe to `--match='v*'`.
# Reproduce by tagging the fixture HEAD with a non-version marker and
# verifying `previousTag` still resolves to `v0.6.1-beta`.
marker_repo="$tmp/marker-repo"
init_repo "$marker_repo"
printf '%s\n' 'fn main() {}' >"$marker_repo/crates/anvil-cli/src/main.rs"
# Module file with a real header table so the known-prefix allowlist
# admits RELORCH.
cat >"$marker_repo/plans/archive/modules/release-orchestration.aps.md" <<'MODULE'
# Release orchestration

| ID | Owner | Status | Progress |
| --- | --- | --- | --- |
| RELORCH | — | Complete | 12/12 |

### RELORCH-003: assessment

- **Status:** Complete
MODULE
git -C "$marker_repo" add crates/anvil-cli/src/main.rs plans/archive/modules/release-orchestration.aps.md
git -C "$marker_repo" commit -q -m "feat: RELORCH-003 assessment with HTTP-404 error path and pre-FIX-001 cleanup"
# Tag HEAD with a non-version marker that previously confused
# `git describe --tags --abbrev=0`. The marker uses a date-like
# suffix so it sorts lexicographically AFTER `v0.6.1-beta`.
git -C "$marker_repo" tag "dev-retired-2026-05-11"
(cd "$marker_repo" && bash "$ASSESS" --json --base v0.6.1-beta --head HEAD) >"$tmp/marker.json"
node - "$tmp/marker.json" <<'NODE'
const fs = require('node:fs');
const doc = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const failures = [];
function expect(condition, message) {
  if (!condition) failures.push(message);
}
// Bug 1 regression: previousTag must NOT be the marker tag.
expect(
  doc.data.previousTag === 'v0.6.1-beta',
  `previousTag picked the marker tag instead of v0.6.1-beta: ${doc.data.previousTag}`
);
expect(
  doc.data.candidateVersion !== 'v0.1.0-beta',
  'candidateVersion fell through to the v0.1.0-beta default — `git describe --match=v*` not honoured'
);
// Bug 2 regression: HTTP-404 / pre-FIX-001 in the commit message
// must NOT leak into apsItems; RELORCH-003 must remain.
expect(
  doc.data.apsItems.includes('RELORCH-003'),
  'legitimate APS item RELORCH-003 was dropped by the prefix filter'
);
expect(
  !doc.data.apsItems.some((i) => i.startsWith('HTTP-')),
  `HTTP-* false positive leaked through prefix filter: ${doc.data.apsItems.filter((i) => i.startsWith('HTTP-'))}`
);
expect(
  !doc.data.apsItems.includes('FIX-001'),
  'pre-FIX-001 hyphen-preceded prose matched FIX-001 — negative lookbehind not applied'
);
if (failures.length > 0) {
  for (const failure of failures) console.error(failure);
  process.exit(1);
}
NODE

echo "assess.test.sh: ok"
