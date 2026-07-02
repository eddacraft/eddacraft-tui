#!/usr/bin/env bash
# Behavioural tests for the "Security Summary" merge gate (CIB-155).
#
# The summary job is the one security context branch protection requires, so
# its conclusion MUST fail when a blocking scan (Dependency Audit, Secret Scan,
# License Compliance) did not pass. This test does NOT reimplement the gate: it
# EXTRACTS the real github-script body from security.yml, substitutes fixture
# values for the `${{ needs.*.result }}` interpolations, stubs the Actions
# `github`/`context`/`core` objects, and executes it per fixture — so there is
# exactly one copy of the decision logic (the workflow's) and a regression of
# the fail-closed skip contract is caught here.
#
# The run ends with a mutation self-check: it neuters the skip-contract line in
# a scratch copy of the workflow and asserts the extracted gate then goes green
# on the contract-violation fixture, proving this test is sensitive to that
# exact line.

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
default_workflow="${repo_root}/.github/workflows/security.yml"

fail() {
  echo "FAIL: $1" >&2
  exit 1
}

tmp_dir=$(mktemp -d)
cleanup() {
  rm -rf "${tmp_dir}"
}
trap cleanup EXIT

# extract_script <workflow.yml> — print the summary job's github-script body.
extract_script() {
  python3 - "$1" <<'PY'
import sys
import yaml

wf = yaml.safe_load(open(sys.argv[1]))
summary = wf["jobs"]["summary"]
script = None
for step in summary.get("steps", []):
    if "github-script" in step.get("uses", ""):
        script = step["with"]["script"]
        break
if script is None:
    sys.stderr.write("could not find github-script step in summary job\n")
    sys.exit(2)
sys.stdout.write(script)
PY
}

# run_gate <script-body-file> — for each fixture, print "NAME<TAB>PASS|FAIL"
# where FAIL means the extracted script called core.setFailed. Pure evaluator:
# always exits 0, the bash layer asserts the outcomes.
run_gate() {
  node - "$1" <<'NODE'
const fs = require('fs');
const rawBody = fs.readFileSync(process.argv[2], 'utf8');

// (name, detect, semgrep, audit, secrets, license)
const fixtures = [
  ['all-pass', 'success', 'success', 'success', 'success', 'success'],
  ['one-failure-audit', 'success', 'success', 'failure', 'success', 'success'],
  ['one-failure-secrets', 'success', 'success', 'success', 'failure', 'success'],
  ['one-failure-license', 'success', 'success', 'success', 'success', 'failure'],
  ['path-gated-skip', 'success', 'skipped', 'skipped', 'success', 'skipped'],
  ['cancelled', 'success', 'success', 'cancelled', 'success', 'success'],
  ['detect-fail-full-sweep-pass', 'failure', 'success', 'success', 'success', 'success'],
  ['detect-fail-skip-violation', 'failure', 'success', 'skipped', 'success', 'success'],
  // Semgrep is deliberately non-blocking: a failing semgrep result must NOT
  // fail the summary (its findings are swallowed by continue-on-error).
  ['semgrep-failure-non-blocking', 'success', 'failure', 'success', 'success', 'success'],
];

function interpolate(body, f) {
  const [, detect, semgrep, audit, secrets, license] = f;
  const values = {
    'detect-changes': detect,
    semgrep,
    'dependency-audit': audit,
    'secret-scan': secrets,
    'license-check': license,
  };
  const out = body.replace(
    /\$\{\{\s*needs\.([a-z-]+)\.result\s*\}\}/g,
    (m, job) => {
      if (!(job in values)) throw new Error(`unmapped needs job: ${job}`);
      return values[job];
    },
  );
  if (out.includes('${{')) {
    throw new Error(`unsubstituted template remains: ${out.match(/\$\{\{[^}]*\}?\}?/)}`);
  }
  return out;
}

async function evaluate(body) {
  let failedMessage = null;
  const core = {
    setFailed: (m) => {
      failedMessage = m;
    },
  };
  const github = {
    rest: {
      issues: {
        listComments: async () => ({ data: [] }),
        createComment: async () => ({}),
        updateComment: async () => ({}),
      },
    },
  };
  const context = {
    repo: { owner: 'owner', repo: 'repo' },
    issue: { number: 1 },
    runId: 1,
    serverUrl: 'https://example.test',
  };
  // eslint-disable-next-line no-new-func
  const runner = new Function(
    'github',
    'context',
    'core',
    `return (async () => {\n${body}\n})();`,
  );
  await runner(github, context, core);
  return failedMessage;
}

(async () => {
  for (const f of fixtures) {
    const body = interpolate(rawBody, f);
    const msg = await evaluate(body);
    process.stdout.write(`${f[0]}\t${msg === null ? 'PASS' : 'FAIL'}\n`);
  }
})().catch((err) => {
  process.stderr.write(`harness error: ${err.stack || err}\n`);
  process.exit(3);
});
NODE
}

# Expected outcome per fixture name.
expected_outcome() {
  case "$1" in
    all-pass) echo PASS ;;
    one-failure-audit) echo FAIL ;;
    one-failure-secrets) echo FAIL ;;
    one-failure-license) echo FAIL ;;
    path-gated-skip) echo PASS ;;
    cancelled) echo FAIL ;;
    detect-fail-full-sweep-pass) echo PASS ;;
    detect-fail-skip-violation) echo FAIL ;;
    semgrep-failure-non-blocking) echo PASS ;;
    *) fail "no expectation for fixture: $1" ;;
  esac
}

# --- Stage 1: gate the REAL workflow against every fixture -------------------

script_body="${tmp_dir}/script-real.js"
extract_script "${default_workflow}" >"${script_body}"

real_results="${tmp_dir}/results-real.txt"
run_gate "${script_body}" >"${real_results}"

seen=0
while IFS=$'\t' read -r name outcome; do
  want=$(expected_outcome "${name}")
  if [ "${outcome}" != "${want}" ]; then
    fail "fixture ${name}: got ${outcome}, want ${want}"
  fi
  echo "ok   ${name} -> ${outcome}"
  seen=$((seen + 1))
done <"${real_results}"

[ "${seen}" -eq 9 ] || fail "expected 9 fixtures, evaluated ${seen}"
echo 'security summary gate: all fixtures matched expectation on the real workflow'

# --- Stage 2: mutation self-check -------------------------------------------
# Neuter the fail-closed skip-contract decision line in a scratch copy and
# confirm the extracted gate then goes green on the contract-violation fixture.
# If the mutation does NOT flip that outcome, this test cannot detect the
# regression it exists to catch — so treat that as a hard failure.

mutant="${tmp_dir}/security.mutated.yml"
cp "${default_workflow}" "${mutant}"

contract_line="if (result === 'skipped' && detect !== 'success') return true;"
grep -Fq -- "${contract_line}" "${mutant}" ||
  fail "skip-contract line not found in workflow — did the gate logic change shape?"

# Replace the decision with a no-op so a skip is never treated as an offender.
python3 - "${mutant}" "${contract_line}" <<'PY'
import sys

path, needle = sys.argv[1], sys.argv[2]
lines = open(path).read().splitlines(keepends=True)
for i, ln in enumerate(lines):
    if needle in ln:
        indent = ln[: len(ln) - len(ln.lstrip())]
        lines[i] = f"{indent}return false; // MUTATED: skip-contract neutered\n"
        break
else:
    sys.stderr.write("mutation target not found\n")
    sys.exit(2)
open(path, "w").write("".join(lines))
PY

mutant_body="${tmp_dir}/script-mutant.js"
extract_script "${mutant}" >"${mutant_body}"

mutant_results="${tmp_dir}/results-mutant.txt"
run_gate "${mutant_body}" >"${mutant_results}"

mutant_outcome=$(awk -F'\t' '$1 == "detect-fail-skip-violation" { print $2 }' "${mutant_results}")
if [ "${mutant_outcome}" != "PASS" ]; then
  fail "mutation self-check: neutering the skip-contract line did NOT flip detect-fail-skip-violation to PASS (got '${mutant_outcome}') — this test is insensitive to the very line CIB-155 enforces"
fi

echo "ok   mutation self-check: neutering the skip-contract line flips detect-fail-skip-violation FAIL -> PASS (test is sensitive to it)"

echo 'security summary gate checks passed'
