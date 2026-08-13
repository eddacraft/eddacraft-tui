#!/usr/bin/env bash

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
validator="${repo_root}/scripts/release/validate-signing-inputs.sh"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

tag="v0.9.0-beta"
source_sha="6b0ed1d1d7d7662d1033403ae6291d907afe262d"
run_id="29190475570"
private_dir="${tmp}/private"
public_dir="${tmp}/public"

fail() {
  echo "validate-signing-inputs.test.sh: $*" >&2
  exit 1
}

write_fixture() {
  rm -rf "$private_dir" "$public_dir"
  mkdir -p "$private_dir" "$public_dir"
  printf '#!/bin/sh\necho installer\n' > "${private_dir}/eddacraft-anvil-installer.sh"
  printf 'Write-Output installer\n' > "${private_dir}/eddacraft-anvil-installer.ps1"
  cp "${private_dir}/eddacraft-anvil-installer.sh" "$public_dir/"
  cp "${private_dir}/eddacraft-anvil-installer.ps1" "$public_dir/"

  sh_sha=$(sha256sum "${private_dir}/eddacraft-anvil-installer.sh" | awk '{print $1}')
  ps1_sha=$(sha256sum "${private_dir}/eddacraft-anvil-installer.ps1" | awk '{print $1}')
  jq -n \
    --arg tag "$tag" \
    --arg sha "$source_sha" \
    --arg run "$run_id" \
    --arg sh_sha "$sh_sha" \
    --arg ps1_sha "$ps1_sha" \
    '{
      release_tag: $tag,
      private_build: {commit_sha: $sha, workflow_run_id: $run},
      assets: [
        {name: "eddacraft-anvil-installer.sh", sha256: $sh_sha},
        {name: "eddacraft-anvil-installer.ps1", sha256: $ps1_sha}
      ]
    }' > "${private_dir}/anvil-${tag}-provenance.json"
  cp "${private_dir}/anvil-${tag}-provenance.json" "$public_dir/"
}

run_validator() {
  "$validator" \
    --tag "$tag" \
    --source-sha "$source_sha" \
    --run-id "$run_id" \
    --private-dir "$private_dir" \
    --public-dir "$public_dir"
}

expect_failure() {
  local label="$1"
  if run_validator >/dev/null 2>&1; then
    fail "expected ${label} fixture to fail"
  fi
}

expect_tag_rejected() {
  local bad_tag="$1"
  local output=""
  local rc=0
  output=$("$validator" \
    --tag "$bad_tag" \
    --source-sha "$source_sha" \
    --run-id "$run_id" \
    --private-dir "$private_dir" \
    --public-dir "$public_dir" 2>&1) || rc=$?
  [ "$rc" -ne 0 ] || fail "expected tag ${bad_tag} to fail"
  grep -Fq -- '--tag must be a single path component' <<< "$output" \
    || fail "tag ${bad_tag} did not fail at tag validation: ${output}"
  if grep -Fq -- 'required private release asset is missing' <<< "$output"; then
    fail "tag ${bad_tag} was rejected after path construction: ${output}"
  fi
}

for flag in --tag --source-sha --run-id --private-dir --public-dir; do
  if output=$("$validator" "$flag" 2>&1); then
    fail "expected missing value for ${flag} to fail"
  fi
  grep -Fq -- "${flag} requires a value" <<< "$output" \
    || fail "missing value for ${flag} did not produce a contextual error"
done

write_fixture
run_validator >/dev/null || fail "valid fixture failed"

rm "${private_dir}/eddacraft-anvil-installer.ps1"
expect_failure "missing required asset"

write_fixture
printf 'tampered\n' >> "${public_dir}/eddacraft-anvil-installer.sh"
expect_failure "public/private mismatch"

write_fixture
jq '.private_build.workflow_run_id = "wrong-run"' \
  "${private_dir}/anvil-${tag}-provenance.json" > "${private_dir}/provenance.tmp"
mv "${private_dir}/provenance.tmp" "${private_dir}/anvil-${tag}-provenance.json"
cp "${private_dir}/anvil-${tag}-provenance.json" "$public_dir/"
expect_failure "provenance run mismatch"

write_fixture
jq '.assets[0].sha256 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"' \
  "${private_dir}/anvil-${tag}-provenance.json" > "${private_dir}/provenance.tmp"
mv "${private_dir}/provenance.tmp" "${private_dir}/anvil-${tag}-provenance.json"
cp "${private_dir}/anvil-${tag}-provenance.json" "$public_dir/"
expect_failure "provenance digest mismatch"

# A traversal tag must be rejected before path construction, even when a
# matching provenance document exists outside the private/public directories.
write_fixture
mkdir -p "${private_dir}/anvil-.." "${public_dir}/anvil-.." "${tmp}/outside"
escaped_tag="../../../outside/release"
sh_sha=$(sha256sum "${private_dir}/eddacraft-anvil-installer.sh" | awk '{print $1}')
ps1_sha=$(sha256sum "${private_dir}/eddacraft-anvil-installer.ps1" | awk '{print $1}')
jq -n \
  --arg tag "$escaped_tag" \
  --arg sha "$source_sha" \
  --arg run "$run_id" \
  --arg sh_sha "$sh_sha" \
  --arg ps1_sha "$ps1_sha" \
  '{
    release_tag: $tag,
    private_build: {commit_sha: $sha, workflow_run_id: $run},
    assets: [
      {name: "eddacraft-anvil-installer.sh", sha256: $sh_sha},
      {name: "eddacraft-anvil-installer.ps1", sha256: $ps1_sha}
    ]
  }' > "${tmp}/outside/release-provenance.json"
expect_tag_rejected "$escaped_tag"
expect_tag_rejected "foo/../bar"
expect_tag_rejected "/tmp/outside/release"

printf 'validate-signing-inputs.test.sh: ok\n'
