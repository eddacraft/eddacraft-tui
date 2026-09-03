#!/usr/bin/env bash
# CIB-396 / issue #4345: lock secret-calibration.yml off LINUX_RUNNER for fork PRs.
#
# DeepSec 20260902184224-6753b67df9c072ab found the advisory calibration job
# scheduled on `vars.LINUX_RUNNER` with no fork ternary, so an external PR
# touching secret-calibration paths could run PR-controlled `cargo test` on
# the org Linux runner. rust-tests.yml and codeql.yml already force forks
# onto ubuntu-latest; this fixture pins the same LINUX_RUNNER-only form.

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
workflow="${repo_root}/.github/workflows/secret-calibration.yml"

if [ ! -f "${workflow}" ]; then
  echo "expected ${workflow} to exist" >&2
  exit 1
fi

assert_contains() {
  local expected="$1"
  if ! grep -Fq -- "${expected}" "${workflow}"; then
    echo "expected ${workflow} to contain: ${expected}" >&2
    exit 1
  fi
}

assert_not_contains() {
  local forbidden="$1"
  if grep -Fq -- "${forbidden}" "${workflow}"; then
    echo "expected ${workflow} not to contain: ${forbidden}" >&2
    exit 1
  fi
}

assert_contains "github.event.pull_request.head.repo.fork && 'ubuntu-latest'"
assert_contains 'vars.LINUX_RUNNER'
assert_not_contains "runs-on: \${{ vars.LINUX_RUNNER || 'ubuntu-latest' }}"

python3 - "${workflow}" <<'PY'
from pathlib import Path
import re
import sys

text = Path(sys.argv[1]).read_text()
expr = re.search(
    r"\$\{\{\s*github\.event\.pull_request\.head\.repo\.fork\s*&&\s*"
    r"'ubuntu-latest'\s*\|\|\s*vars\.LINUX_RUNNER\s*\|\|\s*'ubuntu-latest'\s*\}\}",
    text,
    re.S,
)
if not expr:
    print('secret-calibration.yml is missing the codeql/rust-tests LINUX_RUNNER fork ternary', file=sys.stderr)
    sys.exit(1)
print('secret-calibration fork ternary:', re.sub(r'\s+', ' ', expr.group(0)))
PY

printf 'secret-calibration-workflow.test.sh: ok\n'
