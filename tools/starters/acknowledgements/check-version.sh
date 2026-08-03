#!/usr/bin/env bash
# Verify the acknowledgements kit's release version is internally
# consistent, and optionally that a release tag agrees with it.
#
# The kit carries its version in two places that must never drift:
#   - `VERSION`        — a single semver line (e.g. `1.0.0`)
#   - `CHANGELOG.md`   — newest entry headed `## [X.Y.Z] - <date>`
#
# Both travel into the public mirror, so consumers read the same
# version markers after a `subtree pull` that they see on a GitHub
# Release. This checker is the single source of that invariant: it is
# run by the kit self-tests (against the real files) AND by the
# `release-acknowledgements-starter.yml` workflow (with `--tag`, to
# complete the version-triple assertion before any mirror push).
#
# Usage:
#   check-version.sh                 # VERSION == newest CHANGELOG.md heading
#   check-version.sh --dir <path>    # check a different kit dir (tests/fixtures)
#   check-version.sh --tag <tag>     # also require the tag's X.Y.Z == VERSION
#                                    # (accepts `X.Y.Z`, `vX.Y.Z`, or the
#                                    #  prefixed `acknowledgements-starter-vX.Y.Z`)
#
# Exit codes:
#   0  consistent
#   1  inconsistency / missing file / malformed version
#   2  CLI argument error

set -euo pipefail

dir_override=""
tag=""

while [ $# -gt 0 ]; do
  case "$1" in
    --dir)
      if [ -z "${2:-}" ]; then
        echo "error: --dir requires a path argument" >&2
        exit 2
      fi
      dir_override="$2"
      shift 2
      ;;
    --tag)
      if [ -z "${2:-}" ]; then
        echo "error: --tag requires a tag argument" >&2
        exit 2
      fi
      tag="$2"
      shift 2
      ;;
    -h|--help)
      sed -n '2,27p' "$0"
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

# --- Resolve the kit directory ----------------------------------------
# Default to this script's own directory, resolving symlinks by hand so
# it works when the kit is vendored or the script is symlinked. Plain
# `readlink` (no -f) keeps this portable to macOS, which lacked
# `readlink -f` before 12.3. Bounded to avoid a circular-symlink hang.
if [ -n "$dir_override" ]; then
  kit_dir="$dir_override"
else
  script_path="$0"
  link_hops=0
  while [ -L "$script_path" ]; do
    link_hops=$((link_hops + 1))
    if [ "$link_hops" -gt 40 ]; then
      echo "error: too many symlink levels resolving $0 (circular symlink?)" >&2
      exit 1
    fi
    link_target="$(readlink "$script_path")"
    case "$link_target" in
      /*) script_path="$link_target" ;;
      *)  script_path="$(dirname "$script_path")/$link_target" ;;
    esac
  done
  kit_dir="$(cd "$(dirname "$script_path")" && pwd)"
fi

version_file="$kit_dir/VERSION"
changelog_file="$kit_dir/CHANGELOG.md"

# --- Read + validate VERSION ------------------------------------------
if [ ! -f "$version_file" ]; then
  echo "error: VERSION file is missing at $version_file" >&2
  exit 1
fi

# First non-blank line, trimmed of surrounding whitespace.
version="$(grep -m1 -v '^[[:space:]]*$' "$version_file" 2>/dev/null || true)"
version="${version#"${version%%[![:space:]]*}"}"
version="${version%"${version##*[![:space:]]}"}"

# Strict SemVer: numeric identifiers carry no leading zeros (0 or [1-9]…).
semver_re='^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-[0-9A-Za-z.-]+)?$'
if ! printf '%s' "$version" | grep -qE "$semver_re"; then
  echo "error: VERSION '$version' is not a valid semver (expected X.Y.Z) in $version_file" >&2
  exit 1
fi

# --- Read newest CHANGELOG.md heading ---------------------------------
if [ ! -f "$changelog_file" ]; then
  echo "error: CHANGELOG.md is missing at $changelog_file" >&2
  exit 1
fi

# Newest entry: the first `## [X.Y.Z]` heading from the top of the file.
heading_version="$(grep -m1 -oE '^## \[[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?\]' "$changelog_file" 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?' || true)"
if [ -z "$heading_version" ]; then
  echo "error: no '## [X.Y.Z]' version heading found in $changelog_file" >&2
  exit 1
fi

if [ "$version" != "$heading_version" ]; then
  echo "error: VERSION ($version) does not match the newest CHANGELOG.md heading ([$heading_version])" >&2
  echo "       bump both together, or fix whichever is stale." >&2
  exit 1
fi

# --- Optional: tag must agree -----------------------------------------
if [ -n "$tag" ]; then
  tag_version="$tag"
  tag_version="${tag_version#acknowledgements-starter-}"   # drop source-tag prefix → leaves vX.Y.Z
  tag_version="${tag_version#v}"                            # drop a single leading v → X.Y.Z
  # A malformed double-prefixed tag (…-vvX.Y.Z) now leaves a leading "v",
  # which the semver check below rejects rather than silently accepting.
  if ! printf '%s' "$tag_version" | grep -qE "$semver_re"; then
    echo "error: tag '$tag' does not carry a valid semver version (parsed '$tag_version')" >&2
    exit 1
  fi
  if [ "$tag_version" != "$version" ]; then
    echo "error: tag version ($tag_version, from '$tag') does not match VERSION ($version)" >&2
    echo "       tag the commit whose VERSION + CHANGELOG.md match the release." >&2
    exit 1
  fi
fi

# shellcheck disable=SC2016 # False positive: the single quotes here are
# literal characters inside a double-quoted string, so $tag does expand.
echo "version ok: $version (VERSION == CHANGELOG.md${tag:+ == tag '$tag'})"
