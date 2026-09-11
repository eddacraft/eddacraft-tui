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
  /\bsecrets\s*(?:\.\s*NEON_TEST_API_KEY|\[\s*(['"])NEON_TEST_API_KEY\1\s*\])/i;
const POSSIBLE_SECRET_INDEX = /\bsecrets\s*\[/i;
const POSSIBLE_WHOLE_SECRETS_CONTEXT = /\bsecrets\b(?!\s*(?:\.|\[))/i;
const TEST_COMMAND = 'pnpm --dir apps/anvil-api test:neon';
const CREATE_COMMAND = 'node scripts/ci/create-neon-test-branch.mjs';
const DELETE_COMMAND = 'node scripts/ci/create-neon-test-branch.mjs --delete';
const EXPECTED_GUARD =
  "${{ github.event_name == 'workflow_dispatch' || github.event.pull_request.head.repo.full_name == github.repository }}";
const EXPECTED_REF =
  "${{ github.event_name == 'pull_request' && github.event.pull_request.head.sha || github.sha }}";
const EXPECTED_CLEANUP_IF =
  "${{ always() && steps.create-branch.outputs.branch_name != '' }}";
const EXPECTED_SECRET_BINDING = '${{ secrets.NEON_TEST_API_KEY }}';

const norm = (s) => String(s).replace(/\s+/g, ' ').trim();
const fail = (m) => {
  console.error(`${file}: ${m}`);
  process.exit(1);
};

const containsNeonSecret = (value) => {
  if (typeof value === 'string') {
    return (
      NEON_SECRET_REFERENCE.test(value) ||
      POSSIBLE_SECRET_INDEX.test(value) ||
      POSSIBLE_WHOLE_SECRETS_CONTEXT.test(value)
    );
  }
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

const jobConsumesNeon = (job) => job?.secrets === 'inherit' || containsNeonSecret(job);

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
const isUnconditional = (step) => step && step.if == null;
const isFailClosed = (step) =>
  step && (step['continue-on-error'] == null || step['continue-on-error'] === false);

const hasExactCommand = (step, command) => stepRun(step).trim() === command;

const doc = yaml.parse(fs.readFileSync(file, 'utf8'));
if (!doc || typeof doc !== 'object' || !doc.jobs || typeof doc.jobs !== 'object') {
  fail('workflow jobs mapping not found');
}
if (containsNeonSecret(doc.env)) {
  fail('workflow-level env must not expose Neon credentials');
}
if (
  doc.defaults?.run &&
  Object.prototype.hasOwnProperty.call(doc.defaults.run, 'shell')
) {
  fail('workflow defaults.run.shell must not override critical command execution');
}
for (const [jobId, job] of Object.entries(doc.jobs)) {
  if (
    job &&
    typeof job.uses === 'string' &&
    Object.prototype.hasOwnProperty.call(job, 'secrets')
  ) {
    fail(`jobs.${jobId} reusable workflow jobs must not forward secrets`);
  }
  if (job && containsNeonSecret(job.env)) {
    fail(`jobs.${jobId} job-level env must not expose Neon credentials`);
  }
  if (job && typeof job === 'object' && !Array.isArray(job)) {
    const jobScope = { ...job };
    delete jobScope.steps;
    delete jobScope.env;
    if (containsNeonSecret(jobScope)) {
      fail(
        `jobs.${jobId} job-scope fields outside steps must not expose Neon credentials`,
      );
    }
  }
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
  if (
    job.defaults?.run &&
    Object.prototype.hasOwnProperty.call(job.defaults.run, 'shell')
  ) {
    fail(`jobs.${jobId} job defaults.run.shell must not override critical command execution`);
  }
  if (!isFailClosed(job)) {
    fail(`jobs.${jobId} credentialed job is not fail-closed`);
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
    if (!isUnconditional(checkout) || !isFailClosed(checkout)) {
      fail(`jobs.${jobId} checkout step is not unconditional and fail-closed`);
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

  const createSteps = steps.filter((step) => hasExactCommand(step, CREATE_COMMAND));
  if (createSteps.length === 0) {
    fail(`jobs.${jobId} is missing the credentialed Neon create step`);
  }
  if (createSteps.length !== 1) {
    fail(`jobs.${jobId} expected exactly one Neon create command`);
  }
  if (!envConsumesNeon(createSteps[0].env)) {
    fail(`jobs.${jobId} Neon create step is missing step-local credentials`);
  }
  const createStep = createSteps[0];
  if (createStep.id !== 'create-branch') {
    fail(`jobs.${jobId} credentialed create step must use id: create-branch`);
  }
  if (!isUnconditional(createStep) || !isFailClosed(createStep)) {
    fail(`jobs.${jobId} credentialed create step is not unconditional and fail-closed`);
  }

  const testSteps = steps.filter(
    (step) => isUnconditional(step) && hasExactCommand(step, TEST_COMMAND),
  );
  if (testSteps.length === 0) {
    fail(`jobs.${jobId} is missing an unconditional ${TEST_COMMAND} step`);
  }
  if (!testSteps.some(isFailClosed)) {
    fail(`jobs.${jobId} is missing a fail-closed ${TEST_COMMAND} step`);
  }

  const deleteSteps = steps.filter((step) => hasExactCommand(step, DELETE_COMMAND));
  const cleanupStep = deleteSteps.find(
    (step) =>
      norm(stepIf(step)) === EXPECTED_CLEANUP_IF &&
      envConsumesNeon(step.env) &&
      isFailClosed(step),
  );
  if (!cleanupStep) {
    fail(`jobs.${jobId} is missing always() cleanup that deletes the Neon branch`);
  }

  const allowedSecretSteps = new Set([createStep, cleanupStep]);
  for (const step of steps) {
    if (!containsNeonSecret(step)) continue;
    const env = step.env && typeof step.env === 'object' ? step.env : {};
    const stepWithoutEnv = { ...step };
    delete stepWithoutEnv.env;
    const hasUnexpectedExposure =
      !allowedSecretSteps.has(step) ||
      norm(env.NEON_API_KEY == null ? '' : env.NEON_API_KEY) !== EXPECTED_SECRET_BINDING ||
      Object.entries(env).some(
        ([name, value]) => name !== 'NEON_API_KEY' && containsNeonSecret(value),
      ) ||
      containsNeonSecret(stepWithoutEnv);
    if (hasUnexpectedExposure) {
      fail(`jobs.${jobId} Neon credentials may only be exposed to the create and cleanup steps`);
    }
  }
}

const workflowCommandOwners = (command) => Object.entries(doc.jobs).flatMap(([jobId, job]) => {
  const steps = Array.isArray(job?.steps) ? job.steps : [];
  return steps
    .map((step, stepIndex) => ({ jobId, stepIndex, step }))
    .filter(({ step }) => hasExactCommand(step, command));
});
const commandOwners = Object.fromEntries([
  ['create', CREATE_COMMAND],
  ['test', TEST_COMMAND],
  ['delete', DELETE_COMMAND],
].map(([label, command]) => [label, workflowCommandOwners(command)]));
for (const [label, owners] of Object.entries(commandOwners)) {
  if (owners.length !== 1) {
    fail(`expected exactly one workflow-wide Neon ${label} command, found ${owners.length}`);
  }
}
for (const [jobId, job] of Object.entries(doc.jobs)) {
  const steps = Array.isArray(job?.steps) ? job.steps : [];
  for (const [stepIndex, step] of steps.entries()) {
    const run = stepRun(step);
    if (
      run.includes('scripts/ci/create-neon-test-branch.mjs') &&
      !hasExactCommand(step, CREATE_COMMAND) &&
      !hasExactCommand(step, DELETE_COMMAND)
    ) {
      fail(`jobs.${jobId}.steps[${stepIndex}] has an unsupported wrapped Neon command`);
    }
  }
}
const [createOwner] = commandOwners.create;
const [testOwner] = commandOwners.test;
const [deleteOwner] = commandOwners.delete;
for (const owner of [createOwner, testOwner, deleteOwner]) {
  if (Object.prototype.hasOwnProperty.call(owner.step, 'shell')) {
    fail('critical Neon command steps must not override shell');
  }
}
if (createOwner.jobId !== testOwner.jobId || createOwner.jobId !== deleteOwner.jobId) {
  fail('the Neon create, test, and delete commands must belong to one job');
}
const ownerSteps = doc.jobs[createOwner.jobId].steps;
const checkoutIndexes = ownerSteps
  .map((step, stepIndex) => ({ step, stepIndex }))
  .filter(({ step }) => isCheckout(step))
  .map(({ stepIndex }) => stepIndex);
if (
  checkoutIndexes.some((stepIndex) => stepIndex >= createOwner.stepIndex) ||
  !(createOwner.stepIndex < testOwner.stepIndex && testOwner.stepIndex < deleteOwner.stepIndex)
) {
  fail('required command order is checkout, create, test, delete');
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
      - id: create-branch
        env:
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
      - id: create-branch
        env:
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
      - id: create-branch
        env:
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
  'jobs.otp-attempt-cap is missing an unconditional pnpm --dir apps/anvil-api test:neon step' \
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
      - id: create-branch
        env:
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
      - id: create-branch
        env:
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
  'jobs.otp-attempt-cap is missing an unconditional pnpm --dir apps/anvil-api test:neon step' \
  comment-test

# Negative: a test command guarded by a constant false condition is present in
# the workflow but can never prove the credentialed boundary.
disabled_test="${tmp_dir}/disabled-test.yml"
cat >"${disabled_test}" <<'YAML'
name: disabled-test
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
      - if: false
        run: pnpm --dir apps/anvil-api test:neon
      - if: ${{ always() && steps.create-branch.outputs.branch_name != '' }}
        env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs --delete
YAML

assert_fails_with \
  "${disabled_test}" \
  'jobs.otp-attempt-cap is missing an unconditional pnpm --dir apps/anvil-api test:neon step' \
  disabled-test

# Negative: the proof command must fail the job when it fails.
continued_test="${tmp_dir}/continued-test.yml"
cat >"${continued_test}" <<'YAML'
name: continued-test
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
      - continue-on-error: true
        run: pnpm --dir apps/anvil-api test:neon
      - if: ${{ always() && steps.create-branch.outputs.branch_name != '' }}
        env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs --delete
YAML

assert_fails_with \
  "${continued_test}" \
  'jobs.otp-attempt-cap is missing a fail-closed pnpm --dir apps/anvil-api test:neon step' \
  continued-test

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
      - id: create-branch
        env:
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

# Negative: finding the delete command on one line is insufficient when shell
# control flow guarantees that line never executes.
wrapped_cleanup="${tmp_dir}/wrapped-cleanup.yml"
cat >"${wrapped_cleanup}" <<'YAML'
name: wrapped-cleanup
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
      - run: pnpm --dir apps/anvil-api test:neon
      - if: ${{ always() && steps.create-branch.outputs.branch_name != '' }}
        env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: |
          if false; then
            node scripts/ci/create-neon-test-branch.mjs --delete
          fi
YAML

assert_fails_with \
  "${wrapped_cleanup}" \
  'jobs.otp-attempt-cap is missing always() cleanup that deletes the Neon branch' \
  wrapped-cleanup

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
      - id: create-branch
        env:
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

# Negative: the unique create-branch identity must belong to the credentialed
# provisioner step, not to a harmless decoy whose outputs are then consumed.
displaced_create_id="${tmp_dir}/displaced-create-id.yml"
cat >"${displaced_create_id}" <<'YAML'
name: displaced-create-id
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
      - id: actual-create
        env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs
      - id: create-branch
        run: echo decoy-create-branch
      - run: pnpm --dir apps/anvil-api test:neon
      - if: ${{ always() && steps.create-branch.outputs.branch_name != '' }}
        env:
          NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}
        run: node scripts/ci/create-neon-test-branch.mjs --delete
YAML

assert_fails_with \
  "${displaced_create_id}" \
  'jobs.otp-attempt-cap credentialed create step must use id: create-branch' \
  displaced-create-id

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

# Negative: a dynamic bracket lookup may resolve to the Neon key at runtime.
# The contract must treat unresolved indexing as credential consumption rather
# than guess that a different secret is intended.
computed_secret_consumer="${tmp_dir}/computed-secret-consumer.yml"
cp "${workflow}" "${computed_secret_consumer}"
cat >>"${computed_secret_consumer}" <<'YAML'
  computed-secret-consumer:
    runs-on: self-hosted
    steps:
      - env:
          NEON_API_KEY: ${{ secrets[format('NEON_{0}', 'TEST_API_KEY')] }}
        run: echo computed-secret-consumer
YAML

assert_fails_with \
  "${computed_secret_consumer}" \
  'jobs.computed-secret-consumer.if is missing the internal-PR guard' \
  computed-secret-consumer

# Negative: GitHub's secrets context name is case-insensitive, so
# Secrets[format(...)] is equivalent to secrets[format(...)].
computed_secret_context_case="${tmp_dir}/computed-secret-context-case.yml"
cp "${workflow}" "${computed_secret_context_case}"
cat >>"${computed_secret_context_case}" <<'YAML'
  computed-secret-context-case:
    runs-on: self-hosted
    steps:
      - env:
          NEON_API_KEY: ${{ Secrets[format('NEON_{0}', 'TEST_API_KEY')] }}
        run: echo computed-secret-context-case
YAML

assert_fails_with \
  "${computed_secret_context_case}" \
  'jobs.computed-secret-context-case.if is missing the internal-PR guard' \
  computed-secret-context-case

# Negative: reusable-workflow secret inheritance can forward the Neon key
# without naming it in this document and therefore must fail closed too.
inherited_secret_consumer="${tmp_dir}/inherited-secret-consumer.yml"
cp "${workflow}" "${inherited_secret_consumer}"
cat >>"${inherited_secret_consumer}" <<'YAML'
  inherited-secret-consumer:
    uses: example/repository/.github/workflows/reusable.yml@0123456789abcdef0123456789abcdef01234567
    secrets: inherit
YAML

assert_fails_with \
  "${inherited_secret_consumer}" \
  'jobs.inherited-secret-consumer reusable workflow jobs must not forward secrets' \
  inherited-secret-consumer

# Negative: workflow-level env reaches every ordinary job. A valid decoy job
# must not hide an unguarded self-hosted sibling that inherits the Neon key.
workflow_env_consumer="${tmp_dir}/workflow-env-consumer.yml"
sed '/^jobs:/i\
env:\
  NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}\
' "${workflow}" >"${workflow_env_consumer}"
cat >>"${workflow_env_consumer}" <<'YAML'
  workflow-env-consumer:
    runs-on: self-hosted
    steps:
      - run: echo workflow-env-consumer
YAML

assert_fails_with \
  "${workflow_env_consumer}" \
  'workflow-level env must not expose Neon credentials' \
  workflow-env-consumer

# Negative: command ownership is independent of where credentials are bound.
# A second exact create command without step-local env is still ambiguous.
duplicate_create_command="${tmp_dir}/duplicate-create-command.yml"
awk '
  /      - name: Prove the OTP attempt cap against Neon/ {
    print "      - name: Duplicate Neon create command"
    print "        run: node scripts/ci/create-neon-test-branch.mjs"
    print ""
  }
  { print }
' "${workflow}" >"${duplicate_create_command}"

assert_fails_with \
  "${duplicate_create_command}" \
  'expected exactly one Neon create command' \
  duplicate-create-command

# Negative: job-level env exposes the Neon key to every step and therefore
# cannot establish unique credential ownership for create and cleanup.
job_env_consumer="${tmp_dir}/job-env-consumer.yml"
awk '
  /    timeout-minutes: 15/ {
    print
    print "    env:"
    print "      GLOBAL_NEON_API_KEY: ${{ secrets.NEON_TEST_API_KEY }}"
    next
  }
  { print }
' "${workflow}" >"${job_env_consumer}"

assert_fails_with \
  "${job_env_consumer}" \
  'job-level env must not expose Neon credentials' \
  job-env-consumer

# Negative: job-scope credential fields other than env (container or service
# credentials) still classify the job as credentialed via jobConsumesNeon,
# skip the job.env rejection, and never reach the step-local exposure loop.
job_container_credentials="${tmp_dir}/job-container-credentials.yml"
awk '
  /    timeout-minutes: 15/ {
    print
    print "    container:"
    print "      image: example/image:latest"
    print "      credentials:"
    print "        username: neon"
    print "        password: ${{ secrets.NEON_TEST_API_KEY }}"
    next
  }
  { print }
' "${workflow}" >"${job_container_credentials}"

assert_fails_with \
  "${job_container_credentials}" \
  'jobs.otp-attempt-cap job-scope fields outside steps must not expose Neon credentials' \
  job-container-credentials

# Negative: command ownership is workflow-global, not merely unique inside the
# credentialed job.
cross_job_create_command="${tmp_dir}/cross-job-create-command.yml"
cp "${workflow}" "${cross_job_create_command}"
cat >>"${cross_job_create_command}" <<'YAML'
  cross-job-create-command:
    runs-on: ubuntu-latest
    steps:
      - run: node scripts/ci/create-neon-test-branch.mjs
YAML

assert_fails_with \
  "${cross_job_create_command}" \
  'expected exactly one workflow-wide Neon create command' \
  cross-job-create-command

# Negative: the test proof also has one workflow-wide owner.
duplicate_test_command="${tmp_dir}/duplicate-test-command.yml"
awk '
  /      - name: Delete ephemeral Neon test branch/ {
    print "      - name: Duplicate Neon test command"
    print "        run: pnpm --dir apps/anvil-api test:neon"
    print ""
  }
  { print }
' "${workflow}" >"${duplicate_test_command}"

assert_fails_with \
  "${duplicate_test_command}" \
  'expected exactly one workflow-wide Neon test command' \
  duplicate-test-command

# Negative: a structurally valid owner job must not expose the Neon key to an
# unrelated step. Only the exact create and cleanup owners may receive it.
extra_secret_step="${tmp_dir}/extra-secret-step.yml"
awk '
  /      - name: Delete ephemeral Neon test branch/ {
    print "      - name: Leak Neon key to another action"
    print "        uses: example/upload@0123456789abcdef0123456789abcdef01234567"
    print "        with:"
    print "          token: ${{ secrets.NEON_TEST_API_KEY }}"
    print ""
  }
  { print }
' "${workflow}" >"${extra_secret_step}"

assert_fails_with \
  "${extra_secret_step}" \
  'Neon credentials may only be exposed to the create and cleanup steps' \
  extra-secret-step

# Negative: this workflow cannot prove the internals of a called workflow, so
# reusable-workflow secret forwarding is an explicit fail-closed boundary.
reusable_secret_mapping="${tmp_dir}/reusable-secret-mapping.yml"
cp "${workflow}" "${reusable_secret_mapping}"
cat >>"${reusable_secret_mapping}" <<'YAML'
  reusable-secret-mapping:
    uses: example/repository/.github/workflows/reusable.yml@0123456789abcdef0123456789abcdef01234567
    secrets:
      neon_api_key: ${{ secrets.NEON_TEST_API_KEY }}
YAML

assert_fails_with \
  "${reusable_secret_mapping}" \
  'reusable workflow jobs must not forward secrets' \
  reusable-secret-mapping

# Negative: step-level checks are not a gate when the whole credentialed job
# is allowed to fail without failing the workflow.
continued_job="${tmp_dir}/continued-job.yml"
awk '
  /    timeout-minutes: 15/ {
    print
    print "    continue-on-error: true"
    next
  }
  { print }
' "${workflow}" >"${continued_job}"

assert_fails_with \
  "${continued_job}" \
  'credentialed job is not fail-closed' \
  continued-job

# Negative: cleanup must follow the test it protects; mere co-location in one
# job does not establish the pipeline dependency.
wrong_command_order="${tmp_dir}/wrong-command-order.yml"
NODE_PATH="${repo_root}/node_modules" node - "${workflow}" "${wrong_command_order}" <<'NODE'
const fs = require('node:fs');
const yaml = require('yaml');
const source = process.argv[2];
const target = process.argv[3];
const doc = yaml.parse(fs.readFileSync(source, 'utf8'));
const steps = doc.jobs['otp-attempt-cap'].steps;
const testIndex = steps.findIndex((step) => step.run === 'pnpm --dir apps/anvil-api test:neon');
const deleteIndex = steps.findIndex(
  (step) => step.run === 'node scripts/ci/create-neon-test-branch.mjs --delete',
);
[steps[testIndex], steps[deleteIndex]] = [steps[deleteIndex], steps[testIndex]];
fs.writeFileSync(target, yaml.stringify(doc));
NODE

assert_fails_with \
  "${wrong_command_order}" \
  'required command order is checkout, create, test, delete' \
  wrong-command-order

# Negative: the contract supports a closed command grammar. A second shell
# wrapper around the provisioner must not evade exact-command ownership.
wrapped_duplicate_create="${tmp_dir}/wrapped-duplicate-create.yml"
awk '
  /      - name: Prove the OTP attempt cap against Neon/ {
    print "      - name: Wrapped duplicate Neon create"
    print "        run: |"
    print "          set -e"
    print "          node scripts/ci/create-neon-test-branch.mjs"
    print ""
  }
  { print }
' "${workflow}" >"${wrapped_duplicate_create}"

assert_fails_with \
  "${wrapped_duplicate_create}" \
  'unsupported wrapped Neon command' \
  wrapped-duplicate-create

# Negative: GitHub normalises secret names, so case variation cannot hide a
# second consumer from the structural inventory.
lowercase_secret_consumer="${tmp_dir}/lowercase-secret-consumer.yml"
cp "${workflow}" "${lowercase_secret_consumer}"
cat >>"${lowercase_secret_consumer}" <<'YAML'
  lowercase-secret-consumer:
    runs-on: self-hosted
    steps:
      - env:
          NEON_API_KEY: ${{ secrets.neon_test_api_key }}
        run: echo lowercase-secret-consumer
YAML

assert_fails_with \
  "${lowercase_secret_consumer}" \
  'jobs.lowercase-secret-consumer.if is missing the internal-PR guard' \
  lowercase-secret-consumer

# Negative: serialising the whole secrets context includes the Neon key without
# naming it, so this job must be treated as a potential credential consumer.
whole_secrets_consumer="${tmp_dir}/whole-secrets-consumer.yml"
cp "${workflow}" "${whole_secrets_consumer}"
cat >>"${whole_secrets_consumer}" <<'YAML'
  whole-secrets-consumer:
    runs-on: self-hosted
    steps:
      - env:
          ALL_SECRETS: ${{ toJSON(secrets) }}
        run: echo whole-secrets-consumer
YAML

assert_fails_with \
  "${whole_secrets_consumer}" \
  'jobs.whole-secrets-consumer.if is missing the internal-PR guard' \
  whole-secrets-consumer

# Negative: toJSON(Secrets) is the same whole-context leak with GitHub's
# case-insensitive context name.
whole_secrets_context_case="${tmp_dir}/whole-secrets-context-case.yml"
cp "${workflow}" "${whole_secrets_context_case}"
cat >>"${whole_secrets_context_case}" <<'YAML'
  whole-secrets-context-case:
    runs-on: self-hosted
    steps:
      - env:
          ALL_SECRETS: ${{ toJSON(Secrets) }}
        run: echo whole-secrets-context-case
YAML

assert_fails_with \
  "${whole_secrets_context_case}" \
  'jobs.whole-secrets-context-case.if is missing the internal-PR guard' \
  whole-secrets-context-case

# Negative: workflow defaults can replace the command interpreter. This valid
# custom template only prints the generated script path and never executes it.
workflow_noop_shell="${tmp_dir}/workflow-noop-shell.yml"
sed '/^jobs:/i\
defaults:\
  run:\
    shell: echo {0}\
' "${workflow}" >"${workflow_noop_shell}"

assert_fails_with \
  "${workflow_noop_shell}" \
  'workflow defaults.run.shell must not override critical command execution' \
  workflow-noop-shell

# Negative: job defaults have the same inherited effect on all run steps.
job_noop_shell="${tmp_dir}/job-noop-shell.yml"
awk '
  /    timeout-minutes: 15/ {
    print
    print "    defaults:"
    print "      run:"
    print "        shell: echo {0}"
    next
  }
  { print }
' "${workflow}" >"${job_noop_shell}"

assert_fails_with \
  "${job_noop_shell}" \
  'job defaults.run.shell must not override critical command execution' \
  job-noop-shell

# Negative: a critical step can override otherwise-safe defaults with the same
# non-executing custom shell template.
step_noop_shell="${tmp_dir}/step-noop-shell.yml"
awk '
  /      - name: Prove the OTP attempt cap against Neon/ {
    print
    print "        shell: echo {0}"
    next
  }
  { print }
' "${workflow}" >"${step_noop_shell}"

assert_fails_with \
  "${step_noop_shell}" \
  'critical Neon command steps must not override shell' \
  step-noop-shell

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
