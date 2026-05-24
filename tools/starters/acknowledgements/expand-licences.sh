#!/usr/bin/env bash
# ATTRIB-006: expand `licences.toml` into `about.toml.accepted` and
# `deny.toml.[licenses].allow` array fragments.
#
# ATTRIB-012: also expand into `licences.node-allow.txt` as a single
# semicolon-joined SPDX list — consumed by `drivers/node.sh` via
# `license-checker --onlyAllow`. The Node fragment is emitted only
# when `licences.node-allow.txt` exists alongside the other consumer
# files; absent file means "this consumer does not need a Node
# allow-list", and the expander stays silent rather than failing.
#
# Reads the canonical `licences.toml` (default: project root) and
# rebuilds each consumer file's licence array between BEGIN/END
# marker comments. Hand-curated content outside the markers is
# preserved verbatim — same splice pattern as the
# acknowledgements generator.
#
# Usage:
#   expand-licences.sh                      # rebuild every consumer file in place
#   expand-licences.sh --check              # verify all are in sync; exit 1 on drift
#   expand-licences.sh --config <path>      # explicit licences.toml location
#
# Exit codes:
#   0  success / no drift
#   1  drift detected, missing markers, missing files, or bad config
#   2  CLI argument error

set -euo pipefail

mode="write"
config_override=""

while [ $# -gt 0 ]; do
  case "$1" in
    --check)
      mode="check"
      shift
      ;;
    --config)
      if [ -z "${2:-}" ]; then
        echo "error: --config requires a path argument" >&2
        exit 2
      fi
      case "$2" in
        /*) config_override="$2" ;;
        *)  config_override="$PWD/$2" ;;
      esac
      shift 2
      ;;
    -h|--help)
      sed -n '2,20p' "$0"
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

# --- Locate licences.toml ----------------------------------------------------

if [ -n "$config_override" ]; then
  if [ ! -f "$config_override" ]; then
    echo "error: --config path does not exist: $config_override" >&2
    exit 1
  fi
  config_path="$config_override"
else
  search="$PWD"
  config_path=""
  while [ "$search" != "/" ]; do
    if [ -f "$search/licences.toml" ]; then
      config_path="$search/licences.toml"
      break
    fi
    search="$(dirname "$search")"
  done
  if [ -z "$config_path" ]; then
    echo "error: licences.toml not found in CWD or any parent directory" >&2
    exit 1
  fi
fi

project_root="$(cd "$(dirname "$config_path")" && pwd)"
about_toml="$project_root/about.toml"
deny_toml="$project_root/deny.toml"
node_allow_txt="$project_root/licences.node-allow.txt"
go_allow_txt="$project_root/licences.go-allow.txt"
python_allow_txt="$project_root/licences.python-allow.txt"

if [ ! -f "$about_toml" ]; then
  echo "error: $about_toml is missing" >&2
  exit 1
fi
if [ ! -f "$deny_toml" ]; then
  echo "error: $deny_toml is missing" >&2
  exit 1
fi

# `licences.node-allow.txt` is optional. Consumers that ship a Node
# attribution block create the file (copy
# `licences.node-allow.txt.template` from the kit); consumers without
# a Node block do not. ATTRIB-012 chose this back-compat shape to
# match the dispatcher's flat-`[rust]` shim — existing Rust-only
# consumers don't migrate.
emit_node_fragment=false
if [ -f "$node_allow_txt" ]; then
  emit_node_fragment=true
fi
# `licences.go-allow.txt` is optional on the same back-compat terms as
# the Node fragment (ATTRIB-013): present only for consumers that ship
# a Go attribution block.
emit_go_fragment=false
if [ -f "$go_allow_txt" ]; then
  emit_go_fragment=true
fi
# `licences.python-allow.txt` is optional on the same back-compat terms
# (ATTRIB-014): present only for consumers that ship a Python block.
emit_python_fragment=false
if [ -f "$python_allow_txt" ]; then
  emit_python_fragment=true
fi

# --- Parse licences.toml -----------------------------------------------------
#
# Walk [[licences]] entries with awk. Each entry is a key/value block
# until the next `[[licences]]` or EOF. We emit one record per entry
# to stdout, tab-separated: spdx \t about \t deny \t note
#
# Triple-quoted strings are unwrapped into a single space-joined line
# so the consumer can render them as a single comment.

parsed_entries="$(mktemp)"
trap 'rm -f "$parsed_entries"' EXIT

# Minimal TOML parser scoped to `[[licences]]` entries with the
# four fixed keys (spdx/about/deny/note). Single-line string values
# only — see licences.toml's schema comment.
awk '
function flush() {
  if (in_block) {
    if (spdx == "") {
      printf "error: licences.toml entry near line %d is missing spdx\n", start_line > "/dev/stderr"
      exit 1
    }
    printf "%s\t%s\t%s\t%s\n", spdx, about_v, deny_v, note
  }
  spdx = ""; about_v = "false"; deny_v = "false"; note = ""
}

function strip_string(v) {
  sub(/^[[:space:]]*"/, "", v)
  sub(/"[[:space:]]*$/, "", v)
  # Unescape \" inside the string (TOML basic strings).
  gsub(/\\"/, "\"", v)
  return v
}

BEGIN { in_block = 0 }

/^[[:space:]]*#/ { next }
/^[[:space:]]*$/ { next }

/^[[:space:]]*\[\[licences\]\][[:space:]]*$/ {
  flush()
  in_block = 1
  start_line = NR
  next
}

in_block && /^[[:space:]]*spdx[[:space:]]*=/ {
  v = $0
  sub(/^[^=]*=/, "", v)
  spdx = strip_string(v)
  next
}

in_block && /^[[:space:]]*about[[:space:]]*=/ {
  v = $0
  sub(/^[^=]*=[[:space:]]*/, "", v)
  about_v = v
  next
}

in_block && /^[[:space:]]*deny[[:space:]]*=/ {
  v = $0
  sub(/^[^=]*=[[:space:]]*/, "", v)
  deny_v = v
  next
}

in_block && /^[[:space:]]*note[[:space:]]*=/ {
  v = $0
  sub(/^[^=]*=/, "", v)
  note = strip_string(v)
  next
}

END { flush() }
' "$config_path" >"$parsed_entries"

# --- Render fragments --------------------------------------------------------
#
# Each fragment is an indented multi-line array body matching the
# style each consumer already uses. Inline comments before an entry
# carry the licences.toml `note`.

# ATTRIB-016: deterministic note wrapping. Replaces `fold -s -w 75`,
# which wraps on **byte** count: a note containing multi-byte UTF-8 (an
# em dash is 3 bytes) breaks at a different word boundary across coreutils
# implementations — GNU vs uutils vs BusyBox vs uutils versions — so the
# locally-generated comment lines differ byte-for-byte from what CI
# regenerates and `--check` reports drift the author can't see locally
# (bit PR #1911, fixed by regenerating with a matching `fold` in
# `898554a6`). This wraps on whitespace at <=75 Unicode code points per
# line, computed in byte mode (LC_ALL=C, discounting UTF-8 continuation
# bytes 0x80-0xBF), so the output is identical regardless of which `fold`
# — or none — is on PATH. Code points, not display columns: licence notes
# are Latin prose with the occasional em dash, never wide CJK.
cp_len() {
  # Code-point length of $1: strip UTF-8 continuation bytes (0x80-0xBF)
  # and count the remaining bytes. Byte mode (LC_ALL=C) makes both the
  # bracket-range match and ${#...} operate on bytes deterministically.
  local LC_ALL=C
  local s="$1" stripped
  stripped="${s//[$'\x80'-$'\xbf']/}"
  printf '%s' "${#stripped}"
}

wrap_note() {
  # Emit $1 as `  # `-prefixed comment lines, wrapped on whitespace at
  # <=75 code points per line. No dependency on `fold`.
  local note="$1"
  local -i width=75 lw ww
  local line="" word
  local -a words
  local LC_ALL=C
  # Split on whitespace. Runs of internal spaces collapse to one — the
  # licences.toml schema is single-space prose, so this is lossless here.
  read -ra words <<<"$note"
  for word in "${words[@]}"; do
    ww=$(cp_len "$word")
    if [ -z "$line" ]; then
      line="$word"
      lw=$ww
    elif [ $((lw + 1 + ww)) -le "$width" ]; then
      line="$line $word"
      lw=$((lw + 1 + ww))
    else
      printf '  # %s\n' "$line"
      line="$word"
      lw=$ww
    fi
  done
  [ -n "$line" ] && printf '  # %s\n' "$line"
}

render_fragment() {
  # $1: column to filter on ("about" or "deny")
  # $2: array name for the header comment (e.g. "about.toml.accepted")
  local filter="$1"
  local target_array_name="$2"
  local col
  case "$filter" in
    about) col=2 ;;
    deny)  col=3 ;;
    *) echo "internal error: unknown filter $filter" >&2; exit 1 ;;
  esac

  echo "  # Generated from licences.toml — do not edit between the BEGIN/END markers."
  echo "  # Update licences.toml and rerun tools/starters/acknowledgements/expand-licences.sh."
  echo "  #"
  echo "  # Source: licences.toml ([[licences]] entries where $filter = true)"
  while IFS=$'\t' read -r spdx about_v deny_v note; do
    local include_v
    if [ "$col" = 2 ]; then
      include_v="$about_v"
    else
      include_v="$deny_v"
    fi
    if [ "$include_v" != "true" ]; then
      continue
    fi
    if [ -n "$note" ]; then
      # Wrap long notes at <=75 code points on whitespace so the
      # consumer file stays readable. Deterministic across coreutils
      # implementations — see wrap_note / ATTRIB-016.
      wrap_note "$note"
    fi
    echo "  \"$spdx\","
  done <"$parsed_entries"
  unset target_array_name
}

# ATTRIB-012: render a single semicolon-joined SPDX list for the Node
# driver's `license-checker --onlyAllow` argument. The Node fragment
# is one line — `license-checker` doesn't take a multi-line file,
# so the consumer file has the list on one line between the markers
# and the driver `cat`s it. Only entries where about = true count as
# allowed (the "accepted" set, matching the Rust about.toml allow).
render_node_fragment() {
  local spdx_list=""
  while IFS=$'\t' read -r spdx about_v deny_v note; do
    if [ "$about_v" != "true" ]; then
      continue
    fi
    if [ -z "$spdx_list" ]; then
      spdx_list="$spdx"
    else
      spdx_list="$spdx_list;$spdx"
    fi
  done <"$parsed_entries"
  echo "$spdx_list"
}

# ATTRIB-013: render a single comma-joined SPDX list for the Go driver's
# `go-licenses check --allowed_licenses` argument. Same one-line,
# about = true contract as the Node fragment; only the separator differs
# (go-licenses takes a comma-separated list, while license-checker and
# pip-licenses take semicolon-separated ones).
render_go_fragment() {
  local spdx_list=""
  while IFS=$'\t' read -r spdx about_v deny_v note; do
    if [ "$about_v" != "true" ]; then
      continue
    fi
    if [ -z "$spdx_list" ]; then
      spdx_list="$spdx"
    else
      spdx_list="$spdx_list,$spdx"
    fi
  done <"$parsed_entries"
  echo "$spdx_list"
}

# ATTRIB-014: render a single semicolon-joined SPDX list for the Python
# driver's `pip-licenses --allow-only` argument (semicolon-separated,
# same shape as the Node fragment; only the consumer file differs).
render_python_fragment() {
  local spdx_list=""
  while IFS=$'\t' read -r spdx about_v deny_v note; do
    if [ "$about_v" != "true" ]; then
      continue
    fi
    if [ -z "$spdx_list" ]; then
      spdx_list="$spdx"
    else
      spdx_list="$spdx_list;$spdx"
    fi
  done <"$parsed_entries"
  echo "$spdx_list"
}

about_fragment="$(mktemp)"
deny_fragment="$(mktemp)"
node_allow_fragment="$(mktemp)"
go_allow_fragment="$(mktemp)"
python_allow_fragment="$(mktemp)"
trap 'rm -f "$parsed_entries" "$about_fragment" "$deny_fragment" "$node_allow_fragment" "$go_allow_fragment" "$python_allow_fragment"' EXIT

render_fragment about about.toml.accepted >"$about_fragment"
render_fragment deny  "deny.toml.[licenses].allow" >"$deny_fragment"
render_node_fragment >"$node_allow_fragment"
render_go_fragment >"$go_allow_fragment"
render_python_fragment >"$python_allow_fragment"

# --- Splice into consumer files ---------------------------------------------

MARKER_BEGIN_ABOUT="# BEGIN AUTO-GENERATED FROM licences.toml — accepted"
MARKER_END_ABOUT="# END AUTO-GENERATED FROM licences.toml — accepted"
MARKER_BEGIN_DENY="# BEGIN AUTO-GENERATED FROM licences.toml — allow"
MARKER_END_DENY="# END AUTO-GENERATED FROM licences.toml — allow"
MARKER_BEGIN_NODE_ALLOW="# BEGIN AUTO-GENERATED FROM licences.toml — node-allow"
MARKER_END_NODE_ALLOW="# END AUTO-GENERATED FROM licences.toml — node-allow"
MARKER_BEGIN_GO_ALLOW="# BEGIN AUTO-GENERATED FROM licences.toml — go-allow"
MARKER_END_GO_ALLOW="# END AUTO-GENERATED FROM licences.toml — go-allow"
MARKER_BEGIN_PYTHON_ALLOW="# BEGIN AUTO-GENERATED FROM licences.toml — python-allow"
MARKER_END_PYTHON_ALLOW="# END AUTO-GENERATED FROM licences.toml — python-allow"

splice() {
  # $1: target file path
  # $2: BEGIN marker string
  # $3: END marker string
  # $4: fragment file path
  local target="$1" begin="$2" end="$3" fragment="$4"
  local begin_count end_count
  begin_count=$(grep -cF "$begin" "$target" || true)
  end_count=$(grep -cF "$end" "$target" || true)
  if [ "$begin_count" != "1" ] || [ "$end_count" != "1" ]; then
    echo "error: $target must contain exactly one BEGIN and one END marker." >&2
    echo "  '$begin' count: $begin_count (expected 1)" >&2
    echo "  '$end' count: $end_count (expected 1)" >&2
    exit 1
  fi

  local target_dir tmp
  target_dir="$(cd "$(dirname "$target")" && pwd)"
  tmp="$(mktemp "$target_dir/.expand-licences.tmp.XXXXXX")"

  awk -v frag="$fragment" -v begin="$begin" -v end="$end" '
    BEGIN { in_block = 0 }
    index($0, begin) {
      print
      while ((getline line < frag) > 0) print line
      in_block = 1
      next
    }
    index($0, end) {
      in_block = 0
      print
      next
    }
    in_block { next }
    { print }
  ' "$target" >"$tmp"

  if [ "$mode" = "check" ]; then
    if ! diff -u "$target" "$tmp" >/dev/null; then
      echo "error: $target is out of sync with licences.toml. Diff:" >&2
      diff -u "$target" "$tmp" >&2 || true
      rm -f "$tmp"
      return 1
    fi
    rm -f "$tmp"
  else
    mv "$tmp" "$target"
  fi
}

drift=0
splice "$about_toml" "$MARKER_BEGIN_ABOUT" "$MARKER_END_ABOUT" "$about_fragment" || drift=1
splice "$deny_toml"  "$MARKER_BEGIN_DENY"  "$MARKER_END_DENY"  "$deny_fragment"  || drift=1
if [ "$emit_node_fragment" = "true" ]; then
  splice "$node_allow_txt" "$MARKER_BEGIN_NODE_ALLOW" "$MARKER_END_NODE_ALLOW" "$node_allow_fragment" || drift=1
fi
if [ "$emit_go_fragment" = "true" ]; then
  splice "$go_allow_txt" "$MARKER_BEGIN_GO_ALLOW" "$MARKER_END_GO_ALLOW" "$go_allow_fragment" || drift=1
fi
if [ "$emit_python_fragment" = "true" ]; then
  splice "$python_allow_txt" "$MARKER_BEGIN_PYTHON_ALLOW" "$MARKER_END_PYTHON_ALLOW" "$python_allow_fragment" || drift=1
fi

if [ "$drift" -ne 0 ]; then
  echo "" >&2
  echo "Run 'tools/starters/acknowledgements/expand-licences.sh' to regenerate." >&2
  exit 1
fi

if [ "$mode" = "write" ]; then
  expanded="about.toml (accepted), deny.toml (allow)"
  if [ "$emit_node_fragment" = "true" ]; then
    expanded="$expanded, licences.node-allow.txt (node-allow)"
  fi
  if [ "$emit_go_fragment" = "true" ]; then
    expanded="$expanded, licences.go-allow.txt (go-allow)"
  fi
  if [ "$emit_python_fragment" = "true" ]; then
    expanded="$expanded, licences.python-allow.txt (python-allow)"
  fi
  echo "ok: licences.toml expanded into $expanded"
fi
