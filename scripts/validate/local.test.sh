#!/usr/bin/env bash
# Fixture tests for the CICD local validation command surface.

set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
validator="${script_dir}/local.sh"

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required" >&2
  exit 2
fi

tmp_dir=$(mktemp -d)
cleanup() {
  rm -rf "${tmp_dir}"
}
trap cleanup EXIT

paths_file="${tmp_dir}/changed.paths"
printf '%s\n' \
  'docs/guides/testing.md' \
  'packages/anvil-core/src/index.ts' \
  'crates/anvil-cli/src/main.rs' \
  'policies/fixtures/security.rego' \
  'scripts/validate/local.sh' \
  >"${paths_file}"

plan=$(bash "${validator}" --changed --paths-file "${paths_file}" --dry-run --json)

jq -e '.mode == "changed"' >/dev/null <<<"${plan}"
jq -e '.classification.pathClasses | index("docs")' >/dev/null <<<"${plan}"
jq -e '.classification.pathClasses | index("ts")' >/dev/null <<<"${plan}"
jq -e '.classification.pathClasses | index("rust")' >/dev/null <<<"${plan}"
jq -e '.classification.pathClasses | index("policy")' >/dev/null <<<"${plan}"
jq -e '.commands | index("pnpm format:check")' >/dev/null <<<"${plan}"
jq -e '.commands | index("pnpm lint:check")' >/dev/null <<<"${plan}"
jq -e '.commands | index("pnpm typecheck")' >/dev/null <<<"${plan}"
jq -e '.commands | index("pnpm test")' >/dev/null <<<"${plan}"
jq -e '.commands | index("pnpm test:ci-classify")' >/dev/null <<<"${plan}"
jq -e '.commands | index("pnpm test:ci-cost")' >/dev/null <<<"${plan}"
jq -e '.commands | index("pnpm test:ci-fast-pr")' >/dev/null <<<"${plan}"
jq -e '.commands | index("pnpm test:validate-local")' >/dev/null <<<"${plan}"
jq -e '.commands | index("pnpm -s -F @eddacraft/anvil-docs-meta build")' >/dev/null <<<"${plan}"
jq -e '.commands[] | select(startswith("node scripts/docs/check-diagram-impact.mjs --paths-file "))' >/dev/null <<<"${plan}"
jq -e '
  (.commands | index("pnpm -s -F @eddacraft/anvil-docs-meta build")) <
  (.commands | map(startswith("node scripts/docs/check-diagram-impact.mjs --paths-file ")) | index(true))
' >/dev/null <<<"${plan}"
jq -e '.commands[] | select(contains("scripts/validate/local.sh"))' >/dev/null <<<"${plan}"
jq -e '.commands | index("cargo test --workspace")' >/dev/null <<<"${plan}"
jq -e '.commands | index("opa test --verbose policies/fixtures/")' >/dev/null <<<"${plan}"
# DEVENV-007: a TS source change selects the E2E harness surface (mirrors CI).
jq -e '.commands | index("pnpm --filter @eddacraft/anvil-e2e test")' >/dev/null <<<"${plan}"

dependency_paths="${tmp_dir}/dependency.paths"
printf '%s\n' 'pnpm-lock.yaml' >"${dependency_paths}"
dependency_plan=$(bash "${validator}" --changed --paths-file "${dependency_paths}" --dry-run --json)
jq -e '.commands[] | select(contains("trivy is required"))' >/dev/null <<<"${dependency_plan}"

release_paths="${tmp_dir}/release.paths"
printf '%s\n' 'scripts/release/tag.sh' >"${release_paths}"
release_plan=$(bash "${validator}" --changed --paths-file "${release_paths}" --dry-run --json)
jq -e '.commands[] | select(. == "bash -n scripts/release/*.sh")' >/dev/null <<<"${release_plan}"

full=$(bash "${validator}" --full --dry-run --json)
jq -e '.mode == "full"' >/dev/null <<<"${full}"
jq -e '.commands | index("pnpm format:check")' >/dev/null <<<"${full}"
jq -e '.commands | index("pnpm lint:check")' >/dev/null <<<"${full}"
jq -e '.commands | index("pnpm typecheck")' >/dev/null <<<"${full}"
jq -e '.commands | index("pnpm test")' >/dev/null <<<"${full}"
jq -e '.commands | index("pnpm test:ci-classify")' >/dev/null <<<"${full}"
jq -e '.commands | index("pnpm test:ci-cost")' >/dev/null <<<"${full}"
jq -e '.commands | index("pnpm test:ci-fast-pr")' >/dev/null <<<"${full}"
jq -e '.commands | index("pnpm test:validate-local")' >/dev/null <<<"${full}"
jq -e '.commands | index("cargo test --workspace")' >/dev/null <<<"${full}"
jq -e '.commands | index("opa test --verbose policies/fixtures/")' >/dev/null <<<"${full}"
jq -e '.commands | index("regal lint policies/fixtures/")' >/dev/null <<<"${full}"

empty_paths="${tmp_dir}/empty.paths"
: >"${empty_paths}"
empty=$(bash "${validator}" --changed --paths-file "${empty_paths}" --dry-run --json)
jq -e '.commands == []' >/dev/null <<<"${empty}"

dev_null=$(bash "${validator}" --changed --paths-file /dev/null --dry-run --json)
jq -e '.commands == []' >/dev/null <<<"${dev_null}"

semantic_only_paths="${tmp_dir}/semantic-only.paths"
printf '%s\n' 'docs/guides/testing.md' >"${semantic_only_paths}"
semantic_only=$(bash "${validator}" --changed --paths-file "${semantic_only_paths}" --dry-run --json)
jq -e '.classification.requiredChecks | index("diagram-impact")' >/dev/null <<<"${semantic_only}"
jq -e '.commands[] | select(startswith("node scripts/docs/check-diagram-impact.mjs --paths-file "))' \
  >/dev/null <<<"${semantic_only}"

agent_shell_paths="${tmp_dir}/agent-shell.paths"
printf '%s\n' 'scripts/agent/guidance.sh' >"${agent_shell_paths}"
agent_shell=$(bash "${validator}" --changed --paths-file "${agent_shell_paths}" --dry-run --json)
jq -e '.commands[] | select(contains("scripts/agent/guidance.sh"))' >/dev/null <<<"${agent_shell}"

mkdir -p "${tmp_dir}/scripts"
printf '%s\n' 'if then' >"${tmp_dir}/scripts/bad.sh"
printf '%s\n' '#!/usr/bin/env bash' 'exit 0' >"${tmp_dir}/scripts/good.sh"
syntax_paths="${tmp_dir}/syntax.paths"
printf '%s\n' 'scripts/bad.sh' 'scripts/good.sh' >"${syntax_paths}"
syntax_plan=$(bash "${validator}" --changed --paths-file "${syntax_paths}" --dry-run --json)
syntax_command=$(jq -r '.commands[] | select(contains("bash -n"))' <<<"${syntax_plan}")
if (cd "${tmp_dir}" && bash -lc "${syntax_command}"); then
  echo 'expected shell syntax command to fail on the first invalid script' >&2
  exit 1
fi

if bash "${validator}" --changed --paths-file "${tmp_dir}/missing.paths" --dry-run --json >/dev/null 2>&1; then
  echo 'expected missing paths file to fail' >&2
  exit 1
fi

create_deletion_fixture() {
  local root="$1"
  mkdir -p \
    "${root}/bin" \
    "${root}/scripts/ci" \
    "${root}/scripts/docs" \
    "${root}/crates/example/src"
  cat >"${root}/bin/pnpm" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "$*" != "-s -F @eddacraft/anvil-docs-meta build" ]]; then
  printf 'unexpected pnpm fixture command: %s\n' "$*" >&2
  exit 2
fi
printf 'built\n' >docs-meta-built.marker
EOF
  printf 'export PATH=%q/bin:$PATH\n' "${root}" >"${root}/bash-env"
  cat >"${root}/scripts/ci/classify-changes.sh" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' '{"paths":[],"pathClasses":[],"requiredChecks":["diagram-impact"],"warnings":[]}'
EOF
  cat >"${root}/scripts/docs/check-diagram-impact.mjs" <<'EOF'
import { readFileSync, writeFileSync } from 'node:fs';

const index = process.argv.indexOf('--paths-file');
if (index === -1 || !process.argv[index + 1]) process.exit(2);
writeFileSync('diagram-checker-ran.marker', 'ran');
const paths = readFileSync(process.argv[index + 1], 'utf8').split(/\r?\n/u);
if (paths.includes('crates/example/src/lib.rs')) {
  process.stderr.write('deleted exact declared upstream retained\n');
  process.exit(1);
}
EOF
  chmod +x "${root}/bin/pnpm" "${root}/scripts/ci/classify-changes.sh"
  printf '%s\n' 'export const value = 1;' >"${root}/crates/example/src/lib.rs"
  git -C "${root}" init --quiet
  git -C "${root}" add .
  git -C "${root}" -c user.name='DOCRB test' -c user.email='docrb@example.invalid' \
    commit --quiet -m base
}

deletion_failures=0
changed_root="${tmp_dir}/changed-deletion"
create_deletion_fixture "${changed_root}"
rm "${changed_root}/crates/example/src/lib.rs"
git -C "${changed_root}" add -A
git -C "${changed_root}" -c user.name='DOCRB test' -c user.email='docrb@example.invalid' \
  commit --quiet -m 'delete upstream'
if (
  cd "${changed_root}" &&
    BASH_ENV="${changed_root}/bash-env" PATH="${changed_root}/bin:${PATH}" \
      VALIDATE_BASE_REF=HEAD~1 \
      bash "${validator}" --changed >validation.log 2>&1
); then
  echo 'expected automatic changed validation to retain the deleted upstream and fail' >&2
  deletion_failures=$((deletion_failures + 1))
fi
if [[ ! -f "${changed_root}/docs-meta-built.marker" || ! -f "${changed_root}/diagram-checker-ran.marker" ]]; then
  cat "${changed_root}/validation.log" >&2
  echo 'expected automatic changed validation to build docs-meta and reach the diagram checker' >&2
  deletion_failures=$((deletion_failures + 1))
fi

staged_root="${tmp_dir}/staged-deletion"
create_deletion_fixture "${staged_root}"
rm "${staged_root}/crates/example/src/lib.rs"
git -C "${staged_root}" add -A
if (
  cd "${staged_root}" &&
    BASH_ENV="${staged_root}/bash-env" PATH="${staged_root}/bin:${PATH}" \
      bash "${validator}" --staged >validation.log 2>&1
); then
  echo 'expected automatic staged validation to retain the deleted upstream and fail' >&2
  deletion_failures=$((deletion_failures + 1))
fi
if [[ ! -f "${staged_root}/docs-meta-built.marker" || ! -f "${staged_root}/diagram-checker-ran.marker" ]]; then
  cat "${staged_root}/validation.log" >&2
  echo 'expected automatic staged validation to build docs-meta and reach the diagram checker' >&2
  deletion_failures=$((deletion_failures + 1))
fi

if ((deletion_failures > 0)); then
  exit 1
fi

echo 'local validation fixtures passed'
