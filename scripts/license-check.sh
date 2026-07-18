#!/usr/bin/env bash
# license-check.sh — Check production dependencies for blocked licenses.
#
# Blocked: GPL-2.0, GPL-3.0, AGPL-*, SSPL-*, unlicensed
# Allowed: MIT, Apache-2.0, BSD-*, ISC, 0BSD, CC0-1.0, Unlicense, BlueOak-1.0.0
#
# Usage:
#   bash scripts/license-check.sh
#
# Exit codes:
#   0 — all production licenses acceptable
#   1 — blocked license found
#   2 — tool error (pnpm licenses not available, etc.)
#
# Environment:
#   LICENSE_ALLOWLIST — path to JSON allowlist file (default: scripts/license-allowlist.json)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ALLOWLIST="${LICENSE_ALLOWLIST:-$SCRIPT_DIR/license-allowlist.json}"

# Blocked license patterns (POSIX extended regex)
BLOCKED_PATTERNS="(^|[^L])GPL-2\.0|(^|[^L])GPL-3\.0|AGPL-|SSPL-|UNLICENSED"

echo "=== License Compliance Check ==="
echo ""

# Check pnpm is available
if ! command -v pnpm &>/dev/null; then
  echo "ERROR: pnpm not found"
  exit 2
fi

# Get production licenses as JSON
echo "Scanning production dependency licenses..."

# pnpm licenses list fails with ERR_PNPM_MISSING_PACKAGE_INDEX_FILE when
# packages have ignoredBuiltDependencies (e.g. @swc/core, sharp).
# Try pnpm first, then fall back to npx license-checker.
LICENSE_OUTPUT=""
if ! LICENSE_OUTPUT=$(pnpm licenses list --json --prod 2>/dev/null); then
  echo "WARNING: pnpm licenses list --json failed, trying license-checker..."
  # license-checker reads node_modules directly — no store index needed
  LC_OUTPUT=$(npx -y license-checker --production --json 2>/dev/null) || {
    echo "WARNING: license-checker also failed, trying pnpm text mode..."
    LICENSE_OUTPUT_TEXT=$(pnpm licenses list --prod 2>/dev/null) || {
      echo "ERROR: Could not list licenses via any method"
      exit 2
    }
    # Simple grep-based fallback
    VIOLATIONS=$(echo "$LICENSE_OUTPUT_TEXT" | grep -iE "$BLOCKED_PATTERNS" || true)
    if [ -n "$VIOLATIONS" ]; then
      echo ""
      echo "BLOCKED licenses found:"
      echo "$VIOLATIONS"
      echo ""
      exit 1
    fi
    echo "All production licenses OK (text mode)"
    exit 0
  }

  # license-checker returns {"pkg@ver": {"licenses": "MIT", ...}, ...}
  # Extract unique license strings and check for blocked ones
  LC_LICENSES=$(printf '%s' "$LC_OUTPUT" | node -e "
    const data = JSON.parse(require('fs').readFileSync('/dev/stdin', 'utf8'));
    const seen = new Set();
    for (const [pkg, info] of Object.entries(data)) {
      const lic = info.licenses || 'UNKNOWN';
      if (!seen.has(pkg + ':' + lic)) {
        seen.add(pkg + ':' + lic);
        console.log(lic + '\t' + pkg);
      }
    }
  " 2>/dev/null) || {
    echo "ERROR: Failed to parse license-checker output"
    exit 2
  }

  VIOLATION_COUNT=0
  while IFS=$'\t' read -r license pkg; do
    [ -z "$license" ] && continue
    if echo "$license" | grep -qiE "$BLOCKED_PATTERNS"; then
      # Check allowlist
      PKG_NAME=$(echo "$pkg" | sed 's/@[^@]*$//')
      if [ -f "$ALLOWLIST" ]; then
        ALLOWED=$(ALLOWLIST_PATH="$ALLOWLIST" PKG="$PKG_NAME" node -e "
          const data = JSON.parse(require('fs').readFileSync(process.env.ALLOWLIST_PATH, 'utf8'));
          process.exit((data.allowed || []).includes(process.env.PKG) ? 0 : 1);
        " 2>/dev/null) && {
          echo "  ALLOWED (allowlisted): $pkg ($license)"
          continue
        }
      fi
      echo "BLOCKED: $pkg — $license"
      VIOLATION_COUNT=$((VIOLATION_COUNT + 1))
    fi
  done <<< "$LC_LICENSES"

  echo ""
  if [ "$VIOLATION_COUNT" -gt 0 ]; then
    echo "FAIL: $VIOLATION_COUNT package(s) with blocked licenses"
    exit 1
  else
    echo "PASS: All production licenses are acceptable (via license-checker)"
    exit 0
  fi
fi

# Parse JSON output — look for blocked licenses
# Expected pnpm output format (tested with pnpm >= 8):
#   pnpm licenses list --json --prod => JSON object keyed by license type,
#   e.g. { "MIT": [...], "Apache-2.0": [...] }
BLOCKED_FOUND=""

# Extract license names from the JSON keys, validating the structure first
LICENSE_NAMES=$(printf '%s' "$LICENSE_OUTPUT" | node -e "
  const fs = require('node:fs');
  const raw = fs.readFileSync('/dev/stdin', 'utf8');
  let data;
  try {
    data = JSON.parse(raw);
  } catch (e) {
    console.error('ERROR: Failed to parse pnpm licenses JSON:', e.message);
    process.exit(1);
  }

  if (typeof data !== 'object' || data === null || Array.isArray(data)) {
    console.error('ERROR: Unexpected pnpm licenses JSON format. Expected a top-level object keyed by license type.');
    process.exit(1);
  }

  const licenses = Object.keys(data);
  if (licenses.length === 0) {
    console.error('ERROR: pnpm licenses JSON contained no license keys. Output format may have changed.');
    process.exit(1);
  }

  for (const license of licenses) {
    console.log(license);
  }
") || {
  echo "WARNING: JSON parse failed, falling back to grep"
  VIOLATIONS=$(echo "$LICENSE_OUTPUT" | grep -ioE "$BLOCKED_PATTERNS" || true)
  if [ -n "$VIOLATIONS" ]; then
    echo "BLOCKED licenses found: $VIOLATIONS"
    exit 1
  fi
  echo "All production licenses OK (grep fallback)"
  exit 0
}

# Load allowlist if it exists
ALLOWLIST_PACKAGES=""
if [ -f "$ALLOWLIST" ]; then
  ALLOWLIST_PACKAGES=$(ALLOWLIST_PATH="$ALLOWLIST" node -e "
    const data = JSON.parse(require('fs').readFileSync(process.env.ALLOWLIST_PATH, 'utf8'));
    (data.allowed || []).forEach(p => console.log(p));
  " 2>/dev/null || true)
fi

# Check each license
VIOLATION_COUNT=0
while IFS= read -r license; do
  [ -z "$license" ] && continue

  if echo "$license" | grep -qiE "$BLOCKED_PATTERNS"; then
    # Get the packages under this blocked license. Do not feed fallback text
    # through the package parser — a list/parse failure is a tool error.
    if ! PACKAGES=$(printf '%s' "$LICENSE_OUTPUT" | LICENSE_KEY="$license" node -e "
      const data = JSON.parse(require('fs').readFileSync('/dev/stdin', 'utf8'));
      const key = process.env.LICENSE_KEY;
      if (data[key]) {
        data[key].forEach(pkg => console.log('  - ' + pkg.name + '@' + pkg.version));
      }
    " 2>/dev/null); then
      echo "ERROR: could not list packages for blocked license: $license" >&2
      exit 2
    fi

    # Filter out allowlisted packages
    FILTERED_PACKAGES=()
    while IFS= read -r pkg_line; do
      [ -z "$pkg_line" ] && continue
      raw_pkg="${pkg_line#  - }"
      version="${raw_pkg##*@}"
      PKG_NAME="${raw_pkg%"@$version"}"
      if echo "$ALLOWLIST_PACKAGES" | grep -qxF "$PKG_NAME"; then
        echo "  ALLOWED (allowlisted): $pkg_line"
      else
        FILTERED_PACKAGES+=("$pkg_line")
        VIOLATION_COUNT=$((VIOLATION_COUNT + 1))
      fi
    done <<< "$PACKAGES"

    if [ "${#FILTERED_PACKAGES[@]}" -gt 0 ]; then
      echo ""
      echo "BLOCKED license: $license"
      printf '%s\n' "${FILTERED_PACKAGES[@]}"
    fi
  fi
done <<< "$LICENSE_NAMES"

echo ""
if [ "$VIOLATION_COUNT" -gt 0 ]; then
  echo "FAIL: $VIOLATION_COUNT package(s) with blocked licenses"
  echo ""
  echo "To allow a specific package, add it to $ALLOWLIST:"
  echo '  { "allowed": ["package-name"] }'
  exit 1
else
  echo "PASS: All production licenses are acceptable"
  exit 0
fi
