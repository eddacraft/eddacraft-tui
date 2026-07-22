#!/usr/bin/env bash
set -euo pipefail

# bump-homebrew.sh — patch and (optionally) publish the Anvil Homebrew formula.
#
# Extracted from the inline "Publish Homebrew formula to eddacraft/homebrew-tap"
# step in .github/workflows/release.yml so it can be exercised locally, dry-run
# in CI, and re-run from a workflow_dispatch recovery surface. Per DISTRIB-003
# in plans/modules/distribution-and-update.aps.md and the operator runbook
# docs/runbooks/homebrew-publish.md.
#
# cargo-dist emits Formula/eddacraft-anvil.rb with `class EddacraftAnvil <
# Formula`. Homebrew dispatches `brew install eddacraft/tap/anvil` to a class
# named `Anvil` in Formula/anvil.rb, so the only structural patch needed is the
# class rename; the SHA256s, URLs, and version are correct as cargo-dist emits
# them.

readonly DEFAULT_TAP_REPO="eddacraft/homebrew-tap"
readonly DEFAULT_TARGET_PATH="Formula/anvil.rb"

# sysexits.h conventions: 64 EX_USAGE, 66 EX_NOINPUT, 70 EX_SOFTWARE,
# 78 EX_CONFIG. Tests assert on these exact codes so the contract is stable
# for both the workflow caller and the operator runbook.
readonly EX_USAGE=64
readonly EX_NOINPUT=66
readonly EX_SOFTWARE=70
readonly EX_CONFIG=78

release_tag=""
formula_source=""
out_path=""
tap_repo="$DEFAULT_TAP_REPO"
target_path="$DEFAULT_TARGET_PATH"
publish=false
dry_run=false

usage() {
  cat <<'USAGE'
Usage: bump-homebrew.sh --release-tag <vX.Y.Z[-suffix]> --formula-source <path> --out <path> [--publish] [--tap-repo <owner/name>] [--target-path <path>] [--dry-run]

Patch the cargo-dist Homebrew formula (rename class EddacraftAnvil -> Anvil),
write it to --out, and optionally publish it to the Homebrew tap.

Required:
  --release-tag <tag>      Release tag, e.g. v0.7.0-beta (must look like vX.Y.Z[-suffix]).
  --formula-source <path>  Path to cargo-dist's eddacraft-anvil.rb.
  --out <path>             Local path to write the patched formula.

Optional:
  --publish                After patching, push the formula to the tap via gh api.
  --tap-repo <owner/name>  Tap repo to publish to. Default: eddacraft/homebrew-tap.
  --target-path <path>     Path inside the tap repo. Default: Formula/anvil.rb.
  --dry-run                With --publish, print the intended publish without
                           making the network call. Patched formula is still
                           written to --out so callers can inspect the diff.
  -h, --help               Show this help.

Environment:
  GH_TOKEN | ANVIL_RELEASES_TOKEN  Required when --publish is set without --dry-run.

Exit codes:
   0  success
  64  invalid arguments (EX_USAGE)
  66  --formula-source not found (EX_NOINPUT)
  70  patched formula missing 'class Anvil < Formula' (EX_SOFTWARE)
  78  --publish requested but no GH token configured (EX_CONFIG)
USAGE
}

die() {
  local code="$1"
  shift
  printf 'bump-homebrew: %s\n' "$*" >&2
  exit "$code"
}

while (($# > 0)); do
  case "$1" in
    --release-tag)     release_tag="${2:-}"; shift 2 ;;
    --formula-source)  formula_source="${2:-}"; shift 2 ;;
    --out)             out_path="${2:-}"; shift 2 ;;
    --tap-repo)        tap_repo="${2:-}"; shift 2 ;;
    --target-path)     target_path="${2:-}"; shift 2 ;;
    --publish)         publish=true; shift ;;
    --dry-run)         dry_run=true; shift ;;
    -h|--help)         usage; exit 0 ;;
    *)                 die "$EX_USAGE" "unknown argument: $1" ;;
  esac
done

[[ -n "$release_tag" ]]    || die "$EX_USAGE" "--release-tag is required"
[[ -n "$formula_source" ]] || die "$EX_USAGE" "--formula-source is required"
[[ -n "$out_path" ]]       || die "$EX_USAGE" "--out is required"

if [[ ! "$release_tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)*)?$ ]]; then
  die "$EX_USAGE" "--release-tag must look like vX.Y.Z[-suffix]; got: $release_tag"
fi

if [[ ! -f "$formula_source" ]]; then
  die "$EX_NOINPUT" "formula source not found: $formula_source"
fi

# Patch in a tempfile so we never write a partial/corrupt file to --out.
tmp_patched="$(mktemp)"
trap 'rm -f "$tmp_patched"' EXIT

# Rename the cargo-dist class to the Homebrew-expected class. Anchored to start
# of line + exact `< Formula` suffix so we don't accidentally rewrite a comment
# or doc-string that happens to mention the old name.
sed 's/^class EddacraftAnvil < Formula/class Anvil < Formula/' \
  "$formula_source" > "$tmp_patched"

if ! grep -q '^class Anvil < Formula' "$tmp_patched"; then
  die "$EX_SOFTWARE" "patched formula is missing 'class Anvil < Formula' — source: $formula_source"
fi

# Atomic-ish move so a parallel reader never sees a half-written file.
mv "$tmp_patched" "$out_path"
trap - EXIT

printf 'bump-homebrew: wrote %s\n' "$out_path"
printf 'bump-homebrew: anvil %s targeted at %s:%s\n' \
  "$release_tag" "$tap_repo" "$target_path"

if [[ "$publish" != true ]]; then
  exit 0
fi

# --- Publish path -----------------------------------------------------------

token="${GH_TOKEN:-${ANVIL_RELEASES_TOKEN:-}}"

if [[ "$dry_run" == true ]]; then
  printf 'bump-homebrew: DRY-RUN would PUT %s -> %s\n' \
    "$out_path" "https://api.github.com/repos/$tap_repo/contents/$target_path"
  printf 'bump-homebrew: DRY-RUN commit message would be: anvil %s\n' "$release_tag"
  exit 0
fi

if [[ -z "$token" ]]; then
  die "$EX_CONFIG" "--publish requires GH_TOKEN or ANVIL_RELEASES_TOKEN to be set"
fi

if ! command -v gh >/dev/null 2>&1; then
  die "$EX_CONFIG" "--publish requires the 'gh' CLI to be installed"
fi

# Fetch the existing SHA (if any) so we PUT an update instead of failing on
# the second release. `gh api` exits non-zero on 404; treat that as "no file
# yet" and continue with no sha argument.
existing_sha=""
if existing_sha="$(GH_TOKEN="$token" gh api "repos/$tap_repo/contents/$target_path" --jq '.sha' 2>/dev/null)"; then
  :
else
  existing_sha=""
fi

content_b64="$(base64 -w 0 "$out_path" 2>/dev/null || base64 "$out_path" | tr -d '\n')"

sha_args=()
if [[ -n "$existing_sha" ]]; then
  sha_args=(-f "sha=$existing_sha")
fi

GH_TOKEN="$token" gh api "repos/$tap_repo/contents/$target_path" -X PUT \
  -f message="anvil $release_tag" \
  -f content="$content_b64" \
  "${sha_args[@]}"

printf 'bump-homebrew: published anvil %s to %s:%s\n' \
  "$release_tag" "$tap_repo" "$target_path"
