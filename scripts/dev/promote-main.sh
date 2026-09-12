#!/usr/bin/env bash
# Promote the exact fetched origin/main binary into the rolling anvil-main
# dogfood channel without changing the published `anvil` command.

set -euo pipefail

repo_root="${ANVIL_MAIN_REPO:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
install_root="${ANVIL_MAIN_INSTALL_ROOT:-${HOME}/.local/lib/anvil-main}"
bin_dir="${ANVIL_MAIN_BIN_DIR:-${HOME}/.local/bin}"
state_dir="${ANVIL_MAIN_STATE_DIR:-${HOME}/.anvil-main}"
published_state_dir="${ANVIL_MAIN_PUBLISHED_STATE_DIR:-${HOME}/.anvil}"
target_dir="${ANVIL_MAIN_TARGET_DIR:-${HOME}/.cache/anvil-targets/anvil-main}"
cargo_bin="${ANVIL_MAIN_CARGO:-cargo}"
daemon_launcher="${ANVIL_MAIN_DAEMON_LAUNCHER:-auto}"
channel_path="${bin_dir}/anvil-main"
current_path="${install_root}/current"
metadata_path=''
mode=promote
restart_daemon=true

usage() {
  cat <<'USAGE'
Usage: scripts/dev/promote-main.sh [--status] [--no-restart-daemon]

Fetch and build the exact origin/main commit, install it immutably, then
atomically point `anvil-main` at it. The published `anvil` command is never
changed. By default, only the isolated anvil-main daemon is restarted.

  --status              Report the promoted SHA and local origin/main drift
  --no-restart-daemon   Promote without recycling the anvil-main daemon
  -h, --help            Show this help
USAGE
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --status) mode=status ;;
    --no-restart-daemon) restart_daemon=false ;;
    -h|--help) usage; exit 0 ;;
    *) printf 'unknown argument: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
  shift
done

metadata_value() {
  local key="$1"
  sed -n "s/^${key}=//p" "$metadata_path" | head -n 1
}

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

physical_path() {
  python3 - "$1" <<'PY'
import os
import sys

print(os.path.realpath(sys.argv[1]))
PY
}

reject_symlink_dir() {
  local path="$1"
  local label="$2"
  if [ -L "$path" ]; then
    printf 'refusing symlinked %s: %s\n' "$label" "$path" >&2
    exit 1
  fi
  if [ -e "$path" ] && [ ! -d "$path" ]; then
    printf 'refusing non-directory %s: %s\n' "$label" "$path" >&2
    exit 1
  fi
}

paths_overlap() {
  python3 - "$1" "$2" <<'PY'
import os
import sys

left, right = (os.path.realpath(path) for path in sys.argv[1:])
try:
    common = os.path.commonpath((left, right))
except ValueError:
    raise SystemExit(1)
raise SystemExit(0 if common in (left, right) else 1)
PY
}

candidate_cmd() {
  env -u ANVIL_NO_SAVE_TIME_DRIVER \
    ANVIL_HOME="$state_dir" ANVIL_MCP_PREFERRED="$channel_path" \
    "$channel_path" "$@"
}

atomic_replace() {
  python3 - "$1" "$2" <<'PY'
import os
import sys

os.replace(sys.argv[1], sys.argv[2])
PY
}

mode_status() {
  if [ -L "$install_root" ] || [ ! -d "$install_root" ]; then
    printf 'anvil-main: invalid install root\n' >&2
    return 1
  fi
  if [ ! -L "$channel_path" ]; then
    printf 'anvil-main: not promoted\n'
    return 1
  fi

  local source_sha source_ref version expected_binary actual_binary immutable_dir
  local expected_hash actual_hash actual_version origin_main linked_target install_physical
  actual_binary=$(physical_path "$channel_path")
  immutable_dir=$(dirname "$actual_binary")
  metadata_path="${immutable_dir}/provenance.env"
  if [ ! -f "$metadata_path" ] || [ -L "$metadata_path" ]; then
    printf 'anvil-main: invalid provenance path\n' >&2
    return 1
  fi
  source_sha=$(metadata_value source_sha)
  source_ref=$(metadata_value source_ref)
  version=$(metadata_value version)
  expected_hash=$(metadata_value binary_sha256)
  if [[ ! "$source_sha" =~ ^[0-9a-f]{40}$ ]]; then
    printf 'anvil-main: invalid provenance source SHA\n' >&2
    return 1
  fi
  if [ "$source_ref" != origin/main ]; then
    printf 'anvil-main: invalid provenance source ref\n' >&2
    return 1
  fi
  install_physical=$(physical_path "$install_root")
  if [ -L "${install_root}/${source_sha}" ] || [ ! -d "${install_root}/${source_sha}" ] ||
    [ "$immutable_dir" != "${install_physical}/${source_sha}" ]; then
    printf 'anvil-main: invalid immutable install directory\n' >&2
    return 1
  fi
  expected_binary="${install_root}/${source_sha}/anvil"
  if [ -L "$expected_binary" ] || [ ! -f "$expected_binary" ]; then
    printf 'anvil-main: invalid immutable binary\n' >&2
    return 1
  fi
  if [ "$actual_binary" != "$(physical_path "$expected_binary")" ] || [ ! -x "$actual_binary" ]; then
    printf 'anvil-main: channel target does not match provenance\n' >&2
    return 1
  fi
  actual_hash=$(sha256_file "$actual_binary")
  if [ -z "$expected_hash" ] || [ "$actual_hash" != "$expected_hash" ]; then
    printf 'anvil-main: binary hash does not match provenance\n' >&2
    return 1
  fi
  actual_version=$("$actual_binary" --version 2>/dev/null || true)
  if [ -z "$version" ] || [ "$actual_version" != "$version" ]; then
    printf 'anvil-main: binary version does not match provenance\n' >&2
    return 1
  fi
  origin_main=$(git -C "$repo_root" rev-parse --verify refs/remotes/origin/main 2>/dev/null || true)
  linked_target=$(readlink "$channel_path")

  printf 'anvil-main: %s\n' "$channel_path"
  printf 'target: %s\n' "$linked_target"
  printf 'source ref: %s\n' "$source_ref"
  printf 'source SHA: %s\n' "$source_sha"
  printf 'version: %s\n' "$version"
  if [ "$source_sha" = "$origin_main" ]; then
    printf 'main status: current\n'
  elif git -C "$repo_root" merge-base --is-ancestor "$source_sha" "$origin_main" 2>/dev/null; then
    printf 'main status: behind origin/main\n'
  else
    printf 'main status: differs from origin/main\n'
  fi
}

if [ "$mode" = status ]; then
  mode_status
  exit $?
fi

build_worktree=''
worktree_added=false
lock_dir=''
lock_acquired=false

cleanup() {
  if [ "$worktree_added" = true ]; then
    git -C "$repo_root" worktree remove "$build_worktree" >/dev/null 2>&1 || true
  elif [ -n "$build_worktree" ]; then
    rmdir "$build_worktree" >/dev/null 2>&1 || true
  fi
  if [ "$lock_acquired" = true ]; then
    rm -f "${lock_dir}/owner" >/dev/null 2>&1 || true
    rmdir "$lock_dir" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

reject_symlink_dir "$install_root" 'install root'
mkdir -p "$install_root"
reject_symlink_dir "$install_root" 'install root'
lock_dir="${install_root}/.promote.lock"
if ! mkdir "$lock_dir" 2>/dev/null; then
  lock_owner=$(sed -n '1p' "${lock_dir}/owner" 2>/dev/null || true)
  printf 'another anvil-main promotion holds %s' "$lock_dir" >&2
  if [ -n "$lock_owner" ]; then
    printf ' (pid %s)' "$lock_owner" >&2
  fi
  printf '\n' >&2
  exit 1
fi
lock_acquired=true
printf '%s\n' "$$" > "${lock_dir}/owner"

git -C "$repo_root" fetch origin '+refs/heads/main:refs/remotes/origin/main'
source_sha=$(git -C "$repo_root" rev-parse --verify 'refs/remotes/origin/main^{commit}')
build_worktree=$(mktemp -d "${TMPDIR:-/tmp}/anvil-main-build.XXXXXX")

git -C "$repo_root" worktree add --quiet --detach "$build_worktree" "$source_sha"
worktree_added=true

printf 'building origin/main at %s\n' "$source_sha"
(
  cd "$build_worktree"
  CARGO_TARGET_DIR="${target_dir}/${source_sha}" \
    "$cargo_bin" build --locked --release -p eddacraft-anvil
)

built_binary="${target_dir}/${source_sha}/release/anvil"
if [ ! -x "$built_binary" ]; then
  printf 'expected executable not found: %s\n' "$built_binary" >&2
  exit 1
fi
version=$("$built_binary" --version)

mkdir -p "$install_root" "$bin_dir"
reject_symlink_dir "$state_dir" 'candidate state directory'
mkdir -p "$state_dir"
published_roots=(
  "$published_state_dir"
  "${XDG_CONFIG_HOME:-${HOME}/.config}/anvil"
  "${XDG_STATE_HOME:-${HOME}/.local/state}/anvil"
  "${XDG_CACHE_HOME:-${HOME}/.cache}/anvil"
)
if [ -n "${XDG_RUNTIME_DIR:-}" ]; then
  published_roots+=("${XDG_RUNTIME_DIR}/anvil")
else
  published_roots+=("/run/user/$(id -u)/anvil")
fi
for published_root in "${published_roots[@]}"; do
  if paths_overlap "$state_dir" "$published_root"; then
    printf 'candidate state overlaps a published anvil path: %s and %s\n' \
      "$state_dir" "$published_root" >&2
    exit 1
  fi
done
chmod 700 "$state_dir"
immutable_dir="${install_root}/${source_sha}"
immutable_binary="${immutable_dir}/anvil"
reject_symlink_dir "$immutable_dir" 'immutable install directory'
mkdir -p "$immutable_dir"
if [ "$(physical_path "$immutable_dir")" != "$(physical_path "$install_root")/${source_sha}" ]; then
  printf 'immutable install escaped its root: %s\n' "$immutable_dir" >&2
  exit 1
fi

if [ -e "$immutable_binary" ]; then
  if [ -L "$immutable_binary" ] || [ ! -f "$immutable_binary" ]; then
    printf 'refusing invalid immutable binary path: %s\n' "$immutable_binary" >&2
    exit 1
  fi
  if ! cmp -s "$built_binary" "$immutable_binary"; then
    printf 'immutable install already exists with different content: %s\n' "$immutable_binary" >&2
    exit 1
  fi
else
  immutable_tmp="${immutable_binary}.tmp.$$"
  install -m 755 "$built_binary" "$immutable_tmp"
  mv "$immutable_tmp" "$immutable_binary"
fi

binary_sha256=$(sha256_file "$immutable_binary")
promoted_at=$(date -u '+%Y-%m-%dT%H:%M:%SZ')
immutable_metadata="${immutable_dir}/provenance.env"
if [ -L "$immutable_metadata" ] || { [ -e "$immutable_metadata" ] && [ ! -f "$immutable_metadata" ]; }; then
  printf 'refusing invalid provenance path: %s\n' "$immutable_metadata" >&2
  exit 1
fi
metadata_tmp="${immutable_metadata}.tmp.$$"
{
  printf 'source_ref=origin/main\n'
  printf 'source_sha=%s\n' "$source_sha"
  printf 'binary_sha256=%s\n' "$binary_sha256"
  printf 'version=%s\n' "$version"
  printf 'promoted_at=%s\n' "$promoted_at"
} > "$metadata_tmp"

atomic_replace "$metadata_tmp" "$immutable_metadata"

if [ -e "$channel_path" ] && [ ! -L "$channel_path" ]; then
  printf 'refusing to replace non-symlink channel path: %s\n' "$channel_path" >&2
  exit 1
fi
if [ -e "$current_path" ] && [ ! -L "$current_path" ]; then
  printf 'refusing to replace non-symlink current path: %s\n' "$current_path" >&2
  exit 1
fi

# The channel link is the single authoritative selection pointer. It moves in
# one atomic operation to a binary whose adjacent provenance is already
# complete. `current` follows afterwards as a convenience link only.
channel_tmp="${channel_path}.tmp.$$"
ln -s "$immutable_binary" "$channel_tmp"
atomic_replace "$channel_tmp" "$channel_path"
current_tmp="${current_path}.tmp.$$"
ln -s "$immutable_dir" "$current_tmp"
atomic_replace "$current_tmp" "$current_path"

if [ "$restart_daemon" = true ]; then
  if ! candidate_cmd auth whoami --json >/dev/null 2>&1; then
    printf 'anvil-main promoted, but candidate authentication is required before daemon restart\n' >&2
    printf 'run: ANVIL_HOME=%q %q auth login\n' "$state_dir" "$channel_path" >&2
    exit 1
  fi
  printf 'restarting isolated anvil-main daemon\n'
  candidate_cmd intercept stop >/dev/null

  daemon_stopped=false
  for _ in 1 2 3 4 5; do
    if ! candidate_cmd intercept status >/dev/null 2>&1; then
      daemon_stopped=true
      break
    fi
    sleep 1
  done
  if [ "$daemon_stopped" != true ]; then
    printf 'anvil-main promoted, but the previous daemon did not stop\n' >&2
    exit 1
  fi

  use_systemd=false
  if [ "$daemon_launcher" = systemd ] || {
    [ "$daemon_launcher" = auto ] && command -v systemd-run >/dev/null 2>&1 &&
      systemctl --user show-environment >/dev/null 2>&1;
  }; then
    use_systemd=true
  fi

  if [ "$use_systemd" = true ]; then
    systemctl --user stop anvil-main-intercept.service >/dev/null 2>&1 || true
    systemctl --user reset-failed anvil-main-intercept.service >/dev/null 2>&1 || true
    systemd-run --user --no-block --unit=anvil-main-intercept --collect --quiet \
      env -u ANVIL_NO_SAVE_TIME_DRIVER \
      ANVIL_HOME="$state_dir" ANVIL_MCP_PREFERRED="$channel_path" \
      "$channel_path" intercept start --foreground
  else
    nohup env -u ANVIL_NO_SAVE_TIME_DRIVER \
      ANVIL_HOME="$state_dir" ANVIL_MCP_PREFERRED="$channel_path" \
      "$channel_path" intercept start --foreground \
        >> "${state_dir}/daemon.log" 2>&1 &
    daemon_pid=$!
  fi

  daemon_ready=false
  for _ in 1 2 3 4 5; do
    if [ "$use_systemd" != true ] && ! kill -0 "$daemon_pid" >/dev/null 2>&1; then
      break
    fi
    if candidate_cmd intercept status >/dev/null 2>&1; then
      daemon_ready=true
      break
    fi
    sleep 1
  done
  if [ "$daemon_ready" = true ] && [ "$use_systemd" = true ]; then
    systemctl --user is-active --quiet anvil-main-intercept.service || daemon_ready=false
  fi
  if [ "$daemon_ready" = true ]; then
    sleep 1
    if ! candidate_cmd intercept status >/dev/null 2>&1; then
      daemon_ready=false
    elif [ "$use_systemd" = true ]; then
      systemctl --user is-active --quiet anvil-main-intercept.service || daemon_ready=false
    else
      kill -0 "$daemon_pid" >/dev/null 2>&1 || daemon_ready=false
    fi
  fi
  if [ "$daemon_ready" != true ]; then
    if [ "$use_systemd" = true ]; then
      printf 'anvil-main promoted, but its daemon did not become ready; inspect: journalctl --user-unit anvil-main-intercept.service\n' >&2
    else
      printf 'anvil-main promoted, but its daemon did not become ready; inspect %s\n' \
        "${state_dir}/daemon.log" >&2
    fi
    exit 1
  fi
fi

printf 'promoted %s as %s (%s)\n' "$source_sha" "$channel_path" "$version"
printf 'candidate state: ANVIL_HOME=%s\n' "$state_dir"
printf 'reconnect MCP clients so they launch the promoted binary\n'
