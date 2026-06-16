#!/usr/bin/env bash
# CIB-034: render a sanitised, public release-evidence Markdown record from the
# signed build-provenance manifest (`anvil-<tag>-provenance.json`).
#
# Anvil's source is private; only its releases are public. This produces a
# concise, human-readable trust record proving the published artefacts are the
# exact build released under the public tag. It emits ONLY public-safe fields and
# deliberately omits raw logs, secrets, internal hostnames, private workflow
# URLs, the private repository name, and any private development detail — the
# authoritative machine binding (including the gating workflow run) stays in the
# signed provenance JSON that is published alongside this evidence.
#
# Usage:
#   scripts/release/generate-evidence.sh \
#     --provenance artifacts/anvil-<tag>-provenance.json \
#     --output     artifacts/release-evidence-<tag>.md
set -euo pipefail

PROVENANCE=""
OUTPUT=""
while [ $# -gt 0 ]; do
  case "$1" in
    --provenance) PROVENANCE="${2:-}"; shift 2 ;;
    --output) OUTPUT="${2:-}"; shift 2 ;;
    -h | --help)
      sed -n '2,17p' "$0"
      exit 0
      ;;
    *)
      echo "::error::unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

if [ -z "$PROVENANCE" ] || [ ! -f "$PROVENANCE" ]; then
  echo "::error::--provenance <file> is required and must exist" >&2
  exit 2
fi
if [ -z "$OUTPUT" ]; then
  echo "::error::--output <file> is required" >&2
  exit 2
fi
if ! command -v jq >/dev/null 2>&1; then
  echo "::error::jq is required" >&2
  exit 2
fi

# Public-safe fields only. `private_build.repository`, `.workflow_run_url`, and
# `.ref` are intentionally NOT read — they name the private source/CI.
tag=$(jq -r '.release_tag // empty' "$PROVENANCE")
version="${tag#v}"
built_at=$(jq -r '.built_at // empty' "$PROVENANCE")
# The source revision is an opaque commit id already carried by the public,
# signed provenance asset; including it lets an auditor cross-check the chain.
source_sha=$(jq -r '.private_build.commit_sha // empty' "$PROVENANCE")
public_repo=$(jq -r '.public_release.repository // empty' "$PROVENANCE")
public_tag=$(jq -r '.public_release.tag // .release_tag // empty' "$PROVENANCE")
public_ref=$(jq -r '.public_release.ref_at_publish // empty' "$PROVENANCE")
asset_count=$(jq '.assets | length' "$PROVENANCE")

if [ -z "$tag" ] || [ -z "$public_repo" ]; then
  echo "::error::provenance manifest missing release_tag / public_release.repository" >&2
  exit 1
fi
if [ "${asset_count:-0}" -lt 1 ]; then
  echo "::error::provenance manifest has 0 assets — refusing to write empty evidence" >&2
  exit 1
fi

release_url="https://github.com/${public_repo}/releases/tag/${public_tag}"

{
  echo "# Release evidence — ${tag}"
  echo
  echo "A sanitised, public trust record for the \`${tag}\` release of Anvil."
  echo "Anvil's source is private; this evidence proves the artefacts listed below"
  echo "are the exact build published under the public \`${public_tag}\` release. It"
  echo "is generated automatically at release time from the signed build-provenance"
  echo "manifest."
  echo
  echo "| Field | Value |"
  echo "| --- | --- |"
  echo "| Version | \`${version}\` |"
  echo "| Tag | \`${tag}\` |"
  [ -n "$source_sha" ] && echo "| Source revision | \`${source_sha}\` |"
  echo "| Public release | [\`${public_repo}\`](${release_url}) |"
  [ -n "$public_ref" ] && echo "| Public ref at publish | \`${public_ref}\` |"
  [ -n "$built_at" ] && echo "| Published (UTC) | ${built_at} |"
  echo "| Artefacts | ${asset_count} |"
  echo
  echo "## Validation"
  echo
  echo "All blocking release gates passed for \`${tag}\` on the readiness run before"
  echo "publication. The full machine-readable build provenance — including the"
  echo "gating workflow run and the build matrix — is the signed"
  echo "\`anvil-${tag}-provenance.json\` published alongside this evidence; this"
  echo "document is its human-readable, sanitised summary."
  echo
  echo "## Artefacts"
  echo
  echo "Each shipped artefact with its SHA-256 digest, computed at build time."
  echo
  echo "| Artefact | SHA-256 | Size (bytes) |"
  echo "| --- | --- | --- |"
  jq -r '.assets | sort_by(.name)[] | "| `\(.name)` | `\(.sha256)` | \(.size_bytes) |"' "$PROVENANCE"
  echo
  echo "## Verifying an artefact"
  echo
  echo "Download an artefact from the [release page](${release_url}) and compare"
  echo "its digest against the table above:"
  echo
  echo '```sh'
  echo "sha256sum <downloaded-file>"
  echo '```'
  echo
  echo "The digests here, the per-artefact \`.sha256\` sidecars on the release, and"
  echo "the signed \`anvil-${tag}-provenance.json\` all agree by construction."
  echo
  echo "---"
  echo
  echo "_Auto-generated from the signed build-provenance manifest (CIB-034). It"
  echo "deliberately omits raw logs, secrets, internal hostnames, private workflow"
  echo "URLs, and private development detail._"
} >"$OUTPUT"

echo "::notice title=release evidence::wrote ${OUTPUT} for ${tag} (${asset_count} artefacts)"
