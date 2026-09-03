#!/usr/bin/env bash
# Issue #4349: lock public-reference-regen.yml against tag injection
# and a write-scoped token persisted across pnpm install.
#
# DeepSec 20260902184224-6753b67df9c072ab found two HIGH issues in the
# same privileged workflow:
#   1. rce — github.ref / github.ref_name spliced into the gate `run:` body.
#      Git tag names matching `v[0-9]*` can still carry $() / backticks.
#   2. secrets-exposure — persist-credentials: true stores the write-scoped
#      GITHUB_TOKEN in git config before setup-workspace runs pnpm install.
#
# This fixture asserts the remediations so a later edit cannot silently
# reintroduce either sink.

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
workflow="${repo_root}/.github/workflows/public-reference-regen.yml"

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

assert_contains 'persist-credentials: false'
assert_not_contains 'persist-credentials: true'

assert_contains 'EVENT_NAME: ${{ github.event_name }}'
assert_contains 'GIT_REF: ${{ github.ref }}'
assert_contains 'TAG_NAME: ${{ github.ref_name }}'
assert_contains '[[ "${TAG_NAME}" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9.]+)?$ ]]'

assert_contains 'GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}'
assert_contains 'gh auth setup-git'

tag_re='^v[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9.]+)?$'
assert_tag_match() {
  local tag="$1"
  if ! [[ "${tag}" =~ ${tag_re} ]]; then
    echo "expected tag ${tag} to match the release-tag pattern" >&2
    exit 1
  fi
}
assert_tag_reject() {
  local tag="$1"
  if [[ "${tag}" =~ ${tag_re} ]]; then
    echo "expected tag ${tag} to be rejected by the release-tag pattern" >&2
    exit 1
  fi
}
assert_tag_match 'v0.9.7-beta'
assert_tag_match 'v0.8.2-beta'
assert_tag_match 'v1.0.0'
assert_tag_reject 'v1$(whoami)'
assert_tag_reject 'v0.9.3-beta;id'
assert_tag_reject 'v1`id`'
assert_tag_reject 'v1'

# Interpolation sinks: github.ref / github.ref_name / github.event_name must
# not appear in `run:` script bodies (including comments — Actions expands
# ${{ }} before the shell sees the file). Env-mapping those values is the
# required pattern; YAML `if:` / `env:` / `concurrency:` interpolations are
# not shell injection.
if ! python3 - "${workflow}" <<'PY'
import re
import sys
from pathlib import Path

sink = re.compile(r"\$\{\{\s*github\.(ref|ref_name|event_name)\b")
run_block = re.compile(r"run:\s*[|>][+-]?\s*$")


def scan(text):
    in_run = False
    run_indent = 0
    hits = []
    for i, line in enumerate(text.splitlines(), 1):
        stripped = line.lstrip(" ")
        indent = len(line) - len(stripped)
        if run_block.match(stripped):
            in_run = True
            run_indent = indent
            continue
        if re.match(r"run:\s+\S", stripped) and not run_block.match(stripped):
            if sink.search(line):
                hits.append(i)
            in_run = False
            continue
        if in_run:
            if stripped == "":
                continue
            if indent > run_indent:
                if sink.search(line):
                    hits.append(i)
            else:
                in_run = False
    return hits


probes = {
    "block-pipe": "      - name: x\n        run: |\n          echo ${{ github.ref }}\n",
    "block-strip": "      - name: x\n        run: |-\n          echo ${{ github.ref_name }}\n",
    "block-fold": "      - name: x\n        run: >\n          echo ${{ github.event_name }}\n",
    "single-line": '      - name: x\n        run: echo "${{ github.ref }}"\n',
}
for name, body in probes.items():
    if not scan(body):
        print(f"scanner missed interpolation in {name}", file=sys.stderr)
        sys.exit(1)

hits = scan(Path(sys.argv[1]).read_text())
if hits:
    print(
        "interpolated github.ref/ref_name/event_name in run: bodies at lines:",
        hits,
        file=sys.stderr,
    )
    sys.exit(1)
PY
then
  exit 1
fi

printf 'public-reference-regen-workflow.test.sh: ok\n'
