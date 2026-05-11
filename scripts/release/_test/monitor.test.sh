#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"; HARNESS="$ROOT/scripts/release/_test/harness.sh"; MONITOR="$ROOT/scripts/release/monitor.sh"
tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
pass="$tmp/pass.json"; fail="$tmp/fail.json"
printf '%s\n' '{"status":"completed","conclusion":"success","url":"https://github.com/eddacraft/anvil-001/actions/runs/1"}' >"$pass"
printf '%s\n' '{"status":"completed","conclusion":"failure","url":"https://github.com/eddacraft/anvil-001/actions/runs/2","failedJob":"dist","logUrl":"https://github.com/eddacraft/anvil-001/actions/runs/2/job/3"}' >"$fail"
bash "$HARNESS" run-contract --name monitor-pass --expected-exit 0 --expected-command monitor -- bash -c 'ANVIL_RELEASE_TEST_MODE=monitor-fake-run ANVIL_RELEASE_MONITOR_FAKE_RUN_FILE="$1" bash "$2" --json --version v0.7.0-beta' _ "$pass" "$MONITOR"
bash "$HARNESS" run-contract --name monitor-fail --expected-exit 1 --expected-command monitor -- bash -c 'ANVIL_RELEASE_TEST_MODE=monitor-fake-run ANVIL_RELEASE_MONITOR_FAKE_RUN_FILE="$1" bash "$2" --json --version v0.7.0-beta' _ "$fail" "$MONITOR"
bash "$HARNESS" run-contract --name monitor-run-url-evidence --expected-exit 1 --expected-command monitor -- bash "$MONITOR" --json --version v0.7.0-beta --run-url https://github.com/eddacraft/anvil-001/actions/runs/123
bash "$HARNESS" run-contract --name monitor-invalid --expected-exit 129 --expected-command monitor -- bash "$MONITOR" --json --unknown
echo "monitor.test.sh: ok"
