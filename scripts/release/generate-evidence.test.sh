#!/usr/bin/env bash
# Tests for scripts/release/generate-evidence.sh (CIB-034).
# Guards the public/private sanitisation boundary and the render contract.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
gen="${here}/generate-evidence.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

fail=0
check() { # desc, condition-already-evaluated ($? based)
  if [ "$1" -eq 0 ]; then echo "ok  - $2"; else echo "FAIL - $2"; fail=1; fi
}

# A provenance manifest with the SAME shape release.yml emits, including the
# private_build block that MUST NOT reach the rendered evidence.
cat >"$tmp/prov.json" <<'JSON'
{
  "schema_version": "1",
  "release_tag": "v9.9.9-test",
  "built_at": "2026-01-02T03:04:05Z",
  "private_build": {
    "repository": "eddacraft/anvil-001",
    "commit_sha": "abc123def456",
    "ref": "refs/tags/v9.9.9-test",
    "workflow_run_id": "111222333",
    "workflow_run_url": "https://github.com/eddacraft/anvil-001/actions/runs/111222333"
  },
  "public_release": {
    "repository": "eddacraft/anvil",
    "tag": "v9.9.9-test",
    "ref_at_publish": "0000aaaa1111bbbb"
  },
  "assets": [
    { "name": "eddacraft-anvil-x86_64-unknown-linux-gnu.tar.xz", "sha256": "deadbeefcafe", "size_bytes": 42 },
    { "name": "eddacraft-anvil-installer.sh", "sha256": "feedface1234", "size_bytes": 7 }
  ]
}
JSON

out="$tmp/evidence.md"
bash "$gen" --provenance "$tmp/prov.json" --output "$out"
check $? "renders successfully (exit 0)"

# --- Render contract: public-safe content is present.
grep -q "v9.9.9-test" "$out"; check $? "includes the release tag"
grep -q "deadbeefcafe" "$out"; check $? "includes an artefact sha256"
grep -q "eddacraft-anvil-installer.sh" "$out"; check $? "includes an artefact name"
grep -q "0000aaaa1111bbbb" "$out"; check $? "includes the public ref at publish"
grep -q "eddacraft/anvil" "$out"; check $? "includes the public repo"

# --- Sanitisation: private/internal identifiers MUST NOT leak. The disclaimer
# line is excluded because it names these concepts as policy, not as data.
body="$(grep -v 'deliberately omits' "$out")"
! grep -q "anvil-001" <<<"$body"; check $? "omits the private repository name"
! grep -q "actions/runs" <<<"$body"; check $? "omits private workflow-run URLs"
! grep -q "refs/tags" <<<"$body"; check $? "omits the private build ref"

# --- Guard: a manifest with zero assets must refuse to write empty evidence.
jq '.assets = []' "$tmp/prov.json" >"$tmp/empty.json"
if bash "$gen" --provenance "$tmp/empty.json" --output "$tmp/empty.md" 2>/dev/null; then
  check 1 "rejects a zero-asset manifest"
else
  check 0 "rejects a zero-asset manifest"
fi

# --- Guard: a missing provenance file is a usage error (exit non-zero).
if bash "$gen" --provenance "$tmp/nope.json" --output "$tmp/x.md" 2>/dev/null; then
  check 1 "rejects a missing provenance file"
else
  check 0 "rejects a missing provenance file"
fi

if [ "$fail" -ne 0 ]; then
  echo "generate-evidence.test.sh: FAILED"
  exit 1
fi
echo "generate-evidence.test.sh: all checks passed"
