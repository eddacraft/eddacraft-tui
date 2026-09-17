#!/usr/bin/env bash
# ADR-136 §5 — refresh the vendored gitleaks tier-1 ruleset.
#
# Follows the repository idiom for generated artefacts (`expand-licences.sh`,
# `generate-acknowledgements.sh`): regenerate in place by default, verify in
# `--check` mode.
#
#   refresh-gitleaks-ruleset.sh                regenerate from the current pin
#   refresh-gitleaks-ruleset.sh --check        verify committed data, exit 1 on drift
#   refresh-gitleaks-ruleset.sh --upgrade TAG  move the pin to TAG, then regenerate
#
# `--upgrade` peels annotated and nested tags to a commit before downloading
# or rewriting PIN.toml. A lightweight tag is already a commit and is used as
# is. A tag chain that does not reach a commit within MAX_TAG_PEEL hops, or
# that resolves to any other object type, fails loudly.
#
# A vendored asset with no refresh procedure is stale within a year, and stale
# detection rules are worse than absent ones because they are trusted.

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "${SCRIPT_DIR}/../.." && pwd)"
# Tests point this at a throwaway tree so --upgrade never touches the committed pin.
VENDOR_DIR="${ANVIL_GITLEAKS_VENDOR_DIR:-${REPO_ROOT}/crates/anvil-checks/src/secret/vendor/gitleaks}"
PIN_FILE="${VENDOR_DIR}/PIN.toml"
MEMBERSHIP="${VENDOR_DIR}/tier1-rules.txt"
DATA_FILE="${VENDOR_DIR}/tier1.json"
PROVENANCE="${VENDOR_DIR}/PROVENANCE.md"
LICENCE_FILE="${VENDOR_DIR}/LICENSE"
CONVERTER="${SCRIPT_DIR}/convert-gitleaks-rules.py"

# Peel at most this many annotated/nested tag objects. A release tag that still
# has not reached a commit after this many hops is treated as unbound rather
# than followed forever.
MAX_TAG_PEEL=8

usage() {
  sed -n '2,19p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

object_field() {
  python3 -c '
import json, sys
field = sys.argv[1]
try:
    payload = json.load(sys.stdin)
except json.JSONDecodeError as exc:
    raise SystemExit(f"error: GitHub response is not JSON: {exc}") from exc
try:
    value = payload["object"][field]
except (KeyError, TypeError):
    raise SystemExit(f"error: GitHub response is missing object.{field}")
if not isinstance(value, str) or not value:
    raise SystemExit(f"error: GitHub response object.{field} is empty")
print(value)
' "$1"
}

# Resolve a GitHub tag ref to the commit it names. Lightweight tags are already
# commits. Annotated and nested tags are peeled via the Git Database API.
resolve_tag_to_commit() {
  local slug="$1"
  local tag="$2"
  local json type sha
  local peels=0

  json="$(curl -fsSL --max-time 60 "https://api.github.com/repos/${slug}/git/ref/tags/${tag}")"
  type="$(printf '%s' "${json}" | object_field type)"
  sha="$(printf '%s' "${json}" | object_field sha)"

  while [ "${type}" = "tag" ]; do
    peels=$((peels + 1))
    if [ "${peels}" -gt "${MAX_TAG_PEEL}" ]; then
      echo "error: tag '${tag}' chain exceeded ${MAX_TAG_PEEL} peels without reaching a commit" >&2
      return 1
    fi
    json="$(curl -fsSL --max-time 60 "https://api.github.com/repos/${slug}/git/tags/${sha}")"
    type="$(printf '%s' "${json}" | object_field type)"
    sha="$(printf '%s' "${json}" | object_field sha)"
  done

  if [ "${type}" != "commit" ]; then
    echo "error: tag '${tag}' resolved to a ${type} object (${sha}), not a commit" >&2
    return 1
  fi
  printf '%s\n' "${sha}"
}

pin_value() {
  python3 - "$1" <<'PY'
import sys, tomllib, pathlib
key = sys.argv[1]
pin = tomllib.loads(pathlib.Path(__import__("os").environ["PIN_FILE"]).read_text(encoding="utf-8"))
print(pin[key])
PY
}

refresh_main() {
  local MODE="regenerate"
  local UPGRADE_TAG=""
  local SKIP_CALIBRATE="${ANVIL_SKIP_CALIBRATE:-0}"

  while [ "$#" -gt 0 ]; do
    case "$1" in
      --check) MODE="check"; shift ;;
      --upgrade)
        [ "$#" -ge 2 ] || { echo "error: --upgrade needs a tag (e.g. --upgrade v8.30.1)" >&2; exit 2; }
        MODE="upgrade"; UPGRADE_TAG="$2"; shift 2 ;;
      --no-calibrate) SKIP_CALIBRATE=1; shift ;;
      -h|--help) usage; exit 0 ;;
      *) echo "error: unknown argument '$1'" >&2; usage >&2; exit 2 ;;
    esac
  done

  local tool
  for tool in curl python3 sha256sum; do
    command -v "${tool}" >/dev/null 2>&1 || { echo "error: '${tool}' is required" >&2; exit 2; }
  done

  # The pin is TOML and is read with the stdlib `tomllib`, which is Python 3.11+.
  # Checked up front so an older `python3` fails here with the version it has,
  # rather than several steps later with a bare ModuleNotFoundError.
  python3 -c 'import tomllib' 2>/dev/null || {
    echo "error: python3 lacks 'tomllib' (needs Python >= 3.11); found $(python3 -V 2>&1)" >&2
    exit 2
  }

  export PIN_FILE

  local REPO_URL SOURCE_PATH
  REPO_URL="$(pin_value repo)"
  SOURCE_PATH="$(pin_value source_path)"

  # --upgrade is the ONLY path that moves the pin, so a version bump is an
  # explicit, reviewable act rather than a side effect of running the refresh.
  if [ "${MODE}" = "upgrade" ]; then
    echo "==> resolving ${REPO_URL} tag ${UPGRADE_TAG}"
    local slug new_commit tmp_upgrade new_digest
    slug="${REPO_URL#https://github.com/}"
    new_commit="$(resolve_tag_to_commit "${slug}" "${UPGRADE_TAG}")"
    tmp_upgrade="$(mktemp)"
    curl -fsSL --max-time 120 -o "${tmp_upgrade}" \
      "https://raw.githubusercontent.com/${slug}/${new_commit}/${SOURCE_PATH}"
    new_digest="$(sha256sum "${tmp_upgrade}" | cut -d' ' -f1)"
    rm -f "${tmp_upgrade}"
    python3 - "${UPGRADE_TAG}" "${new_commit}" "${new_digest}" <<'PY'
import os, pathlib, re, sys
tag, commit, digest = sys.argv[1:4]
path = pathlib.Path(os.environ["PIN_FILE"])
text = path.read_text(encoding="utf-8")
for key, value in (("tag", tag), ("commit", commit), ("sha256", digest)):
    text = re.sub(rf'(?m)^{key} = ".*"$', f'{key} = "{value}"', text)
path.write_text(text, encoding="utf-8")
PY
    echo "==> pin moved to ${UPGRADE_TAG} (${new_commit})"
    MODE="regenerate"
  fi

  local TAG COMMIT EXPECTED_SHA SLUG
  TAG="$(pin_value tag)"
  COMMIT="$(pin_value commit)"
  EXPECTED_SHA="$(pin_value sha256)"
  SLUG="${REPO_URL#https://github.com/}"

  # WORK is global so the EXIT trap can still see it after refresh_main returns.
  WORK="$(mktemp -d)"
  trap 'rm -rf "${WORK}"' EXIT

  echo "==> fetching ${SLUG}@${COMMIT}:${SOURCE_PATH}"
  curl -fsSL --max-time 120 -o "${WORK}/gitleaks.toml" \
    "https://raw.githubusercontent.com/${SLUG}/${COMMIT}/${SOURCE_PATH}"

  local ACTUAL_SHA
  ACTUAL_SHA="$(sha256sum "${WORK}/gitleaks.toml" | cut -d' ' -f1)"
  if [ "${ACTUAL_SHA}" != "${EXPECTED_SHA}" ]; then
    cat >&2 <<EOF
error: upstream ruleset digest mismatch — ABORTING.
  pinned commit : ${COMMIT}
  expected sha256: ${EXPECTED_SHA}
  actual   sha256: ${ACTUAL_SHA}
The content at the pinned commit changed underneath the pin. Do not "fix" this
by pasting the new digest: establish why the immutable content moved first.
EOF
    exit 1
  fi
  echo "==> digest verified (${EXPECTED_SHA})"

  # The licence path is pinned, not assumed: hardcoding `/LICENSE` would make
  # `PIN.toml` misleading the moment upstream moved the file, and the attribution
  # would silently vendor the wrong text.
  local LICENCE_PATH
  LICENCE_PATH="$(pin_value licence_path)"
  curl -fsSL --max-time 60 -o "${WORK}/LICENSE" \
    "https://raw.githubusercontent.com/${SLUG}/${COMMIT}/${LICENCE_PATH}"

  python3 "${CONVERTER}" \
    --source "${WORK}/gitleaks.toml" \
    --membership "${MEMBERSHIP}" \
    --pin "${PIN_FILE}" \
    --out "${WORK}/tier1.json"

  local RULE_COUNT RETRIEVED
  RULE_COUNT="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["rule_count"])' "${WORK}/tier1.json")"
  RETRIEVED="$(date -u +%Y-%m-%d)"

  # `--check` must not report drift merely because the clock moved, so the
  # retrieval date is carried over from the committed file when verifying.
  if [ "${MODE}" = "check" ] && [ -f "${PROVENANCE}" ]; then
    RETRIEVED="$(sed -n 's/^- \*\*Retrieved:\*\* \(.*\)$/\1/p' "${PROVENANCE}" | head -n1)"
    [ -n "${RETRIEVED}" ] || RETRIEVED="$(date -u +%Y-%m-%d)"
  fi

  cat > "${WORK}/PROVENANCE.md" <<EOF
# Vendored gitleaks ruleset — provenance

<!-- GENERATED by scripts/secret/refresh-gitleaks-ruleset.sh — do not hand-edit. -->

Anvil vendors detection **knowledge as data** from the gitleaks ruleset and
compiles it into its own scanner (ADR-136 §1). No third-party engine, binary, or
runtime enters the product.

A list rather than a table on purpose: a table's column widths change with the
tag and rule count, which would make the formatter rewrite this generated file
and put \`--check\` permanently at odds with \`pnpm format:check\`.

- **Upstream:** ${REPO_URL}
- **Release tag:** ${TAG}
- **Commit:** \`${COMMIT}\`
- **Source file:** \`${SOURCE_PATH}\`
- **SHA-256:**
  \`${EXPECTED_SHA}\`
- **Retrieved:** ${RETRIEVED}
- **Licence:** MIT (see \`LICENSE\` beside this file)
- **Tier:** 1 — high-confidence, prefix-anchored provider rules
- **Rules vendored:** ${RULE_COUNT}

## What "tier 1" means

A tier-1 rule matches a credential carrying a literal, provider-specific prefix,
so a match **is** the credential rather than a guess about one. That is why
tier-1 rules compile with \`high_confidence: true\` and are therefore exempt from
the fuzzy false-positive filters (the \`example\`/\`test\` keyword allowlist and
\`looks_like_code\`) that exist for hand-written, keyword-driven regexes.

Generic and entropy-adjacent rules are **not** here. They are where
false-positive volume lives and they are a later tier with its own measurement.

Anvil's allowlist and suppression layer applies to these rules exactly as it
does to built-ins: shape-anchored allowlist entries and any operator
\`custom_allowlist\` entry still suppress a vendored match, and the suppression is
recorded with \`AllowlistProvenance\` like any other.

## Refreshing

\`\`\`bash
scripts/secret/refresh-gitleaks-ruleset.sh            # regenerate from the pin
scripts/secret/refresh-gitleaks-ruleset.sh --check    # verify, non-zero on drift
scripts/secret/refresh-gitleaks-ruleset.sh --upgrade v8.31.0
\`\`\`

\`--upgrade\` is the only path that moves the pin. Every refresh runs
\`pnpm secret:calibrate\`, so no ruleset change lands unmeasured against the
SDT-002 corpus.

## Attribution

The \`## Thanks\` section of \`ACKNOWLEDGEMENTS.md\` carries the gitleaks entry. It
is **hand-curated**: \`ACKNOWLEDGEMENTS.md\` generates from dependency manifests,
and a vendored data file is a dependency of neither, so no generated gate will
ever notice the entry going missing (ADR-136 §5).
EOF

  if [ "${MODE}" = "check" ]; then
    local status=0
    local pair name committed
    for pair in "tier1.json:${DATA_FILE}" "PROVENANCE.md:${PROVENANCE}" "LICENSE:${LICENCE_FILE}"; do
      name="${pair%%:*}"; committed="${pair#*:}"
      if [ ! -f "${committed}" ]; then
        echo "error: ${committed} is missing" >&2
        status=1
        continue
      fi
      if ! diff -u "${committed}" "${WORK}/${name}" >/dev/null 2>&1; then
        echo "error: ${committed} has drifted from the pinned upstream:" >&2
        diff -u "${committed}" "${WORK}/${name}" >&2 || true
        status=1
      fi
    done
    if [ "${status}" -ne 0 ]; then
      echo "" >&2
      echo "Run scripts/secret/refresh-gitleaks-ruleset.sh and commit the result." >&2
      exit 1
    fi
    echo "==> vendored ruleset matches the pinned upstream"
    exit 0
  fi

  cp "${WORK}/tier1.json" "${DATA_FILE}"
  cp "${WORK}/PROVENANCE.md" "${PROVENANCE}"
  cp "${WORK}/LICENSE" "${LICENCE_FILE}"
  echo "==> wrote ${RULE_COUNT} tier-1 rule(s) to ${DATA_FILE#"${REPO_ROOT}/"}"

  if [ "${SKIP_CALIBRATE}" = "1" ]; then
    echo "==> skipping pnpm secret:calibrate (ANVIL_SKIP_CALIBRATE=1)"
    exit 0
  fi

  # ADR-136 §5: no ruleset change lands unmeasured.
  echo "==> running pnpm secret:calibrate"
  cd "${REPO_ROOT}"
  pnpm secret:calibrate
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  refresh_main "$@"
fi
