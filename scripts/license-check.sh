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
LICENSE_OUTPUT=$(pnpm licenses list --json --prod 2>/dev/null) || {
  echo "WARNING: pnpm licenses list failed, falling back to text mode"
  LICENSE_OUTPUT_TEXT=$(pnpm licenses list --prod 2>/dev/null) || {
    echo "ERROR: Could not list licenses"
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

# Parse JSON output — look for blocked licenses
# pnpm licenses list --json returns an object keyed by license type
BLOCKED_FOUND=""

# Extract license names from the JSON keys
LICENSE_NAMES=$(echo "$LICENSE_OUTPUT" | node -e "
  const data = JSON.parse(require('fs').readFileSync('/dev/stdin', 'utf8'));
  for (const license of Object.keys(data)) {
    console.log(license);
  }
" 2>/dev/null) || {
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
    # Get the packages under this blocked license
    PACKAGES=$(echo "$LICENSE_OUTPUT" | LICENSE_KEY="$license" node -e "
      const data = JSON.parse(require('fs').readFileSync('/dev/stdin', 'utf8'));
      const key = process.env.LICENSE_KEY;
      if (data[key]) {
        data[key].forEach(pkg => console.log('  - ' + pkg.name + '@' + pkg.version));
      }
    " 2>/dev/null || echo "  (could not list packages)")

    # Filter out allowlisted packages
    FILTERED=""
    while IFS= read -r pkg_line; do
      [ -z "$pkg_line" ] && continue
      PKG_NAME=$(echo "$pkg_line" | sed 's/^  - //' | sed 's/@[^@]*$//')
      if echo "$ALLOWLIST_PACKAGES" | grep -qxF "$PKG_NAME"; then
        echo "  ALLOWED (allowlisted): $pkg_line"
      else
        FILTERED="${FILTERED}${pkg_line}\n"
        VIOLATION_COUNT=$((VIOLATION_COUNT + 1))
      fi
    done <<< "$PACKAGES"

    if [ -n "$FILTERED" ]; then
      echo ""
      echo "BLOCKED license: $license"
      echo -e "$FILTERED"
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
