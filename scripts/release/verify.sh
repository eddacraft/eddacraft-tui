#!/usr/bin/env bash
set -euo pipefail

COMMAND="verify"; PHASE="verify"; SCHEMA_VERSION="1.0.0"; DEFAULT_REPO="eddacraft/anvil-001"
DEFAULT_PUBLIC_REPO="eddacraft/anvil"
DEFAULT_TAP_REPO="eddacraft/homebrew-tap"
DEFAULT_INSTALL_URL="https://install.eddacraft.ai"
json=false; repo="$DEFAULT_REPO"; public_repo="$DEFAULT_PUBLIC_REPO"; version=""; source_sha=""; mode="target"; started_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
tap_repo="${ANVIL_RELEASE_VERIFY_TAP_REPO:-$DEFAULT_TAP_REPO}"
install_url="${ANVIL_RELEASE_VERIFY_INSTALL_URL:-$DEFAULT_INSTALL_URL}"

usage() {
  cat <<'USAGE'
Usage: verify.sh --version <vX.Y.Z[-suffix]> --source-sha <sha> [--json] [--repo <owner/name>] [--public-repo <owner/name>]

Confirm the tagged cut is published: private and public GitHub releases, cargo-dist
assets, provenance SHA, release-evidence asset and public-repo copy, Homebrew tap
version, and https://install.eddacraft.ai. Scoop and WinGet are recorded as
not-configured unless dist-workspace.toml lists them.
USAGE
}

now() { date -u +%Y-%m-%dT%H:%M:%SZ; }

failure_json() {
  node - "$1" "$2" "$3" "$4" <<'NODE'
const [code, message, retryableRaw, recovery] = process.argv.slice(2);
process.stdout.write(JSON.stringify([{
  code,
  message,
  retryable: retryableRaw === 'true',
  recovery,
  evidence: { command: 'scripts/release/verify.sh', url: null, path: null },
}]));
NODE
}

emit() {
  local status="$1" data="$2" failures="$3" next="$4" reason="$5" ended
  ended="$(now)"
  node - "$SCHEMA_VERSION" "$COMMAND" "$PHASE" "$status" "$started_at" "$ended" "$repo" "$mode" "$version" "$source_sha" "$data" "$failures" "$next" "$reason" <<'NODE'
const [schemaVersion, command, phase, status, startedAt, endedAt, repository, mode, version, sourceSha, dataRaw, failuresRaw, nextCommand, nextReason] = process.argv.slice(2);
const data = JSON.parse(dataRaw);
process.stdout.write(JSON.stringify({
  schemaVersion, command, phase, mode, status, startedAt, endedAt, repository,
  inputs: { base: null, head: null, version, sourceSha },
  trackingIssue: { repository, number: null, url: null, metadataCommentUrl: null },
  releaseRecord: {
    lifecycleState: status === 'success' ? 'published' : null,
    recordUrl: data.releaseRecordUrl || null,
    sha256: data.releaseRecordSha256 || null,
  },
  data,
  warnings: [],
  failures: JSON.parse(failuresRaw),
  next: { command: nextCommand, reason: nextReason },
}) + '\n');
NODE
}

fail_usage() {
  if [[ "$json" == true ]]; then
    emit failed '{"checks":[],"releaseRecordUrl":null,"releaseRecordSha256":null,"commsDraft":null}' "$(failure_json invalid-input "$1" false correct-usage)" verify 'Fix command arguments.'
  else
    usage >&2
  fi
  exit 129
}

empty_data() { printf '%s' '{"checks":[],"releaseRecordUrl":null,"releaseRecordSha256":null,"commsDraft":null}'; }

required_assets_json() {
  node - "$version" <<'NODE'
const version = process.argv[2];
const crate = 'eddacraft-anvil';
const targets = [
  ['aarch64-apple-darwin', 'tar.xz'],
  ['aarch64-pc-windows-msvc', 'zip'],
  ['aarch64-unknown-linux-gnu', 'tar.xz'],
  ['x86_64-apple-darwin', 'tar.xz'],
  ['x86_64-unknown-linux-gnu', 'tar.xz'],
  ['x86_64-pc-windows-msvc', 'zip'],
];
const names = [];
for (const [triple, ext] of targets) {
  names.push(`${crate}-${triple}.${ext}`, `${crate}-${triple}.${ext}.sha256`);
}
names.push(
  `${crate}-installer.sh`,
  `${crate}-installer.sh.minisig`,
  `${crate}-installer.ps1`,
  `${crate}-installer.ps1.minisig`,
  'dist-manifest.json',
  `anvil-${version}-provenance.json`,
  `anvil-${version}-provenance.json.minisig`,
  `release-evidence-${version}.md`,
);
process.stdout.write(JSON.stringify(names));
NODE
}

evaluate_snapshot() {
  local snapshot_file="$1"
  node - "$version" "$source_sha" "$public_repo" "$(required_assets_json)" "$snapshot_file" <<'NODE'
const fs = require('node:fs');
const [version, sourceSha, publicRepo, requiredRaw, snapshotFile] = process.argv.slice(2);
const required = JSON.parse(requiredRaw);
const snapshot = JSON.parse(fs.readFileSync(snapshotFile, 'utf8'));
const checks = [];
const fail = (name, code, url, detail) => {
  checks.push({ name, status: 'fail', code, url: url || null, detail: detail || null });
};
const pass = (name, url, detail) => {
  checks.push({ name, status: 'pass', url: url || null, detail: detail || null });
};

const releaseOk = (label, rel) => {
  if (!rel || !rel.found) {
    fail(label, 'integrity-failed', rel && rel.url, 'release not found');
    return [];
  }
  if (rel.isDraft) {
    fail(label, 'integrity-failed', rel.url, 'release is still a draft');
    return rel.assets || [];
  }
  pass(label, rel.url, null);
  return rel.assets || [];
};

const privateAssets = releaseOk('private-release', snapshot.privateRelease);
const publicAssets = releaseOk('public-release', snapshot.publicRelease);
const assetSet = new Set([...privateAssets, ...publicAssets]);
const missing = required.filter((name) => !assetSet.has(name));
if (missing.length) {
  fail('cargo-dist-assets', 'integrity-failed', snapshot.publicRelease && snapshot.publicRelease.url, `missing: ${missing.join(', ')}`);
} else {
  pass('cargo-dist-assets', snapshot.publicRelease && snapshot.publicRelease.url, `${required.length} expected assets`);
}

const evidenceName = `release-evidence-${version}.md`;
if (publicAssets.includes(evidenceName)) {
  pass('release-evidence', snapshot.publicRelease && snapshot.publicRelease.url, evidenceName);
} else {
  fail('release-evidence', 'integrity-failed', snapshot.publicRelease && snapshot.publicRelease.url, `${evidenceName} missing from public release`);
}

const provenance = snapshot.provenance;
if (!provenance || typeof provenance !== 'object') {
  fail('provenance', 'integrity-failed', null, 'provenance manifest missing');
} else {
  const tag = provenance.release_tag || '';
  const sha = provenance.private_build && provenance.private_build.commit_sha;
  const runUrl = provenance.private_build && provenance.private_build.workflow_run_url;
  const problems = [];
  if (tag !== version) problems.push(`release_tag ${tag || '(empty)'} != ${version}`);
  if (sha !== sourceSha) problems.push('commit_sha does not match --source-sha');
  if (!runUrl) problems.push('workflow_run_url missing');
  if (problems.length) fail('provenance', 'integrity-failed', runUrl || null, problems.join('; '));
  else pass('provenance', runUrl || null, sha);
}

const evidenceRepo = snapshot.evidenceRepo || {};
if (evidenceRepo.found) {
  pass('evidence-repo', evidenceRepo.url || null, `releases/${evidenceName}`);
} else {
  fail('evidence-repo', 'integrity-failed', evidenceRepo.url || null, `releases/${evidenceName} missing from ${publicRepo}`);
}

const bare = version.replace(/^v/, '');
const formula = snapshot.homebrewFormula || '';
if (!formula) {
  fail('homebrew', 'integrity-failed', null, 'Homebrew formula not found');
} else if (!formula.includes(`version "${bare}"`)) {
  fail('homebrew', 'integrity-failed', null, `formula does not declare version "${bare}"`);
} else {
  pass('homebrew', null, `version ${bare}`);
}

const recordPublisher = (name, entry) => {
  const state = (entry && entry.state) || 'not-configured';
  pass(name, null, state);
};
recordPublisher('scoop', snapshot.scoop);
recordPublisher('winget', snapshot.winget);

const site = snapshot.installSite || {};
const siteUrl = site.url || null;
if (Number(site.status) === 200) pass('install-site', siteUrl, 'HTTP 200');
else fail('install-site', 'integrity-failed', siteUrl, `HTTP ${site.status == null ? '(none)' : site.status}`);

const failed = checks.filter((check) => check.status !== 'pass');
const provenanceSha = provenance && provenance.private_build && provenance.private_build.commit_sha;
const evidenceUrl = snapshot.publicRelease && snapshot.publicRelease.url
  ? `${String(snapshot.publicRelease.url).replace(/\/tag\/[^/]+$/, '')}/download/${version}/${evidenceName}`
  : null;
const data = {
  checks,
  releaseRecordUrl: evidenceUrl,
  releaseRecordSha256: provenanceSha || null,
  commsDraft: failed.length ? null : `Release ${version} verified (${sourceSha.slice(0, 11)}).`,
};
process.stdout.write(JSON.stringify({
  status: failed.length ? 'failed' : 'success',
  exitCode: failed.length ? 1 : 0,
  data,
  failures: failed.map((check) => ({
    code: check.code || 'integrity-failed',
    message: `${check.name} failed${check.detail ? `: ${check.detail}` : ''}`,
    retryable: true,
    recovery: 'fix-and-rerun-verify',
    evidence: { command: 'scripts/release/verify.sh', url: check.url || null, path: null },
  })),
}));
NODE
}

gh_release_snapshot() {
  local target_repo="$1"
  if ! command -v gh >/dev/null 2>&1; then
    printf '%s' '{"found":false,"isDraft":false,"url":null,"assets":[]}'
    return 0
  fi
  local json
  if ! json="$(gh release view "$version" --repo "$target_repo" --json url,isDraft,tagName,assets 2>/dev/null)"; then
    printf '%s' '{"found":false,"isDraft":false,"url":null,"assets":[]}'
    return 0
  fi
  node - "$json" <<'NODE'
const rel = JSON.parse(process.argv[2]);
process.stdout.write(JSON.stringify({
  found: true,
  isDraft: Boolean(rel.isDraft),
  url: rel.url || null,
  assets: Array.isArray(rel.assets) ? rel.assets.map((asset) => asset.name).filter(Boolean) : [],
}));
NODE
}

fetch_provenance_json() {
  local target_repo="$1"
  local tmp
  tmp="$(mktemp -d)"
  if ! command -v gh >/dev/null 2>&1; then
    rm -rf "$tmp"
    printf '%s' 'null'
    return 0
  fi
  if ! gh release download "$version" --repo "$target_repo" --pattern "anvil-*-provenance.json" --dir "$tmp" >/dev/null 2>&1; then
    rm -rf "$tmp"
    printf '%s' 'null'
    return 0
  fi
  local file
  file="$(find "$tmp" -maxdepth 1 -name 'anvil-*-provenance.json' | head -n 1 || true)"
  if [[ -z "$file" ]]; then
    rm -rf "$tmp"
    printf '%s' 'null'
    return 0
  fi
  node - "$file" <<'NODE'
const fs = require('node:fs');
try { process.stdout.write(fs.readFileSync(process.argv[2], 'utf8').trim() || 'null'); }
catch { process.stdout.write('null'); }
NODE
  rm -rf "$tmp"
}

fetch_homebrew_formula() {
  if ! command -v gh >/dev/null 2>&1; then
    printf '%s' ''
    return 0
  fi
  local encoded
  if ! encoded="$(gh api "repos/${tap_repo}/contents/Formula/anvil.rb" --jq .content 2>/dev/null)"; then
    printf '%s' ''
    return 0
  fi
  printf '%s' "$encoded" | tr -d '\n' | base64 -d 2>/dev/null || true
}

fetch_evidence_repo() {
  local path="releases/release-evidence-${version}.md"
  if ! command -v gh >/dev/null 2>&1; then
    printf '%s' '{"found":false,"url":null}'
    return 0
  fi
  if gh api "repos/${public_repo}/contents/${path}" --silent >/dev/null 2>&1; then
    node - "$public_repo" "$path" <<'NODE'
const [repo, path] = process.argv.slice(2);
process.stdout.write(JSON.stringify({
  found: true,
  url: `https://github.com/${repo}/blob/main/${path}`,
}));
NODE
  else
    printf '%s' '{"found":false,"url":null}'
  fi
}

fetch_install_site() {
  local status="0"
  if command -v curl >/dev/null 2>&1; then
    status="$(curl -sS -o /dev/null -w '%{http_code}' --max-time 20 "$install_url" || true)"
  fi
  node - "$install_url" "$status" <<'NODE'
const [url, status] = process.argv.slice(2);
process.stdout.write(JSON.stringify({ url, status: Number(status) || 0 }));
NODE
}

collect_live_snapshot() {
  local private_json public_json provenance_json formula evidence_json site_json formula_file
  private_json="$(gh_release_snapshot "$repo")"
  public_json="$(gh_release_snapshot "$public_repo")"
  provenance_json="$(fetch_provenance_json "$public_repo")"
  if [[ "$provenance_json" == "null" ]]; then
    provenance_json="$(fetch_provenance_json "$repo")"
  fi
  formula="$(fetch_homebrew_formula)"
  evidence_json="$(fetch_evidence_repo)"
  site_json="$(fetch_install_site)"
  formula_file="$(mktemp)"
  printf '%s' "$formula" >"$formula_file"
  node - "$private_json" "$public_json" "$provenance_json" "$formula_file" "$evidence_json" "$site_json" <<'NODE'
const fs = require('node:fs');
const [privateRelease, publicRelease, provenanceRaw, formulaFile, evidenceRepo, installSite] = process.argv.slice(2);
let provenance = null;
try { provenance = JSON.parse(provenanceRaw); } catch { provenance = null; }
process.stdout.write(JSON.stringify({
  privateRelease: JSON.parse(privateRelease),
  publicRelease: JSON.parse(publicRelease),
  provenance,
  homebrewFormula: fs.readFileSync(formulaFile, 'utf8'),
  evidenceRepo: JSON.parse(evidenceRepo),
  installSite: JSON.parse(installSite),
  scoop: { configured: false, state: 'not-configured' },
  winget: { configured: false, state: 'not-configured' },
}));
NODE
  rm -f "$formula_file"
}

emit_evaluated() {
  local snapshot="$1"
  local snapshot_file result status code data fails
  snapshot_file="$(mktemp)"
  printf '%s' "$snapshot" >"$snapshot_file"
  result="$(evaluate_snapshot "$snapshot_file")"
  rm -f "$snapshot_file"
  status="$(node -e "process.stdout.write(JSON.parse(process.argv[1]).status)" "$result")"
  code="$(node -e "process.stdout.write(String(JSON.parse(process.argv[1]).exitCode))" "$result")"
  data="$(node -e "process.stdout.write(JSON.stringify(JSON.parse(process.argv[1]).data))" "$result")"
  fails="$(node -e "process.stdout.write(JSON.stringify(JSON.parse(process.argv[1]).failures))" "$result")"
  if [[ "$status" == success ]]; then
    emit success "$data" "$fails" closeout 'Verification complete; proceed to closeout if successful.'
  else
    emit failed "$data" "$fails" verify 'Fix failed checks and rerun verify.'
  fi
  exit "$code"
}

while (($# > 0)); do
  case "$1" in
    --json) json=true; shift ;;
    --version) version="${2:-}"; shift 2 ;;
    --source-sha) source_sha="${2:-}"; shift 2 ;;
    --repo) repo="${2:-}"; shift 2 ;;
    --public-repo) public_repo="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) fail_usage "unknown argument: $1" ;;
  esac
done

[[ -n "$version" ]] || fail_usage '--version is required'
[[ "$version" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)*)?$ ]] || fail_usage '--version must look like vX.Y.Z[-suffix]'
[[ -n "$source_sha" ]] || fail_usage '--source-sha is required'
[[ "$source_sha" =~ ^[0-9a-fA-F]{40}$ ]] || fail_usage '--source-sha requires a full 40-character commit SHA'

if [[ -n "${ANVIL_RELEASE_VERIFY_FAKE_REPORT_FILE:-}" && "${ANVIL_RELEASE_TEST_MODE:-}" != verify-fake-report ]]; then
  emit failed "$(empty_data)" "$(failure_json invalid-input "ANVIL_RELEASE_VERIFY_FAKE_REPORT_FILE requires ANVIL_RELEASE_TEST_MODE=verify-fake-report" false correct-test-usage)" verify 'Unset the test hook or enable explicit fake report test mode.'
  exit 129
fi
if [[ -n "${ANVIL_RELEASE_VERIFY_FAKE_LIVE_FILE:-}" && "${ANVIL_RELEASE_TEST_MODE:-}" != verify-live-fake ]]; then
  emit failed "$(empty_data)" "$(failure_json invalid-input "ANVIL_RELEASE_VERIFY_FAKE_LIVE_FILE requires ANVIL_RELEASE_TEST_MODE=verify-live-fake" false correct-test-usage)" verify 'Unset the test hook or enable explicit live-fake test mode.'
  exit 129
fi

if [[ -n "${ANVIL_RELEASE_VERIFY_FAKE_REPORT_FILE:-}" ]]; then
  result="$(node - "$ANVIL_RELEASE_VERIFY_FAKE_REPORT_FILE" "$version" "$source_sha" <<'NODE'
const fs = require('node:fs');
const [path, version, sourceSha] = process.argv.slice(2);
const report = JSON.parse(fs.readFileSync(path, 'utf8'));
const failed = (report.checks || []).filter((check) => check.status !== 'pass');
const mismatches = [];
if (report.version && report.version !== version) mismatches.push({ name: 'version binding', code: 'integrity-failed', url: null });
if (report.sourceSha && report.sourceSha !== sourceSha) mismatches.push({ name: 'source SHA binding', code: 'integrity-failed', url: null });
const failures = failed.concat(mismatches);
process.stdout.write(JSON.stringify({
  status: failures.length ? 'failed' : 'success',
  exitCode: failures.length ? 1 : 0,
  data: report,
  failures: failures.map((check) => ({
    code: check.code || 'integrity-failed',
    message: `${check.name} failed`,
    retryable: true,
    recovery: 'fix-and-rerun-verify',
    evidence: { command: 'verify fake report', url: check.url || null, path: null },
  })),
}));
NODE
)"
  status="$(node -e "process.stdout.write(JSON.parse(process.argv[1]).status)" "$result")"
  code="$(node -e "process.stdout.write(String(JSON.parse(process.argv[1]).exitCode))" "$result")"
  data="$(node -e "process.stdout.write(JSON.stringify(JSON.parse(process.argv[1]).data))" "$result")"
  fails="$(node -e "process.stdout.write(JSON.stringify(JSON.parse(process.argv[1]).failures))" "$result")"
  emit "$status" "$data" "$fails" closeout 'Verification complete; proceed to closeout if successful.'
  exit "$code"
fi

if [[ -n "${ANVIL_RELEASE_VERIFY_FAKE_LIVE_FILE:-}" ]]; then
  emit_evaluated "$(cat "$ANVIL_RELEASE_VERIFY_FAKE_LIVE_FILE")"
fi

if ! command -v gh >/dev/null 2>&1; then
  emit blocked "$(empty_data)" "$(failure_json infra-failed 'gh is required for live verification' true install-or-auth-gh)" verify 'Install/authenticate gh and rerun verify.'
  exit 1
fi
if ! command -v curl >/dev/null 2>&1; then
  emit blocked "$(empty_data)" "$(failure_json infra-failed 'curl is required for the install-site check' true install-curl)" verify 'Install curl and rerun verify.'
  exit 1
fi

emit_evaluated "$(collect_live_snapshot)"
