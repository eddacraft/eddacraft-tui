#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"; HARNESS="$ROOT/scripts/release/_test/harness.sh"; TAG="$ROOT/scripts/release/tag.sh"
tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
repo="$tmp/tag-repo"; mkdir -p "$repo"; git -C "$repo" init -q; git -C "$repo" config user.email relorch@example.invalid; git -C "$repo" config user.name 'RELORCH Test'; printf x >"$repo/file"; git -C "$repo" add file; git -C "$repo" commit -q -m init
sha="$(git -C "$repo" rev-parse HEAD)"; remote="$tmp/tags.json"

bash "$HARNESS" run-contract --name tag-pre-push --expected-exit 0 --expected-command tag -- bash -c 'cd "$1" && ANVIL_RELEASE_TEST_MODE=tag-fake-remote ANVIL_RELEASE_TAG_FAKE_REMOTE_FILE="$4" bash "$2" --json --version v0.7.0-beta --source-sha "$3"' _ "$repo" "$TAG" "$sha" "$remote"
bash "$HARNESS" run-contract --name tag-refuses-repush --expected-exit 1 --expected-command tag -- bash -c 'cd "$1" && ANVIL_RELEASE_TEST_MODE=tag-fake-remote ANVIL_RELEASE_TAG_FAKE_REMOTE_FILE="$4" bash "$2" --json --version v0.7.0-beta --source-sha "$3"' _ "$repo" "$TAG" "$sha" "$remote"
bash "$HARNESS" run-contract --name tag-recovers-pushed --expected-exit 0 --expected-command tag -- bash -c 'cd "$1" && ANVIL_RELEASE_TEST_MODE=tag-fake-remote ANVIL_RELEASE_TAG_FAKE_REMOTE_FILE="$4" bash "$2" --json --recover --version v0.7.0-beta --source-sha "$3"' _ "$repo" "$TAG" "$sha" "$remote"

other_sha="$(git -C "$repo" commit --allow-empty -q -m other && git -C "$repo" rev-parse HEAD)"
bash "$HARNESS" run-contract --name tag-remote-conflict --expected-exit 1 --expected-command tag -- bash -c 'cd "$1" && ANVIL_RELEASE_TEST_MODE=tag-fake-remote ANVIL_RELEASE_TAG_FAKE_REMOTE_FILE="$4" bash "$2" --json --version v0.7.0-beta --source-sha "$3"' _ "$repo" "$TAG" "$other_sha" "$remote"
bash "$HARNESS" run-contract --name tag-invalid --expected-exit 129 --expected-command tag -- bash "$TAG" --json --unknown

live_remote="$tmp/live-origin.git"; live_repo="$tmp/live-repo"
git init -q --bare "$live_remote"
git init -q -b main "$live_repo"
git -C "$live_repo" config user.email relorch@example.invalid
git -C "$live_repo" config user.name 'RELORCH Test'
printf x >"$live_repo/file"; git -C "$live_repo" add file; git -C "$live_repo" commit -q -m init
git -C "$live_repo" remote add origin "$live_remote"; git -C "$live_repo" push -q -u origin main
live_sha="$(git -C "$live_repo" rev-parse HEAD)"; readiness="$tmp/readiness.json"
node - "$readiness" "$live_sha" <<'NODE'
const fs = require('node:fs');
const [path, headSha] = process.argv.slice(2);
fs.writeFileSync(path, JSON.stringify({ runs: [{ headSha, conclusion: 'success' }] }) + '\n');
NODE
bash "$HARNESS" run-contract --name tag-live-push --expected-exit 0 --expected-command tag -- bash -c 'cd "$1" && ANVIL_RELEASE_TEST_MODE=tag-fake-readiness ANVIL_RELEASE_TAG_FAKE_READINESS_FILE="$5" bash "$2" --json --repo "$4" --version v0.8.0-beta --source-sha "$3"' _ "$live_repo" "$TAG" "$live_sha" "$live_remote" "$readiness"
bash "$HARNESS" run-contract --name tag-live-refuses-repush --expected-exit 1 --expected-command tag -- bash -c 'cd "$1" && ANVIL_RELEASE_TEST_MODE=tag-fake-readiness ANVIL_RELEASE_TAG_FAKE_READINESS_FILE="$5" bash "$2" --json --repo "$4" --version v0.8.0-beta --source-sha "$3"' _ "$live_repo" "$TAG" "$live_sha" "$live_remote" "$readiness"
bash "$HARNESS" run-contract --name tag-live-recovers-pushed --expected-exit 0 --expected-command tag -- bash -c 'cd "$1" && ANVIL_RELEASE_TEST_MODE=tag-fake-readiness ANVIL_RELEASE_TAG_FAKE_READINESS_FILE="$5" bash "$2" --json --recover --repo "$4" --version v0.8.0-beta --source-sha "$3"' _ "$live_repo" "$TAG" "$live_sha" "$live_remote" "$readiness"


# --hotfix waives the main-reachability gate for off-main source SHAs.
hotfix_remote="$tmp/hotfix-origin.git"; hotfix_repo="$tmp/hotfix-repo"
git init -q --bare "$hotfix_remote"
git init -q -b main "$hotfix_repo"
git -C "$hotfix_repo" config user.email relorch@example.invalid
git -C "$hotfix_repo" config user.name 'RELORCH Test'
printf '%s\n' '{"version":"0.6.3-beta"}' >"$hotfix_repo/package.json"
git -C "$hotfix_repo" add package.json
git -C "$hotfix_repo" commit -q -m "main: v0.6.2-beta baseline"
git -C "$hotfix_repo" remote add origin "$hotfix_remote"
git -C "$hotfix_repo" push -q -u origin main
git -C "$hotfix_repo" checkout -q -b hotfix/x main
git -C "$hotfix_repo" commit --allow-empty -q -m "hotfix: tip"
hotfix_sha="$(git -C "$hotfix_repo" rev-parse HEAD)"
hotfix_readiness="$tmp/hotfix-readiness.json"
node - "$hotfix_readiness" "$hotfix_sha" <<'NODE'
const fs = require('node:fs');
const [path, headSha] = process.argv.slice(2);
fs.writeFileSync(path, JSON.stringify({ runs: [{ headSha, conclusion: 'success' }] }) + '\n');
NODE

# Without --hotfix, off-main SHA is refused with stale-source.
rc=0; (cd "$hotfix_repo" && ANVIL_RELEASE_TEST_MODE=tag-fake-readiness ANVIL_RELEASE_TAG_FAKE_READINESS_FILE="$hotfix_readiness" bash "$TAG" --json --repo "$hotfix_remote" --version v0.6.3-beta --source-sha "$hotfix_sha" >"$tmp/hotfix-off-main.json") || rc=$?
[[ "$rc" == "1" ]] || { echo "expected off-main without --hotfix to exit 1, got $rc" >&2; cat "$tmp/hotfix-off-main.json"; exit 1; }
node - "$tmp/hotfix-off-main.json" <<'NODE'
const fs = require('node:fs');
const doc = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
if (!doc.failures.some((failure) => failure.code === 'stale-source')) {
  throw new Error(`expected stale-source failure, got ${JSON.stringify(doc.failures)}`);
}
NODE

# With --hotfix, the same off-main SHA passes the main-reachability gate.
bash "$HARNESS" run-contract --name tag-hotfix-off-main --expected-exit 0 --expected-command tag -- bash -c 'cd "$1" && ANVIL_RELEASE_TEST_MODE=tag-fake-readiness ANVIL_RELEASE_TAG_FAKE_READINESS_FILE="$5" bash "$2" --json --hotfix --repo "$4" --version v0.6.3-beta --source-sha "$3"' _ "$hotfix_repo" "$TAG" "$hotfix_sha" "$hotfix_remote" "$hotfix_readiness"

echo "tag.test.sh: ok"
