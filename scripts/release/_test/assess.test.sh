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
  mkdir -p "$repo/plans/modules" "$repo/crates/anvil-cli/src"
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
printf '%s\n' 'RELORCH-003 assessment text' >"$success_repo/plans/modules/release-orchestration.aps.md"
git -C "$success_repo" add crates/anvil-cli/src/main.rs plans/modules/release-orchestration.aps.md
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

echo "assess.test.sh: ok"
