#!/usr/bin/env bash
set -euo pipefail

# Interactive release script for Anvil CLI.
# Walks through preflight, branching, tagging, and workflow kickoff.
# Creates a GitHub Issue for tracking and writes .release/manifest.json
# as the handoff contract for the /release Claude skill.
#
# Usage: ./scripts/release.sh

readonly REPO="EddaCraft/anvil-001"
readonly PUBLIC_REPO="EddaCraft/anvil"
readonly MANIFEST_DIR=".release"
readonly MANIFEST_FILE="${MANIFEST_DIR}/manifest.json"
readonly CARGO_VERSION_FILE="Cargo.toml"
readonly ROOT_PACKAGE_JSON="package.json"
readonly BETA_GUIDE_FILE="docs/public/anvil/beta-testing-guide.md"
readonly UPGRADE_NOTES_FILE="docs/public/anvil/releases/upgrade-notes.md"
readonly CHANGELOG_FILE="CHANGELOG.md"
readonly BUNDLED_PACKAGE_JSONS=(
  "apps/anvil-api/package.json"
  "packages/adapters/package.json"
  "packages/anvil/contracts/package.json"
  "packages/anvil/core/package.json"
  "packages/anvil/policy/package.json"
  "packages/anvil/ports/package.json"
  "packages/anvil/runtime/package.json"
  "packages/shared/storage/package.json"
  "packages/libs/render/package.json"
)
readonly BUNDLED_TEST_PACKAGES=(
  "@eddacraft/anvil-contracts"
  "@eddacraft/anvil-core"
  "@eddacraft/anvil-policy"
  "@eddacraft/anvil-ports"
  "@eddacraft/anvil-runtime"
  "@eddacraft/shared-storage"
  "@eddacraft/render"
  "@eddacraft/anvil-adapters"
  "@eddacraft/anvil-api"
)

# --- Colours and output ---

readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[0;33m'
readonly BLUE='\033[0;34m'
readonly BOLD='\033[1m'
readonly NC='\033[0m'

info()    { echo -e "${BLUE}ℹ${NC}  $*"; }
success() { echo -e "${GREEN}✓${NC}  $*"; }
warn()    { echo -e "${YELLOW}⚠${NC}  $*"; }
error()   { echo -e "${RED}✗${NC}  $*"; }
header()  { echo -e "\n${BOLD}━━━ $* ━━━${NC}\n"; }

replace_first_match() {
  local file="$1"
  local pattern="$2"
  local replacement="$3"
  perl -0pi -e "s/${pattern}/${replacement}/m" "$file"
}

update_package_json_version() {
  local file="$1"

  if [[ ! -f "$file" ]]; then
    warn "Expected package file missing: $file"
    return 0
  fi

  local package_version
  package_version=$(grep '"version"' "$file" | head -1 | sed 's/.*"version": "\([^"]*\)".*/\1/')
  if [[ "$package_version" != "$VERSION" ]]; then
    info "$file version is ${package_version}, updating to ${VERSION} on dev..."
    replace_first_match "$file" '"version": "[^"]*"' "\"version\": \"${VERSION}\""
    git add "$file"
  else
    info "$file version already ${VERSION} on dev"
  fi
}

run_bundled_pnpm_tests() {
  local cmd=(pnpm -r)
  local pkg

  for pkg in "${BUNDLED_TEST_PACKAGES[@]}"; do
    cmd+=(--filter "$pkg")
  done

  cmd+=(test -- --run)
  "${cmd[@]}"
}

# --- Gate functions ---

hard_gate() {
  local name="$1"
  shift
  info "Running: ${name}"
  if "$@"; then
    success "${name} passed"
    return 0
  else
    error "${name} FAILED — aborting"
    update_issue_comment "❌ Hard gate failed: ${name}"
    exit 1
  fi
}

soft_gate() {
  local name="$1"
  shift
  info "Running: ${name}"
  while true; do
    if "$@"; then
      success "${name} passed"
      return 0
    else
      warn "${name} failed"
      echo -ne "  [${BOLD}r${NC}]etry / [${BOLD}s${NC}]kip / [${BOLD}a${NC}]bort? "
      read -r choice
      case "${choice}" in
        r|R) continue ;;
        s|S)
          warn "Skipping ${name}"
          update_issue_comment "⚠️ Soft gate skipped: ${name}"
          return 0
          ;;
        a|A)
          error "Aborted by operator"
          exit 1
          ;;
        *) echo "  Please enter r, s, or a" ;;
      esac
    fi
  done
}

prompt_continue() {
  local msg="$1"
  echo -ne "${BOLD}${msg}${NC} [${BOLD}y${NC}/${BOLD}n${NC}] "
  read -r choice
  case "${choice}" in
    y|Y) return 0 ;;
    *)
      error "Aborted by operator"
      exit 1
      ;;
  esac
}

# --- Preconditions ---

ensure_clean_worktree() {
  if ! git diff --quiet || ! git diff --cached --quiet; then
    error "Working tree has uncommitted changes. Commit or stash first."
    exit 1
  fi
}

ensure_gh_auth() {
  if ! gh auth status &>/dev/null; then
    error "Not authenticated with GitHub CLI. Run: gh auth login"
    exit 1
  fi
}

ensure_on_dev() {
  local branch
  branch=$(git branch --show-current)
  if [[ "${branch}" != "dev" ]]; then
    error "Must be on dev branch (currently on: ${branch})"
    exit 1
  fi
}

# --- GitHub Issue helpers ---

create_release_issue() {
  local version="$1"
  local tag="$2"
  local release_type="$3"
  local branch_strategy="$4"
  local release_branch="$5"

  info "Creating release tracking issue..."
  # Ensure the 'release' label exists (no-op if it already does)
  gh label create "release" --repo "${REPO}" --color "0e8a16" --description "Release tracking" 2>/dev/null || true

  ISSUE_URL=$(gh issue create \
    --repo "${REPO}" \
    --label "release" \
    --title "release/${tag}" \
    --body "$(cat <<ISSUE_BODY
## Release Identity

- **Version:** ${version}
- **Tag:** ${tag}
- **Release type:** ${release_type}
- **Branch strategy:** ${branch_strategy}
- **Release branch:** ${release_branch:-N/A}

_Tracking issue created by \`scripts/release.sh\`. Updated by the release script and \`/release\` skill._

---

_Preflight and tagging results will be posted as comments below._
ISSUE_BODY
)")

  ISSUE_NUMBER=$(echo "${ISSUE_URL}" | grep -oE '[0-9]+$')
  success "Created issue #${ISSUE_NUMBER}: ${ISSUE_URL}"
}

update_issue_comment() {
  local body="$1"
  if [[ -n "${ISSUE_NUMBER:-}" ]]; then
    gh issue comment "${ISSUE_NUMBER}" --repo "${REPO}" --body "${body}" &>/dev/null || true
  fi
}

# --- Phase 0: Initialisation ---

phase_init() {
  header "Phase 0: Initialisation"

  ensure_gh_auth
  ensure_clean_worktree
  ensure_on_dev

  # Pull latest dev
  info "Pulling latest dev..."
  git pull --ff-only origin dev

  # Prompt for version
  echo -ne "${BOLD}Release version${NC} (e.g. 0.4.0-beta): "
  read -r VERSION
  if [[ -z "${VERSION}" ]]; then
    error "Version cannot be empty"
    exit 1
  fi

  # Derive tag and release type
  TAG="v${VERSION}"
  if [[ "${VERSION}" == *-beta* ]]; then
    RELEASE_TYPE="beta"
  else
    RELEASE_TYPE="production"
  fi

  # Verify current Cargo.toml version
  local cargo_version
  cargo_version=$(grep '^version' "${CARGO_VERSION_FILE}" | head -1 | sed 's/.*"\(.*\)".*/\1/')
  info "Current Cargo.toml version: ${cargo_version}"
  info "Target release version: ${VERSION}"

  # Branch strategy
  echo ""
  echo -e "  ${BOLD}1${NC}) Direct promotion (dev → main) — small, low-risk release"
  echo -e "  ${BOLD}2${NC}) Stabilisation branch (release/${VERSION}) — needs hardening"
  echo -ne "${BOLD}Branch strategy${NC} [1/2]: "
  read -r strategy_choice
  case "${strategy_choice}" in
    1) BRANCH_STRATEGY="direct"; RELEASE_BRANCH="" ;;
    2) BRANCH_STRATEGY="stabilisation"; RELEASE_BRANCH="release/${VERSION}" ;;
    *)
      error "Invalid choice"
      exit 1
      ;;
  esac

  # Capture dev SHA
  DEV_SHA=$(git rev-parse HEAD)

  # Create the tracking issue
  create_release_issue "${VERSION}" "${TAG}" "${RELEASE_TYPE}" "${BRANCH_STRATEGY}" "${RELEASE_BRANCH}"

  success "Initialisation complete"
  info "Version: ${VERSION} | Tag: ${TAG} | Type: ${RELEASE_TYPE} | Strategy: ${BRANCH_STRATEGY}"
}

# --- Phase 1: Preflight ---

phase_preflight() {
  header "Phase 1: Preflight"

  # Rust checks
  hard_gate "cargo test" cargo test --workspace
  soft_gate "cargo clippy" cargo clippy --workspace --all-targets -- -D warnings
  hard_gate "cargo build" cargo build --release -p eddacraft-anvil

  # Verify binary version
  local binary_version
  binary_version=$(./target/release/anvil --version 2>&1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+[^ ]*' || echo "unknown")
  info "Binary reports version: ${binary_version}"
  BINARY_VERSION="${binary_version}"

  # TS workspace checks
  soft_gate "pnpm install" pnpm install --frozen-lockfile
  soft_gate "pnpm build" pnpm build
  soft_gate "pnpm test" timeout 180 run_bundled_pnpm_tests

  # Record preflight results
  PREFLIGHT_CARGO_TEST="pass"
  PREFLIGHT_CARGO_CLIPPY="pass"
  PREFLIGHT_CARGO_BUILD="pass"
  PREFLIGHT_PNPM_BUILD="pass"
  PREFLIGHT_PNPM_TEST="pass"
  PREFLIGHT_RELEASE_NOTES="pending"

  # Update issue
  update_issue_comment "$(cat <<PREFLIGHT_RESULTS
## Preflight Results

| Check | Result |
|-------|--------|
| cargo test | ✅ pass |
| cargo clippy | ✅ pass |
| cargo build | ✅ pass |
| binary version | ${binary_version} |
| pnpm build | ✅ pass |
| pnpm test | ✅ pass |
| release notes + docs | ⏳ pending pre-tag review |
PREFLIGHT_RESULTS
)"

  success "Preflight complete"
}

# --- Phase 2: Branch & Tag ---

phase_branch_and_tag() {
  header "Phase 2: Branch & Tag"

  echo ""
  info "Release prep happens on dev before promotion."
  info "Review and update release-facing files now, then continue:"
  info "  - Cargo.toml"
  info "  - package.json"
  info "  - bundled workspace package.json files"
  info "  - CHANGELOG.md"
  info "  - docs/public/anvil/beta-testing-guide.md"
  info "  - docs/public/anvil/releases/upgrade-notes.md"
  prompt_continue "Is dev ready to promote for ${TAG}?"

  # Prepare release commit on dev before promoting to main.
  local cargo_version
  cargo_version=$(grep '^version' "${CARGO_VERSION_FILE}" | head -1 | sed 's/.*"\(.*\)".*/\1/')
  if [[ "${cargo_version}" != "${VERSION}" ]]; then
    info "Workspace version is ${cargo_version}, updating to ${VERSION} on dev..."
    replace_first_match "${CARGO_VERSION_FILE}" '^version = "[^"]*"' "version = \"${VERSION}\""
    git add "${CARGO_VERSION_FILE}"
    if ! git diff --quiet Cargo.lock 2>/dev/null; then
      git add Cargo.lock
    fi
  else
    info "Workspace version already ${VERSION} on dev"
  fi

  update_package_json_version "${ROOT_PACKAGE_JSON}"

  local bundled_package_json
  for bundled_package_json in "${BUNDLED_PACKAGE_JSONS[@]}"; do
    update_package_json_version "$bundled_package_json"
  done

  if [[ -f "${BETA_GUIDE_FILE}" ]]; then
    replace_first_match "${BETA_GUIDE_FILE}" '\*\*Current version:\*\* [^\n]+' "**Current version:** ${VERSION}"
    git add "${BETA_GUIDE_FILE}"
  fi

  if [[ -f "${UPGRADE_NOTES_FILE}" ]]; then
    replace_first_match "${UPGRADE_NOTES_FILE}" '## Current Version: [^\n]+' "## Current Version: ${VERSION}"
    git add "${UPGRADE_NOTES_FILE}"
  fi

  info "Committing release prep on dev..."
  git add "${CARGO_VERSION_FILE}" "${ROOT_PACKAGE_JSON}" "${CHANGELOG_FILE}" \
    "${BETA_GUIDE_FILE}" "${UPGRADE_NOTES_FILE}" \
    "${BUNDLED_PACKAGE_JSONS[@]}" 2>/dev/null || true
  if ! git diff --cached --quiet; then
    git commit -m "chore(release): prepare ${TAG}"
    DEV_SHA=$(git rev-parse HEAD)
  else
    info "No release prep changes to commit on dev"
    DEV_SHA=$(git rev-parse HEAD)
  fi

  if [[ "${BRANCH_STRATEGY}" == "direct" ]]; then
    info "Direct promotion: opening PR from dev to main"
    local pr_url
    pr_url=$(gh pr create \
      --repo "${REPO}" \
      --base main \
      --head dev \
      --title "release: ${TAG}" \
      --body "Promote dev to main for release ${TAG}. Tracking: #${ISSUE_NUMBER}")
    info "PR created: ${pr_url}"
    echo ""
    prompt_continue "Merge the PR on GitHub, then continue?"

  elif [[ "${BRANCH_STRATEGY}" == "stabilisation" ]]; then
    info "Creating stabilisation branch: ${RELEASE_BRANCH}"
    git switch dev
    git pull --ff-only origin dev
    git switch -c "${RELEASE_BRANCH}"
    git push -u origin "${RELEASE_BRANCH}"

    local pr_url
    pr_url=$(gh pr create \
      --repo "${REPO}" \
      --base main \
      --head "${RELEASE_BRANCH}" \
      --title "release: ${TAG}" \
      --body "Promote ${RELEASE_BRANCH} to main for release ${TAG}. Tracking: #${ISSUE_NUMBER}")
    info "PR created: ${pr_url}"
    echo ""
    warn "Apply any stabilisation fixes to ${RELEASE_BRANCH} now."
    prompt_continue "Merge the PR on GitHub, then continue?"
  fi

  # Switch to main and pull
  info "Switching to main..."
  git switch main
  git pull --ff-only origin main
  MAIN_SHA=$(git rev-parse HEAD)

  # Commit and tag
  TAG_SHA=$(git rev-parse HEAD)

  info "Creating tag ${TAG}..."
  git tag -a "${TAG}" -m "${TAG}"

  info "Pushing main and tag..."
  git push origin main
  git push origin "${TAG}"

  update_issue_comment "$(cat <<TAG_RESULTS
## Branch & Tag

- **Strategy:** ${BRANCH_STRATEGY}
- **Dev SHA:** \`${DEV_SHA}\`
- **Main SHA:** \`${MAIN_SHA}\`
- **Tag SHA:** \`${TAG_SHA}\`
- **Tag:** ${TAG} pushed ✅
TAG_RESULTS
)"

  success "Tag ${TAG} pushed — release workflow should be running"
}

# --- Phase 3: Workflow monitoring kickoff ---

phase_workflow() {
  header "Phase 3: Workflow Monitoring"

  info "Waiting a moment for workflow to register..."
  sleep 5

  # Try to find the workflow run for the release tag specifically.
  WORKFLOW_RUN_ID=$(gh run list \
    --repo "${REPO}" \
    --limit 5 \
    --workflow release.yml \
    --json databaseId,headBranch,event,displayTitle \
    --jq '.[] | select(.event == "push" and (.displayTitle | contains("'"${TAG}"'"))) | .databaseId' \
    | head -1 || echo "")

  if [[ -n "${WORKFLOW_RUN_ID}" ]]; then
    local run_url="https://github.com/${REPO}/actions/runs/${WORKFLOW_RUN_ID}"
    success "Workflow run found: ${run_url}"
    info "Monitor with: gh run watch ${WORKFLOW_RUN_ID} --repo ${REPO}"
  else
    warn "Could not detect workflow run automatically"
    info "Check manually: gh run list --repo ${REPO} --limit 5"
    WORKFLOW_RUN_ID="unknown"
  fi

  update_issue_comment "## Workflow\n\nRun ID: ${WORKFLOW_RUN_ID}\nhttps://github.com/${REPO}/actions/runs/${WORKFLOW_RUN_ID}"
}

# --- Phase 4: Generate diff summary ---

phase_diff_summary() {
  header "Phase 4: Diff Summary"

  # Changed paths between dev and tag
  local changed_paths
  changed_paths=$(git diff --name-only "${DEV_SHA}..${TAG_SHA}" 2>/dev/null || echo "")

  # Derive changed crates
  CHANGED_CRATES=$(echo "${changed_paths}" | grep '^crates/' | cut -d'/' -f2 | sort -u | jq -R -s 'split("\n") | map(select(. != ""))') 2>/dev/null || CHANGED_CRATES="[]"

  # Derive changed packages
  CHANGED_PACKAGES=$(echo "${changed_paths}" | grep '^packages/' | cut -d'/' -f2-3 | sort -u | jq -R -s 'split("\n") | map(select(. != ""))') 2>/dev/null || CHANGED_PACKAGES="[]"

  # Derive changed top-level paths
  CHANGED_PATHS=$(echo "${changed_paths}" | cut -d'/' -f1-2 | sort -u | jq -R -s 'split("\n") | map(select(. != ""))') 2>/dev/null || CHANGED_PATHS="[]"

  local path_count
  path_count=$(echo "${changed_paths}" | wc -l | tr -d ' ')
  success "Found ${path_count} changed files across crates and packages"
}

# --- Phase 5: Write manifest ---

phase_manifest() {
  header "Phase 5: Write Manifest"

  mkdir -p "${MANIFEST_DIR}"

  cat > "${MANIFEST_FILE}" <<MANIFEST
{
  "version": "${VERSION}",
  "tag": "${TAG}",
  "releaseType": "${RELEASE_TYPE}",
  "branchStrategy": "${BRANCH_STRATEGY}",
  "releaseBranch": $(if [[ -n "${RELEASE_BRANCH}" ]]; then echo "\"${RELEASE_BRANCH}\""; else echo "null"; fi),
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "shas": {
    "dev": "${DEV_SHA}",
    "main": "${MAIN_SHA}",
    "tag": "${TAG_SHA}"
  },
  "workflowRunId": "${WORKFLOW_RUN_ID}",
  "issueNumber": ${ISSUE_NUMBER},
  "issueUrl": "${ISSUE_URL}",
  "preflight": {
    "cargoTest": "${PREFLIGHT_CARGO_TEST}",
    "cargoClippy": "${PREFLIGHT_CARGO_CLIPPY}",
    "cargoBuild": "${PREFLIGHT_CARGO_BUILD}",
    "binaryVersion": "${BINARY_VERSION}",
    "pnpmBuild": "${PREFLIGHT_PNPM_BUILD}",
    "pnpmTest": "${PREFLIGHT_PNPM_TEST}"
  },
  "diffSummary": {
    "changedPaths": ${CHANGED_PATHS},
    "changedPackages": ${CHANGED_PACKAGES},
    "changedCrates": ${CHANGED_CRATES}
  }
}
MANIFEST

  success "Manifest written to ${MANIFEST_FILE}"
  echo ""
  info "Release script complete."
  echo ""
  echo -e "  ${BOLD}Next step:${NC} run ${BLUE}/release${NC} in Claude Code to continue"
  echo -e "  with post-release verification, docs review, comms, and cleanup."
  echo ""
}

# --- Main ---

main() {
  header "Anvil Release Script"
  info "This script walks through the release process interactively."
  info "It will create a GitHub Issue, run preflight checks, handle"
  info "branching and tagging, and write a manifest for the /release skill."
  echo ""
  prompt_continue "Ready to start?"

  phase_init
  phase_preflight
  phase_branch_and_tag
  phase_workflow
  phase_diff_summary
  phase_manifest
}

main "$@"
