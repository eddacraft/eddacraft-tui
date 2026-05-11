#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"; HARNESS="$ROOT/scripts/release/_test/harness.sh"; VERIFY="$ROOT/scripts/release/verify.sh"
tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
sha=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
pass="$tmp/verify-pass.json"; fail="$tmp/verify-fail.json"
printf '%s\n' '{"checks":[{"name":"private-release","status":"pass"},{"name":"public-release","status":"pass"},{"name":"install-site","status":"pass"}],"releaseRecordUrl":"https://github.com/eddacraft/anvil-001/releases/download/v0.7.0-beta/release-record.json","releaseRecordSha256":"abc123","commsDraft":"Release v0.7.0-beta verified."}' >"$pass"
printf '%s\n' '{"checks":[{"name":"install-site","status":"fail","code":"integrity-failed","url":"https://install.eddacraft.ai"}],"releaseRecordUrl":null,"releaseRecordSha256":null,"commsDraft":null}' >"$fail"
bash "$HARNESS" run-contract --name verify-pass --expected-exit 0 --expected-command verify -- bash -c 'ANVIL_RELEASE_TEST_MODE=verify-fake-report ANVIL_RELEASE_VERIFY_FAKE_REPORT_FILE="$1" bash "$2" --json --version v0.7.0-beta --source-sha "$3"' _ "$pass" "$VERIFY" "$sha"
bash "$HARNESS" run-contract --name verify-fail --expected-exit 1 --expected-command verify -- bash -c 'ANVIL_RELEASE_TEST_MODE=verify-fake-report ANVIL_RELEASE_VERIFY_FAKE_REPORT_FILE="$1" bash "$2" --json --version v0.7.0-beta --source-sha "$3"' _ "$fail" "$VERIFY" "$sha"
bash "$HARNESS" run-contract --name verify-invalid --expected-exit 129 --expected-command verify -- bash "$VERIFY" --json --unknown
echo "verify.test.sh: ok"
