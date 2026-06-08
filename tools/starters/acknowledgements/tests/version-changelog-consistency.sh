#!/usr/bin/env bash
# Version / changelog consistency test for the acknowledgements kit.
#
# Exercises `check-version.sh` — the shared checker used by both this
# self-test and the `release-acknowledgements-starter.yml` workflow's
# version-triple assertion — across:
#
#   1.  The REAL kit: its `VERSION` agrees with the newest `## [X.Y.Z]`
#       heading in `CHANGELOG.md` → exit 0. (This is the invariant CI
#       gates: a version bump must update both files.)
#   1b. The REAL kit with a prefixed `--tag` — the exact call the release
#       workflow makes; also covers the default-dir symlink-resolution
#       path the `--dir` fixtures bypass.
#   2.  Fixture where VERSION disagrees with the changelog heading →
#       exit 1, stderr names both versions.
#   3.  Fixture with a malformed VERSION → exit 1, stderr says so.
#   4.  Fixture missing CHANGELOG.md → exit 1, stderr names the file.
#   5.  `--tag` matching VERSION (bare `vX.Y.Z`) → exit 0;
#       a mismatching tag → exit 1.
#   6.  `--tag` in the prefixed source form
#       (`acknowledgements-starter-vX.Y.Z`) is accepted and compared on
#       its `X.Y.Z` component → exit 0.
#   7.  A malformed double-prefixed tag (`…-vvX.Y.Z`) → exit 1.
#
# Local invocation:
#   tools/starters/acknowledgements/tests/version-changelog-consistency.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CHECKER="$SCRIPT_DIR/../check-version.sh"

if [ ! -x "$CHECKER" ]; then
  echo "error: checker not found or not executable at $CHECKER" >&2
  exit 1
fi

fixture_root="$(mktemp -d)"
trap 'rm -rf "$fixture_root"' EXIT

# ── Scenario 1: the real kit is internally consistent ────────────────
exit1=0
out1="$("$CHECKER" 2>&1)" || exit1=$?
if [ "$exit1" -ne 0 ]; then
  echo "fail scenario 1: real kit VERSION/CHANGELOG.md disagree (exit $exit1): $out1" >&2
  exit 1
fi
echo "ok scenario 1: real kit VERSION matches CHANGELOG.md heading"

# ── Scenario 1b: real kit + prefixed --tag (the exact release-workflow
# call). Also exercises the default-dir symlink-resolution path that the
# --dir fixtures below bypass.
real_ver="$(grep -m1 -v '^[[:space:]]*$' "$SCRIPT_DIR/../VERSION" | tr -d '[:space:]')"
exit1b=0
out1b="$("$CHECKER" --tag "acknowledgements-starter-v${real_ver}" 2>&1)" || exit1b=$?
if [ "$exit1b" -ne 0 ]; then
  echo "fail scenario 1b: real kit + prefixed --tag acknowledgements-starter-v${real_ver} rejected (exit $exit1b): $out1b" >&2
  exit 1
fi
echo "ok scenario 1b: real kit + prefixed --tag (release-workflow call) accepted"

# ── Scenario 2: VERSION disagrees with the changelog heading ─────────
d2="$fixture_root/mismatch"
mkdir -p "$d2"
printf '1.2.0\n' >"$d2/VERSION"
printf '# Changelog\n\n## [1.3.0] - 2026-06-08\n\n- thing\n' >"$d2/CHANGELOG.md"
exit2=0
out2="$("$CHECKER" --dir "$d2" 2>&1)" || exit2=$?
if [ "$exit2" -eq 0 ]; then
  echo "fail scenario 2: checker accepted VERSION 1.2.0 vs heading 1.3.0" >&2
  exit 1
fi
if ! printf '%s' "$out2" | grep -q "1.2.0" || ! printf '%s' "$out2" | grep -q "1.3.0"; then
  echo "fail scenario 2: stderr does not name both versions (got: $out2)" >&2
  exit 1
fi
echo "ok scenario 2: VERSION/heading mismatch rejected, names both (exit $exit2)"

# ── Scenario 3: malformed VERSION ────────────────────────────────────
d3="$fixture_root/malformed"
mkdir -p "$d3"
printf 'v1.0\n' >"$d3/VERSION"
printf '# Changelog\n\n## [1.0.0] - 2026-06-08\n' >"$d3/CHANGELOG.md"
exit3=0
out3="$("$CHECKER" --dir "$d3" 2>&1)" || exit3=$?
if [ "$exit3" -eq 0 ]; then
  echo "fail scenario 3: checker accepted malformed VERSION 'v1.0'" >&2
  exit 1
fi
if ! printf '%s' "$out3" | grep -qiE "semver|version"; then
  echo "fail scenario 3: stderr lacks a version/semver hint (got: $out3)" >&2
  exit 1
fi
echo "ok scenario 3: malformed VERSION rejected (exit $exit3)"

# ── Scenario 4: missing CHANGELOG.md ─────────────────────────────────
d4="$fixture_root/no-changelog"
mkdir -p "$d4"
printf '1.0.0\n' >"$d4/VERSION"
exit4=0
out4="$("$CHECKER" --dir "$d4" 2>&1)" || exit4=$?
if [ "$exit4" -eq 0 ]; then
  echo "fail scenario 4: checker accepted a missing CHANGELOG.md" >&2
  exit 1
fi
if ! printf '%s' "$out4" | grep -q "CHANGELOG.md"; then
  echo "fail scenario 4: stderr does not name CHANGELOG.md (got: $out4)" >&2
  exit 1
fi
echo "ok scenario 4: missing CHANGELOG.md rejected (exit $exit4)"

# ── Scenario 5: --tag match / mismatch ───────────────────────────────
d5="$fixture_root/tagged"
mkdir -p "$d5"
printf '2.1.0\n' >"$d5/VERSION"
printf '# Changelog\n\n## [2.1.0] - 2026-06-08\n\n- thing\n' >"$d5/CHANGELOG.md"
exit5=0
out5="$("$CHECKER" --dir "$d5" --tag "v2.1.0" 2>&1)" || exit5=$?
if [ "$exit5" -ne 0 ]; then
  echo "fail scenario 5a: matching --tag v2.1.0 rejected (exit $exit5): $out5" >&2
  exit 1
fi
echo "ok scenario 5a: matching --tag v2.1.0 accepted"

exit5b=0
out5b="$("$CHECKER" --dir "$d5" --tag "v2.2.0" 2>&1)" || exit5b=$?
if [ "$exit5b" -eq 0 ]; then
  echo "fail scenario 5b: mismatching --tag v2.2.0 accepted" >&2
  exit 1
fi
if ! printf '%s' "$out5b" | grep -q "2.2.0" || ! printf '%s' "$out5b" | grep -q "2.1.0"; then
  echo "fail scenario 5b: stderr does not name both tag + VERSION (got: $out5b)" >&2
  exit 1
fi
echo "ok scenario 5b: mismatching --tag rejected, names both (exit $exit5b)"

# ── Scenario 6: prefixed source tag form is accepted ─────────────────
exit6=0
out6="$("$CHECKER" --dir "$d5" --tag "acknowledgements-starter-v2.1.0" 2>&1)" || exit6=$?
if [ "$exit6" -ne 0 ]; then
  echo "fail scenario 6: prefixed --tag acknowledgements-starter-v2.1.0 rejected (exit $exit6): $out6" >&2
  exit 1
fi
echo "ok scenario 6: prefixed source-tag form accepted"

# ── Scenario 7: a malformed double-prefixed tag is rejected ──────────
# `acknowledgements-starter-vv2.1.0` must NOT strip down to a valid
# version — guards the defense-in-depth property of the checker.
exit7=0
out7="$("$CHECKER" --dir "$d5" --tag "acknowledgements-starter-vv2.1.0" 2>&1)" || exit7=$?
if [ "$exit7" -eq 0 ]; then
  echo "fail scenario 7: checker accepted malformed double-prefixed tag '…-vv2.1.0'" >&2
  exit 1
fi
echo "ok scenario 7: malformed double-prefixed tag rejected (exit $exit7)"

echo
echo "version/changelog consistency tests passed: all scenarios green."
