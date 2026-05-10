#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
CHECK=(node "$ROOT/scripts/aps/drift-check.mjs")

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

assert_json_has_code() {
  local json="$1"
  local code="$2"
  printf '%s' "$json" | node -e '
const doc = JSON.parse(require("node:fs").readFileSync(0, "utf8"));
const code = process.argv[1];
if (!doc.advisory || doc.enforcement !== "none") throw new Error("expected warning-mode output");
if (!doc.findings.some((finding) => finding.code === code)) {
  throw new Error(`missing finding code: ${code}\n${JSON.stringify(doc, null, 2)}`);
}
' "$code"
}

write_module() {
  local path="$1"
  mkdir -p "$(dirname "$path")"
  cat > "$path" <<'EOF'
# Fixture Module

| ID | Owner | Status | Progress |
| --- | --- | --- | --- |
| FIX | — | In Progress | 2/4 |

### FIX-001: Complete item

- **Status:** Complete
- **Validation:** `pnpm test`
- **Files:** `src/known.ts`

### FIX-002: Merged item

- **Status:** Merged
- **Validation:** `pnpm test`
- **Files:** `src/merged.ts`

### FIX-003: Shipped item

- **Status:** Released/Shipped
- **Validation:** `pnpm test`
- **Files:** `src/shipped.ts`

### FIX-004: Ready item

- **Status:** Ready
- **Validation:** `pnpm test`
- **Files:** `src/ready.ts`
EOF
}

mkdir -p "$tmp/plans/modules"
write_module "$tmp/plans/modules/fixture.aps.md"
cat > "$tmp/plans/index.aps.md" <<'EOF'
# Fixture Index

| [fixture](./modules/fixture.aps.md) | FIX | 1/4 | Fixture module. |
EOF
cat > "$tmp/package.json" <<'EOF'
{"version":"1.2.3"}
EOF

printf '%s\n' 'src/unknown.ts' > "$tmp/changed-files"
cat > "$tmp/candidate.json" <<'EOF'
{
  "lifecycleState": "candidate",
  "version": "v1.2.3",
  "source": {"tag": "v1.2.3"},
  "aps": {"items": []}
}
EOF
candidate_json="$("${CHECK[@]}" --root "$tmp" --changed-files "$tmp/changed-files" --release-record "$tmp/candidate.json" --json)"
assert_json_has_code "$candidate_json" 'aps-progress-mismatch'
assert_json_has_code "$candidate_json" 'aps-index-progress-mismatch'
assert_json_has_code "$candidate_json" 'aps-complete-without-validation-evidence'
assert_json_has_code "$candidate_json" 'changed-file-without-aps-reference'
assert_json_has_code "$candidate_json" 'candidate-missing-merged-aps-item'
assert_json_has_code "$candidate_json" 'shipped-aps-without-release-record'

cat > "$tmp/published.json" <<'EOF'
{
  "lifecycleState": "published",
  "version": "v1.2.4",
  "source": {"tag": "v1.2.5"},
  "aps": {"items": [{"id":"FIX-003"}]},
  "artifacts": [{"name":"anvil.tar.gz","url":"https://example.test/anvil.tar.gz"}]
}
EOF
published_json="$("${CHECK[@]}" --root "$tmp" --release-record "$tmp/published.json" --json)"
assert_json_has_code "$published_json" 'release-version-tag-mismatch'
assert_json_has_code "$published_json" 'package-version-tag-mismatch'
assert_json_has_code "$published_json" 'release-artifact-missing-integrity'

echo "drift-check.test.sh: ok"
