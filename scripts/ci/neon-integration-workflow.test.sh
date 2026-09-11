#!/usr/bin/env bash
# CIB-416 / issue #4586: the credentialed Neon integration workflow contract
# must be structural. CLAWOPEN-011 still owns the hosted proof; this lock
# binds the internal-PR guard, least-privilege permissions, pinned checkout,
# persist-credentials, test command, and always() cleanup to the job/step that
# consumes Neon credentials. Whole-file grep is not sufficient: a required
# value parked in a comment or unrelated job must fail.
#
# Pattern: scripts/ci/secret-calibration-workflow.test.sh (CIB-396 / #4391).

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
workflow="${repo_root}/.github/workflows/neon-integration.yml"
readme="${repo_root}/.github/workflows/README.md"
provisioner="${repo_root}/scripts/ci/create-neon-test-branch.mjs"
provisioner_test="${repo_root}/scripts/ci/create-neon-test-branch.test.mjs"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

assert_contains() {
  local path="$1"
  local expected="$2"
  grep -Fq -- "${expected}" "${path}" || fail "expected ${path} to contain: ${expected}"
}

assert_not_contains() {
  local path="$1"
  local forbidden="$2"
  if grep -Fq -- "${forbidden}" "${path}"; then
    fail "expected ${path} not to contain: ${forbidden}"
  fi
}

assert_not_matches() {
  local path="$1"
  local forbidden="$2"
  if grep -Eq -- "${forbidden}" "${path}"; then
    fail "expected ${path} not to match: ${forbidden}"
  fi
}

[[ -f "${workflow}" ]] || fail "expected workflow to exist: ${workflow}"
[[ -f "${readme}" ]] || fail "expected workflow inventory to exist: ${readme}"
[[ -f "${provisioner}" ]] || fail "expected repo-owned Neon provisioner: ${provisioner}"
[[ -f "${provisioner_test}" ]] || fail "expected provisioner regression test: ${provisioner_test}"

tmp_dir=$(mktemp -d)
cleanup() {
  rm -rf "${tmp_dir}"
}
trap cleanup EXIT

# Node + the repo's `yaml` package (same toolchain as secret-calibration and
# security-summary-gate) so the test needs no PyYAML and cannot be satisfied
# by a parked substring.
checker="${tmp_dir}/neon-credentialed-job.cjs"
cat >"${checker}" <<'NODE'
const fs = require('node:fs');
const yaml = require('yaml');

const file = process.argv[2];
const NEON_SECRET = 'secrets.NEON_TEST_API_KEY';
const NEON_SECRET_REFERENCE =
  /\bsecrets\s*(?:\.\s*NEON_TEST_API_KEY|\[\s*(['"])NEON_TEST_API_KEY\1\s*\])/;
const TEST_COMMAND = 'pnpm --dir apps/anvil-api test:neon';
const CREATE_COMMAND = 'node scripts/ci/create-neon-test-branch.mjs';
const DELETE_COMMAND = 'node scripts/ci/create-neon-test-branch.mjs --delete';
const EXPECTED_GUARD =
  "${{ github.event_name == 'workflow_dispatch' || github.event.pull_request.head.repo.full_name == github.repository }}";
const EXPECTED_REF =
  "${{ github.event_name == 'pull_request' && github.event.pull_request.head.sha || github.sha }}";
const EXPECTED_CLEANUP_IF =
  "${{ always() && steps.create-branch.outputs.branch_name != '' }}";

const norm = (s) => String(s).replace(/\s+/g, ' ').trim();
const fail = (m) => {
  console.error(`${file}: ${m}`);
  process.exit(1);
};

const containsNeonSecret = (value) => {
  if (typeof value === 'string') return NEON_SECRET_REFERENCE.test(value);
  if (Array.isArray(value)) return value.some(containsNeonSecret);
  if (value && typeof value === 'object') {
    return Object.values(value).some(containsNeonSecret);
  }
  return false;
};

const envConsumesNeon = (env) => {
  if (!env || typeof env !== 'object' || Array.isArray(env)) return false;
  return Object.values(env).some(containsNeonSecret);
};

const jobConsumesNeon = (job) => containsNeonSecret(job);

const effectivePermissions = (doc, job) => {
  if (Object.prototype.hasOwnProperty.call(job, 'permissions')) {
    return job.permissions;
  }
  return doc.permissions;
};

const isLeastPrivilegeContentsRead = (perms) => {
  if (!perms || typeof perms !== 'object' || Array.isArray(perms)) return false;
  if (perms.contents !== 'read') return false;
  for (const [key, value] of Object.entries(perms)) {
    if (key === 'contents') continue;
    if (value !== 'none') return false;
  }
  return true;
};

const isCheckout = (step) =>
  typeof step.uses === 'string' && step.uses.startsWith('actions/checkout@');

const isPinnedCheckout = (step) =>
  typeof step.uses === 'string' && /^actions\/checkout@[0-9a-f]{40}$/i.test(step.uses);

const impersonatesCheckout = (step) =>
  typeof step.uses === 'string' &&
  /(?:^|\/)actions\/checkout@/.test(step.uses) &&
  !step.uses.startsWith('actions/checkout@');

const stepRun = (step) => (typeof step.run === 'string' ? step.run : '');
const stepIf = (step) => (step && step.if != null ? String(step.if) : '');

const commandLines = (run) =>
  String(run)
    .split('\n')
    .map((line) => line.replace(/\r$/, '').trim())
    .filter((line) => line.length > 0 && !line.startsWith('#'));

const hasCommandLine = (run, command) => commandLines(run).includes(command);

const doc = yaml.parse(fs.readFileSync(file, 'utf8'));
if (!doc || typeof doc !== 'object' || !doc.jobs || typeof doc.jobs !== 'object') {
  fail('workflow jobs mapping not found');
}

const credentialed = Object.entries(doc.jobs).filter(([, job]) => job && jobConsumesNeon(job));
if (credentialed.length === 0) {
  fail(`no credentialed job consumes ${NEON_SECRET}`);
}

for (const [jobId, job] of credentialed) {
  const cond = norm(job.if == null ? '' : String(job.if));
  if (cond !== EXPECTED_GUARD) {
    fail(`jobs.${jobId}.if is missing the internal-PR guard`);
  }

  const perms = effectivePermissions(doc, job);
  if (!isLeastPrivilegeContentsRead(perms)) {
    fail(`jobs.${jobId} effective permissions are not least-privilege contents: read`);
  }

  if (job['runs-on'] !== 'ubuntu-latest') {
    fail(`jobs.${jobId}.runs-on is not ubuntu-latest`);
  }
  if (job['timeout-minutes'] !== 15) {
    fail(`jobs.${jobId}.timeout-minutes is not 15`);
  }

  const steps = Array.isArray(job.steps) ? job.steps : [];
  const stepIds = new Set();
  for (const step of steps) {
    if (typeof step.id !== 'string' || step.id.length === 0) continue;
    if (stepIds.has(step.id)) {
      fail(`jobs.${jobId} has duplicate step id: ${step.id}`);
    }
    stepIds.add(step.id);
  }
  if (steps.some(impersonatesCheckout)) {
    fail(`jobs.${jobId} has a non-canonical actions/checkout step`);
  }
  const checkouts = steps.filter(isCheckout);
  if (checkouts.length === 0) {
    fail(`jobs.${jobId} has no actions/checkout step`);
  }
  for (const checkout of checkouts) {
    if (!isPinnedCheckout(checkout)) {
      fail(`jobs.${jobId} checkout action is not pinned to a commit SHA`);
    }
    const checkoutWith = checkout.with && typeof checkout.with === 'object' ? checkout.with : {};
    if (checkoutWith['persist-credentials'] !== false) {
      fail(`jobs.${jobId} checkout step is missing persist-credentials: false`);
    }
    const ref = norm(checkoutWith.ref == null ? '' : String(checkoutWith.ref));
    if (ref !== EXPECTED_REF) {
      fail(`jobs.${jobId} checkout step is missing the PR-head SHA ref`);
    }
  }

  const hasCreate = steps.some(
    (step) => hasCommandLine(stepRun(step), CREATE_COMMAND) && envConsumesNeon(step.env),
  );
  if (!hasCreate) {
    fail(`jobs.${jobId} is missing the credentialed Neon create step`);
  }

  if (!steps.some((step) => hasCommandLine(stepRun(step), TEST_COMMAND))) {
    fail(`jobs.${jobId} is missing ${TEST_COMMAND}`);
  }

  const hasCleanup = steps.some(
    (step) =>
      norm(stepIf(step)) === EXPECTED_CLEANUP_IF &&
      hasCommandLine(stepRun(step), DELETE_COMMAND) &&
      envConsumesNeon(step.env),
  );
  if (!hasCleanup) {
    fail(`jobs.${jobId} is missing always() cleanup that deletes the Neon branch`);
  }
}

console.log(
  `${file}: bound ${credentialed.length} credentialed job(s) (${credentialed
    .map(([id]) => id)
    .join(', ')})`,
);
NODE

check_workflow() {
  NODE_PATH="${repo_root}/node_modules" node "${checker}" "$1"
}

# Trigger and authority boundary (workflow-level; not job-local).
assert_contains "${workflow}" 'pull_request:'
assert_contains "${workflow}" 'branches: [main]'
assert_contains "${workflow}" 'workflow_dispatch: {}'
assert_contains "${workflow}" "- 'apps/anvil-api/**'"
assert_contains "${workflow}" "- '.github/workflows/neon-integration.yml'"
assert_contains "${workflow}" "- 'scripts/ci/neon-integration-workflow.test.sh'"
assert_contains "${workflow}" "- 'scripts/ci/create-neon-test-branch.mjs'"
assert_contains "${workflow}" "- 'scripts/ci/create-neon-test-branch.test.mjs'"
assert_contains "${workflow}" "- 'package.json'"
assert_contains "${workflow}" "- 'pnpm-lock.yaml'"
assert_contains "${workflow}" 'concurrency:'
assert_contains "${workflow}" 'cancel-in-progress: true'

assert_not_contains "${workflow}" 'contents: write'
assert_not_contains "${workflow}" 'actions: write'
assert_not_contains "${workflow}" 'id-token: write'
assert_not_contains "${workflow}" 'pull-requests: write'

check_workflow "${workflow}"

assert_fails_with() {
  local fixture="$1"
  local needle="$2"
  local label="$3"
  if check_workflow "${fixture}" 2>"${tmp_dir}/${label}.err"; then
    echo "expected ${label} fixture to fail the structural contract" >&2
    exit 1
  fi
  if ! grep -Fq -- "${needle}" "${tmp_dir}/${label}.err"; then
    echo "${label} fixture failed for the wrong reason:" >&2
    cat "${tmp_dir}/${label}.err" >&2
    exit 1
  fi
  echo "ok: ${label} does not satisfy the structural contract"
}

# Negative: the internal-PR guard appears in a comment and an unrelated job,
# but the credentialed job has no `if`. Whole-file grep would pass; this must
# not.
parked_guard="${tmp_dir}/parked-guard.yml"
cat >"${parked_guard}" <<'YAML'
name: parked-guard
on:
  pull_request:
    branches: [main]
permissions:
  contents: read
jobs:
  otp-attempt-cap:
    # if: ${{ github.event_name == 'workflow_dispatch' || github.event.pull_request.head.repo.full_name == github.repository }}
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1
        with:
          ref: ${{ github.event_name == 'pull_request' && github.event.pull_request.head.sha || github.sha }}
          persist-credentials: false
      - env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs
      - run: pnpm --dir apps/anvil-api test:neon
      - if: ${{ always() && steps.create-branch.outputs.branch_name != '' }}
        env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs --delete
  decoy:
    if: ${{ github.event_name == 'workflow_dispatch' || github.event.pull_request.head.repo.full_name == github.repository }}
    runs-on: ubuntu-latest
    steps:
      - run: echo dead
YAML

grep -Fq 'github.event.pull_request.head.repo.full_name' "${parked_guard}" ||
  fail 'parked-guard fixture must contain the guard string so whole-file grep would pass'
assert_fails_with "${parked_guard}" 'jobs.otp-attempt-cap.if is missing the internal-PR guard' parked-guard

# Negative: persist-credentials: false is parked in a comment and on an
# unrelated job's checkout. The credentialed checkout omits it (GitHub
# defaults to persisting credentials).
parked_persist="${tmp_dir}/parked-persist-credentials.yml"
cat >"${parked_persist}" <<'YAML'
name: parked-persist-credentials
on:
  pull_request:
    branches: [main]
permissions:
  contents: read
jobs:
  otp-attempt-cap:
    if: ${{ github.event_name == 'workflow_dispatch' || github.event.pull_request.head.repo.full_name == github.repository }}
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1
        with:
          ref: ${{ github.event_name == 'pull_request' && github.event.pull_request.head.sha || github.sha }}
          # persist-credentials: false
      - env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs
      - run: pnpm --dir apps/anvil-api test:neon
      - if: ${{ always() && steps.create-branch.outputs.branch_name != '' }}
        env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs --delete
  decoy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1
        with:
          persist-credentials: false
YAML

grep -Fq 'persist-credentials: false' "${parked_persist}" ||
  fail 'parked-persist-credentials fixture must contain persist-credentials: false so whole-file grep would pass'
assert_fails_with \
  "${parked_persist}" \
  'jobs.otp-attempt-cap checkout step is missing persist-credentials: false' \
  parked-persist-credentials

# Negative: the test command and always() cleanup exist only on an unrelated
# job / in comments. The credentialed job still consumes Neon credentials.
parked_test_cleanup="${tmp_dir}/parked-test-cleanup.yml"
cat >"${parked_test_cleanup}" <<'YAML'
name: parked-test-cleanup
on:
  pull_request:
    branches: [main]
permissions:
  contents: read
jobs:
  otp-attempt-cap:
    if: ${{ github.event_name == 'workflow_dispatch' || github.event.pull_request.head.repo.full_name == github.repository }}
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1
        with:
          ref: ${{ github.event_name == 'pull_request' && github.event.pull_request.head.sha || github.sha }}
          persist-credentials: false
      - env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs
      # run: pnpm --dir apps/anvil-api test:neon
      # if: ${{ always() && steps.create-branch.outputs.branch_name != '' }}
      # run: node scripts/ci/create-neon-test-branch.mjs --delete
  decoy:
    runs-on: ubuntu-latest
    steps:
      - run: pnpm --dir apps/anvil-api test:neon
      - if: ${{ always() && steps.create-branch.outputs.branch_name != '' }}
        run: node scripts/ci/create-neon-test-branch.mjs --delete
YAML

grep -Fq 'pnpm --dir apps/anvil-api test:neon' "${parked_test_cleanup}" ||
  fail 'parked-test-cleanup fixture must contain the test command so whole-file grep would pass'
grep -Fq 'always()' "${parked_test_cleanup}" ||
  fail 'parked-test-cleanup fixture must contain always() so whole-file grep would pass'
assert_fails_with \
  "${parked_test_cleanup}" \
  'jobs.otp-attempt-cap is missing pnpm --dir apps/anvil-api test:neon' \
  parked-test-cleanup

# Negative: always() cleanup is parked on an unrelated job while the
# credentialed job still consumes Neon credentials and runs the test.
parked_cleanup="${tmp_dir}/parked-cleanup.yml"
cat >"${parked_cleanup}" <<'YAML'
name: parked-cleanup
on:
  pull_request:
    branches: [main]
permissions:
  contents: read
jobs:
  otp-attempt-cap:
    if: ${{ github.event_name == 'workflow_dispatch' || github.event.pull_request.head.repo.full_name == github.repository }}
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1
        with:
          ref: ${{ github.event_name == 'pull_request' && github.event.pull_request.head.sha || github.sha }}
          persist-credentials: false
      - env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs
      - run: pnpm --dir apps/anvil-api test:neon
      # if: ${{ always() && steps.create-branch.outputs.branch_name != '' }}
      # run: node scripts/ci/create-neon-test-branch.mjs --delete
  decoy:
    runs-on: ubuntu-latest
    steps:
      - if: ${{ always() && steps.create-branch.outputs.branch_name != '' }}
        run: node scripts/ci/create-neon-test-branch.mjs --delete
YAML

grep -Fq 'always()' "${parked_cleanup}" ||
  fail 'parked-cleanup fixture must contain always() so whole-file grep would pass'
assert_fails_with \
  "${parked_cleanup}" \
  'jobs.otp-attempt-cap is missing always() cleanup that deletes the Neon branch' \
  parked-cleanup

# Negative: the secret is only in `with` on a hidden job. A fully valid decoy
# still consumes it via `env`. Env-only scanning would validate the decoy and
# skip the hidden job.
secret_in_with="${tmp_dir}/secret-in-with.yml"
cat >"${secret_in_with}" <<'YAML'
name: secret-in-with
on:
  pull_request:
    branches: [main]
permissions:
  contents: read
jobs:
  hidden-with:
    if: ${{ github.event_name == 'workflow_dispatch' || github.event.pull_request.head.repo.full_name == github.repository }}
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
      - uses: example/upload@v1
        with:
          token: ${{ secrets.NEON_TEST_API_KEY }}
  decoy:
    if: ${{ github.event_name == 'workflow_dispatch' || github.event.pull_request.head.repo.full_name == github.repository }}
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1
        with:
          ref: ${{ github.event_name == 'pull_request' && github.event.pull_request.head.sha || github.sha }}
          persist-credentials: false
      - env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs
      - run: pnpm --dir apps/anvil-api test:neon
      - if: ${{ always() && steps.create-branch.outputs.branch_name != '' }}
        env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs --delete
YAML

assert_fails_with \
  "${secret_in_with}" \
  'jobs.hidden-with has no actions/checkout step' \
  secret-in-with

# Negative: an OR-escape still contains the internal-repository comparison but
# also runs on every pull_request, including forks.
or_escape="${tmp_dir}/or-escape-guard.yml"
cat >"${or_escape}" <<'YAML'
name: or-escape-guard
on:
  pull_request:
    branches: [main]
permissions:
  contents: read
jobs:
  otp-attempt-cap:
    if: ${{ github.event.pull_request.head.repo.full_name == github.repository || github.event_name == 'pull_request' }}
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1
        with:
          ref: ${{ github.event_name == 'pull_request' && github.event.pull_request.head.sha || github.sha }}
          persist-credentials: false
      - env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs
      - run: pnpm --dir apps/anvil-api test:neon
      - if: ${{ always() && steps.create-branch.outputs.branch_name != '' }}
        env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs --delete
YAML

assert_fails_with \
  "${or_escape}" \
  'jobs.otp-attempt-cap.if is missing the internal-PR guard' \
  or-escape-guard

# Negative: contents: read plus another write scope is not least privilege.
extra_permission="${tmp_dir}/extra-permission.yml"
cat >"${extra_permission}" <<'YAML'
name: extra-permission
on:
  pull_request:
    branches: [main]
permissions:
  contents: read
jobs:
  otp-attempt-cap:
    if: ${{ github.event_name == 'workflow_dispatch' || github.event.pull_request.head.repo.full_name == github.repository }}
    permissions:
      contents: read
      issues: write
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1
        with:
          ref: ${{ github.event_name == 'pull_request' && github.event.pull_request.head.sha || github.sha }}
          persist-credentials: false
      - env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs
      - run: pnpm --dir apps/anvil-api test:neon
      - if: ${{ always() && steps.create-branch.outputs.branch_name != '' }}
        env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs --delete
YAML

assert_fails_with \
  "${extra_permission}" \
  'jobs.otp-attempt-cap effective permissions are not least-privilege contents: read' \
  extra-permission

# Negative: both SHA expressions appear, but the operator selects the merge
# SHA on pull requests.
malformed_ref="${tmp_dir}/malformed-ref.yml"
cat >"${malformed_ref}" <<'YAML'
name: malformed-ref
on:
  pull_request:
    branches: [main]
permissions:
  contents: read
jobs:
  otp-attempt-cap:
    if: ${{ github.event_name == 'workflow_dispatch' || github.event.pull_request.head.repo.full_name == github.repository }}
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1
        with:
          ref: ${{ github.event.pull_request.head.sha && github.sha }}
          persist-credentials: false
      - env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs
      - run: pnpm --dir apps/anvil-api test:neon
      - if: ${{ always() && steps.create-branch.outputs.branch_name != '' }}
        env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs --delete
YAML

assert_fails_with \
  "${malformed_ref}" \
  'jobs.otp-attempt-cap checkout step is missing the PR-head SHA ref' \
  malformed-ref

# Negative: a later checkout omits persist-credentials and uses a different ref.
extra_checkout="${tmp_dir}/extra-checkout.yml"
cat >"${extra_checkout}" <<'YAML'
name: extra-checkout
on:
  pull_request:
    branches: [main]
permissions:
  contents: read
jobs:
  otp-attempt-cap:
    if: ${{ github.event_name == 'workflow_dispatch' || github.event.pull_request.head.repo.full_name == github.repository }}
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1
        with:
          ref: ${{ github.event_name == 'pull_request' && github.event.pull_request.head.sha || github.sha }}
          persist-credentials: false
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1
        with:
          ref: ${{ github.sha }}
      - env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs
      - run: pnpm --dir apps/anvil-api test:neon
      - if: ${{ always() && steps.create-branch.outputs.branch_name != '' }}
        env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs --delete
YAML

assert_fails_with \
  "${extra_checkout}" \
  'jobs.otp-attempt-cap checkout step is missing persist-credentials: false' \
  extra-checkout

# Negative: a non-canonical checkout action would satisfy a path-substring
# match and receive this job's credentials context.
impersonation_checkout="${tmp_dir}/impersonation-checkout.yml"
cat >"${impersonation_checkout}" <<'YAML'
name: impersonation-checkout
on:
  pull_request:
    branches: [main]
permissions:
  contents: read
jobs:
  otp-attempt-cap:
    if: ${{ github.event_name == 'workflow_dispatch' || github.event.pull_request.head.repo.full_name == github.repository }}
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
      - uses: evil/actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1
        with:
          ref: ${{ github.event_name == 'pull_request' && github.event.pull_request.head.sha || github.sha }}
          persist-credentials: false
      - env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs
      - run: pnpm --dir apps/anvil-api test:neon
      - if: ${{ always() && steps.create-branch.outputs.branch_name != '' }}
        env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs --delete
YAML

assert_fails_with \
  "${impersonation_checkout}" \
  'jobs.otp-attempt-cap has a non-canonical actions/checkout step' \
  impersonation-checkout

# Negative: the canonical checkout repository is still unsafe when its action
# revision is a mutable branch or tag instead of an immutable commit SHA.
mutable_checkout_ref="${tmp_dir}/mutable-checkout-ref.yml"
cat >"${mutable_checkout_ref}" <<'YAML'
name: mutable-checkout-ref
on:
  pull_request:
    branches: [main]
permissions:
  contents: read
jobs:
  otp-attempt-cap:
    if: ${{ github.event_name == 'workflow_dispatch' || github.event.pull_request.head.repo.full_name == github.repository }}
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
      - uses: actions/checkout@main
        with:
          ref: ${{ github.event_name == 'pull_request' && github.event.pull_request.head.sha || github.sha }}
          persist-credentials: false
      - id: create-branch
        env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs
      - run: pnpm --dir apps/anvil-api test:neon
      - if: ${{ always() && steps.create-branch.outputs.branch_name != '' }}
        env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs --delete
YAML

assert_fails_with \
  "${mutable_checkout_ref}" \
  'jobs.otp-attempt-cap checkout action is not pinned to a commit SHA' \
  mutable-checkout-ref

# Negative: the create command exists only as a shell comment (or echo).
comment_create="${tmp_dir}/comment-create.yml"
cat >"${comment_create}" <<'YAML'
name: comment-create
on:
  pull_request:
    branches: [main]
permissions:
  contents: read
jobs:
  otp-attempt-cap:
    if: ${{ github.event_name == 'workflow_dispatch' || github.event.pull_request.head.repo.full_name == github.repository }}
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1
        with:
          ref: ${{ github.event_name == 'pull_request' && github.event.pull_request.head.sha || github.sha }}
          persist-credentials: false
      - env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: |
          # node scripts/ci/create-neon-test-branch.mjs
          echo node scripts/ci/create-neon-test-branch.mjs
      - run: pnpm --dir apps/anvil-api test:neon
      - if: ${{ always() && steps.create-branch.outputs.branch_name != '' }}
        env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs --delete
YAML

assert_fails_with \
  "${comment_create}" \
  'jobs.otp-attempt-cap is missing the credentialed Neon create step' \
  comment-create

# Negative: the test command exists only as a shell comment.
comment_test="${tmp_dir}/comment-test.yml"
cat >"${comment_test}" <<'YAML'
name: comment-test
on:
  pull_request:
    branches: [main]
permissions:
  contents: read
jobs:
  otp-attempt-cap:
    if: ${{ github.event_name == 'workflow_dispatch' || github.event.pull_request.head.repo.full_name == github.repository }}
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1
        with:
          ref: ${{ github.event_name == 'pull_request' && github.event.pull_request.head.sha || github.sha }}
          persist-credentials: false
      - env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs
      - run: |
          # pnpm --dir apps/anvil-api test:neon
          echo pnpm --dir apps/anvil-api test:neon
      - if: ${{ always() && steps.create-branch.outputs.branch_name != '' }}
        env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs --delete
YAML

assert_fails_with \
  "${comment_test}" \
  'jobs.otp-attempt-cap is missing pnpm --dir apps/anvil-api test:neon' \
  comment-test

# Negative: cleanup --delete exists only as a shell comment.
comment_cleanup="${tmp_dir}/comment-cleanup.yml"
cat >"${comment_cleanup}" <<'YAML'
name: comment-cleanup
on:
  pull_request:
    branches: [main]
permissions:
  contents: read
jobs:
  otp-attempt-cap:
    if: ${{ github.event_name == 'workflow_dispatch' || github.event.pull_request.head.repo.full_name == github.repository }}
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1
        with:
          ref: ${{ github.event_name == 'pull_request' && github.event.pull_request.head.sha || github.sha }}
          persist-credentials: false
      - env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs
      - run: pnpm --dir apps/anvil-api test:neon
      - if: ${{ always() && steps.create-branch.outputs.branch_name != '' }}
        env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: |
          # node scripts/ci/create-neon-test-branch.mjs --delete
          echo node scripts/ci/create-neon-test-branch.mjs --delete
YAML

assert_fails_with \
  "${comment_cleanup}" \
  'jobs.otp-attempt-cap is missing always() cleanup that deletes the Neon branch' \
  comment-cleanup

# Negative: always() is present but negated / AND-false, so cleanup never runs.
always_false_cleanup="${tmp_dir}/always-false-cleanup.yml"
cat >"${always_false_cleanup}" <<'YAML'
name: always-false-cleanup
on:
  pull_request:
    branches: [main]
permissions:
  contents: read
jobs:
  otp-attempt-cap:
    if: ${{ github.event_name == 'workflow_dispatch' || github.event.pull_request.head.repo.full_name == github.repository }}
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1
        with:
          ref: ${{ github.event_name == 'pull_request' && github.event.pull_request.head.sha || github.sha }}
          persist-credentials: false
      - env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs
      - run: pnpm --dir apps/anvil-api test:neon
      - if: ${{ always() && false }}
        env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs --delete
YAML

assert_fails_with \
  "${always_false_cleanup}" \
  'jobs.otp-attempt-cap is missing always() cleanup that deletes the Neon branch' \
  always-false-cleanup

# Negative: duplicate step ids make steps.create-branch.outputs ambiguous. The
# credentialed create step must be the unique owner of that identity.
duplicate_step_id="${tmp_dir}/duplicate-step-id.yml"
cat >"${duplicate_step_id}" <<'YAML'
name: duplicate-step-id
on:
  pull_request:
    branches: [main]
permissions:
  contents: read
jobs:
  otp-attempt-cap:
    if: ${{ github.event_name == 'workflow_dispatch' || github.event.pull_request.head.repo.full_name == github.repository }}
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1
        with:
          ref: ${{ github.event_name == 'pull_request' && github.event.pull_request.head.sha || github.sha }}
          persist-credentials: false
      - id: create-branch
        env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs
      - id: create-branch
        run: echo duplicate-step-id
      - run: pnpm --dir apps/anvil-api test:neon
      - if: ${{ always() && steps.create-branch.outputs.branch_name != '' }}
        env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs --delete
YAML

assert_fails_with \
  "${duplicate_step_id}" \
  'jobs.otp-attempt-cap has duplicate step id: create-branch' \
  duplicate-step-id

# Negative: bracket notation is equivalent GitHub expression syntax. Every job
# that consumes the Neon secret must be discovered and checked, even when it
# does not use the dot spelling present in the primary job.
bracket_secret_consumer="${tmp_dir}/bracket-secret-consumer.yml"
cp "${workflow}" "${bracket_secret_consumer}"
cat >>"${bracket_secret_consumer}" <<'YAML'
  bracket-secret-consumer:
    runs-on: self-hosted
    steps:
      - env:
          NEON_API_KEY: ${{ secrets['NEON_TEST_API_KEY'] }}
        run: echo bracket-secret-consumer
YAML

assert_fails_with \
  "${bracket_secret_consumer}" \
  'jobs.bracket-secret-consumer.if is missing the internal-PR guard' \
  bracket-secret-consumer

# Hosted-proof wiring retained from CLAWOPEN-011. These strings still appear
# in the live workflow; the structural lock above is what stops them satisfying
# the credentialed-job contract from a comment or unrelated job.
assert_contains "${workflow}" 'ci-test-clawopen-011-${GITHUB_RUN_ID}-${GITHUB_RUN_ATTEMPT}'
assert_contains "${workflow}" 'ANVIL_API_TEST_DATABASE_URL: ${{ steps.create-branch.outputs.db_url }}'
assert_contains "${workflow}" 'ANVIL_API_TEST_DATABASE_PROJECT_NAME: anvil-api-test'
assert_contains "${workflow}" 'ANVIL_API_TEST_DATABASE_BRANCH_NAME: ${{ steps.branch-metadata.outputs.branch_name }}'
assert_contains "${workflow}" 'NEON_PROJECT_ID: ${{ vars.NEON_TEST_PROJECT_ID }}'
assert_contains "${workflow}" 'NEON_BRANCH_NAME: ${{ steps.branch-metadata.outputs.branch_name }}'
assert_contains "${workflow}" 'NEON_BRANCH_EXPIRES_AT: ${{ steps.branch-metadata.outputs.expires_at }}'
assert_contains "${workflow}" 'NEON_BRANCH_ID: ${{ steps.create-branch.outputs.branch_id }}'

# The repo-owned provisioner verifies the control-plane project before the
# branch POST, masks generated credentials, and only then emits the direct URL.
assert_not_contains "${workflow}" 'neondatabase/create-branch-action'
assert_contains "${provisioner}" "const EXPECTED_PROJECT_NAME = 'anvil-api-test'"
assert_contains "${provisioner}" "const EXPECTED_DATABASE_NAME = 'anvil_test'"
assert_contains "${provisioner}" "const EXPECTED_ROLE_NAME = 'anvil_test_owner'"
assert_contains "${provisioner}" '::add-mask::'
assert_contains "${provisioner}" 'writeOutput(`db_url=${rawDatabaseUrl}`)'
assert_contains "${provisioner}" 'writeOutput(`branch_id=${branchId}`)'
assert_not_contains "${workflow}" 'db_url_pooled'
assert_not_matches "${workflow}" '^[[:space:]]*DATABASE_URL:'

# Cleanup resolves an exact branch name when a partial create did not return an
# ID. The branch expiry remains the final fallback for runner cancellation.
assert_contains "${provisioner}" "method: 'DELETE'"
assert_contains "${provisioner}" 'branch?.name === branchName'
assert_not_contains "${workflow}" 'neondatabase/delete-branch-action'
assert_not_contains "${workflow}" 'neonctl'

assert_contains "${readme}" '`neon-integration.yml`'
assert_contains "${readme}" 'CLAWOPEN-011'

echo 'Neon integration workflow contract passed'
