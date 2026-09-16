#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"; HARNESS="$ROOT/scripts/release/_test/harness.sh"; VERIFY="$ROOT/scripts/release/verify.sh"
tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
sha=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
version=v0.7.0-beta
pass="$tmp/verify-pass.json"; fail="$tmp/verify-fail.json"
printf '%s\n' '{"checks":[{"name":"private-release","status":"pass"},{"name":"public-release","status":"pass"},{"name":"install-site","status":"pass"}],"releaseRecordUrl":"https://github.com/eddacraft/anvil-001/releases/download/v0.7.0-beta/release-record.json","releaseRecordSha256":"abc123","commsDraft":"Release v0.7.0-beta verified."}' >"$pass"
printf '%s\n' '{"checks":[{"name":"install-site","status":"fail","code":"integrity-failed","url":"https://install.eddacraft.ai"}],"releaseRecordUrl":null,"releaseRecordSha256":null,"commsDraft":null}' >"$fail"
bash "$HARNESS" run-contract --name verify-pass --expected-exit 0 --expected-command verify -- bash -c 'ANVIL_RELEASE_TEST_MODE=verify-fake-report ANVIL_RELEASE_VERIFY_FAKE_REPORT_FILE="$1" bash "$2" --json --version v0.7.0-beta --source-sha "$3"' _ "$pass" "$VERIFY" "$sha"
bash "$HARNESS" run-contract --name verify-fail --expected-exit 1 --expected-command verify -- bash -c 'ANVIL_RELEASE_TEST_MODE=verify-fake-report ANVIL_RELEASE_VERIFY_FAKE_REPORT_FILE="$1" bash "$2" --json --version v0.7.0-beta --source-sha "$3"' _ "$fail" "$VERIFY" "$sha"
bash "$HARNESS" run-contract --name verify-invalid --expected-exit 129 --expected-command verify -- bash "$VERIFY" --json --unknown
bash "$HARNESS" run-contract --name verify-live-fake-guard --expected-exit 129 --expected-command verify -- bash -c 'ANVIL_RELEASE_VERIFY_FAKE_LIVE_FILE="$1" bash "$2" --json --version v0.7.0-beta --source-sha "$3"' _ "$pass" "$VERIFY" "$sha"

write_live_snapshot() {
  local dest="$1"
  node - "$dest" "$version" "$sha" <<'NODE'
const fs = require('node:fs');
const [dest, version, sourceSha] = process.argv.slice(2);
const crate = 'eddacraft-anvil';
const targets = [
  ['aarch64-apple-darwin', 'tar.xz'],
  ['aarch64-pc-windows-msvc', 'zip'],
  ['aarch64-unknown-linux-gnu', 'tar.xz'],
  ['x86_64-apple-darwin', 'tar.xz'],
  ['x86_64-unknown-linux-gnu', 'tar.xz'],
  ['x86_64-pc-windows-msvc', 'zip'],
];
const assets = [];
for (const [triple, ext] of targets) {
  assets.push(`${crate}-${triple}.${ext}`, `${crate}-${triple}.${ext}.sha256`);
}
assets.push(
  `${crate}-installer.sh`,
  `${crate}-installer.sh.minisig`,
  `${crate}-installer.ps1`,
  `${crate}-installer.ps1.minisig`,
  'dist-manifest.json',
  `anvil-${version}-provenance.json`,
  `anvil-${version}-provenance.json.minisig`,
  `release-evidence-${version}.md`,
);
const release = {
  found: true,
  isDraft: false,
  url: `https://github.com/eddacraft/anvil/releases/tag/${version}`,
  assets,
};
const snapshot = {
  privateRelease: { ...release, url: `https://github.com/eddacraft/anvil-001/releases/tag/${version}` },
  publicRelease: release,
  provenance: {
    release_tag: version,
    private_build: {
      commit_sha: sourceSha,
      workflow_run_url: 'https://github.com/eddacraft/anvil-001/actions/runs/1',
    },
  },
  homebrewFormula: `class Anvil < Formula\n  version "${version.replace(/^v/, '')}"\nend\n`,
  evidenceRepo: {
    found: true,
    url: `https://github.com/eddacraft/anvil/blob/main/releases/release-evidence-${version}.md`,
  },
  installSite: { status: 200, url: 'https://install.eddacraft.ai' },
  scoop: { configured: false, state: 'not-configured' },
  winget: { configured: false, state: 'not-configured' },
};
fs.writeFileSync(dest, JSON.stringify(snapshot));
NODE
}

live_pass="$tmp/live-pass.json"
write_live_snapshot "$live_pass"
bash "$HARNESS" run-contract --name verify-live-pass --expected-exit 0 --expected-command verify -- bash -c 'ANVIL_RELEASE_TEST_MODE=verify-live-fake ANVIL_RELEASE_VERIFY_FAKE_LIVE_FILE="$1" bash "$2" --json --version v0.7.0-beta --source-sha "$3"' _ "$live_pass" "$VERIFY" "$sha"

live_sha="$tmp/live-sha.json"
write_live_snapshot "$live_sha"
node - "$live_sha" <<'NODE'
const fs = require('node:fs');
const path = process.argv[2];
const doc = JSON.parse(fs.readFileSync(path, 'utf8'));
doc.provenance.private_build.commit_sha = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
fs.writeFileSync(path, JSON.stringify(doc));
NODE
bash "$HARNESS" run-contract --name verify-live-sha --expected-exit 1 --expected-command verify -- bash -c 'ANVIL_RELEASE_TEST_MODE=verify-live-fake ANVIL_RELEASE_VERIFY_FAKE_LIVE_FILE="$1" bash "$2" --json --version v0.7.0-beta --source-sha "$3"' _ "$live_sha" "$VERIFY" "$sha"

live_site="$tmp/live-site.json"
write_live_snapshot "$live_site"
node - "$live_site" <<'NODE'
const fs = require('node:fs');
const path = process.argv[2];
const doc = JSON.parse(fs.readFileSync(path, 'utf8'));
doc.installSite.status = 503;
fs.writeFileSync(path, JSON.stringify(doc));
NODE
bash "$HARNESS" run-contract --name verify-live-site --expected-exit 1 --expected-command verify -- bash -c 'ANVIL_RELEASE_TEST_MODE=verify-live-fake ANVIL_RELEASE_VERIFY_FAKE_LIVE_FILE="$1" bash "$2" --json --version v0.7.0-beta --source-sha "$3"' _ "$live_site" "$VERIFY" "$sha"

live_assets="$tmp/live-assets.json"
write_live_snapshot "$live_assets"
node - "$live_assets" <<'NODE'
const fs = require('node:fs');
const path = process.argv[2];
const doc = JSON.parse(fs.readFileSync(path, 'utf8'));
doc.publicRelease.assets = doc.publicRelease.assets.filter((name) => !name.includes('installer.sh'));
doc.privateRelease.assets = doc.publicRelease.assets;
fs.writeFileSync(path, JSON.stringify(doc));
NODE
bash "$HARNESS" run-contract --name verify-live-assets --expected-exit 1 --expected-command verify -- bash -c 'ANVIL_RELEASE_TEST_MODE=verify-live-fake ANVIL_RELEASE_VERIFY_FAKE_LIVE_FILE="$1" bash "$2" --json --version v0.7.0-beta --source-sha "$3"' _ "$live_assets" "$VERIFY" "$sha"

echo "verify.test.sh: ok"
