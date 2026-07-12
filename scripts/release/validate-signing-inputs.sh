#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: validate-signing-inputs.sh --tag <tag> --source-sha <sha> --run-id <id> \
  --private-dir <path> --public-dir <path>

Validate that the exact required signing inputs exist on both releases, are
byte-identical, and are bound by provenance to the successful Release run.
USAGE
}

die() {
  echo "validate-signing-inputs: $*" >&2
  exit 1
}

require_value() {
  case "${2:-}" in
    "" | --*) die "$1 requires a value" ;;
  esac
}

tag=""
source_sha=""
run_id=""
private_dir=""
public_dir=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    --tag)
      require_value "$@"
      tag="$2"
      shift 2
      ;;
    --source-sha)
      require_value "$@"
      source_sha="$2"
      shift 2
      ;;
    --run-id)
      require_value "$@"
      run_id="$2"
      shift 2
      ;;
    --private-dir)
      require_value "$@"
      private_dir="$2"
      shift 2
      ;;
    --public-dir)
      require_value "$@"
      public_dir="$2"
      shift 2
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *) die "unknown argument: $1" ;;
  esac
done

[ -n "$tag" ] || die "--tag is required"
[[ "$source_sha" =~ ^[0-9a-f]{40}$ ]] || die "--source-sha must be a 40-character lowercase Git SHA"
[[ "$run_id" =~ ^[0-9]+$ ]] || die "--run-id must be numeric"
[ -d "$private_dir" ] || die "private asset directory does not exist: $private_dir"
[ -d "$public_dir" ] || die "public asset directory does not exist: $public_dir"

provenance="anvil-${tag}-provenance.json"
required_assets=(
  "eddacraft-anvil-installer.sh"
  "eddacraft-anvil-installer.ps1"
  "$provenance"
)

for asset in "${required_assets[@]}"; do
  private_asset="${private_dir}/${asset}"
  public_asset="${public_dir}/${asset}"
  [ -f "$private_asset" ] || die "required private release asset is missing: $asset"
  [ -f "$public_asset" ] || die "required public release asset is missing: $asset"
  cmp -s "$private_asset" "$public_asset" || die "public release asset differs from private build asset: $asset"
done

provenance_path="${private_dir}/${provenance}"
actual_tag=$(jq -er '.release_tag' "$provenance_path") || die "provenance release_tag is missing"
actual_sha=$(jq -er '.private_build.commit_sha' "$provenance_path") || die "provenance commit_sha is missing"
actual_run=$(jq -er '.private_build.workflow_run_id' "$provenance_path") || die "provenance workflow_run_id is missing"

[ "$actual_tag" = "$tag" ] || die "provenance tag mismatch: expected $tag, found $actual_tag"
[ "$actual_sha" = "$source_sha" ] || die "provenance source SHA mismatch: expected $source_sha, found $actual_sha"
[ "$actual_run" = "$run_id" ] || die "provenance workflow run mismatch: expected $run_id, found $actual_run"

for asset in "eddacraft-anvil-installer.sh" "eddacraft-anvil-installer.ps1"; do
  entry_count=$(jq --arg name "$asset" '[.assets[] | select(.name == $name)] | length' "$provenance_path")
  [ "$entry_count" -eq 1 ] || die "provenance must contain exactly one digest for $asset; found $entry_count"
  recorded_sha=$(jq -er --arg name "$asset" '.assets[] | select(.name == $name) | .sha256' "$provenance_path") \
    || die "provenance digest is missing for $asset"
  actual_asset_sha=$(sha256sum "${private_dir}/${asset}" | awk '{print $1}')
  [ "$recorded_sha" = "$actual_asset_sha" ] \
    || die "provenance digest mismatch for $asset: expected $recorded_sha, computed $actual_asset_sha"
done

printf 'validate-signing-inputs: ok (%s, run %s, %s)\n' "$tag" "$run_id" "$source_sha"
