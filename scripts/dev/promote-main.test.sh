#!/usr/bin/env bash
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
helper="${repo_root}/scripts/dev/promote-main.sh"

tmp_parent=$(mktemp -d)
tmp="${tmp_parent}/space root"
mkdir "$tmp"
cleanup() {
  if [ -n "${concurrent_pid:-}" ]; then
    kill "$concurrent_pid" >/dev/null 2>&1 || true
  fi
  if [ -f "${tmp}/fake-daemon.pid" ]; then
    kill "$(cat "${tmp}/fake-daemon.pid")" >/dev/null 2>&1 || true
  fi
  rm -rf "$tmp_parent"
}
trap cleanup EXIT

origin="${tmp}/origin.git"
source_repo="${tmp}/source"
install_root="${tmp}/lib/anvil-main"
bin_dir="${tmp}/bin"
state_dir="${tmp}/state"
target_dir="${tmp}/target"
fake_cargo="${tmp}/cargo"
stable_binary="${bin_dir}/anvil"

git init -q --bare "$origin"
git init -q -b main "$source_repo"
git -C "$source_repo" config user.email test@example.invalid
git -C "$source_repo" config user.name 'Test User'
printf '[workspace]\nmembers = []\n' > "${source_repo}/Cargo.toml"
printf '# lock\n' > "${source_repo}/Cargo.lock"
git -C "$source_repo" add Cargo.toml Cargo.lock
git -C "$source_repo" commit -q -m initial
git -C "$source_repo" remote add origin "$origin"
git -C "$source_repo" push -q -u origin main
expected_sha=$(git -C "$source_repo" rev-parse HEAD)

mkdir -p "$bin_dir"
printf '#!/usr/bin/env bash\nprintf "stable sentinel\\n"\n' > "$stable_binary"
chmod +x "$stable_binary"

cat > "$fake_cargo" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail
if [ "${FAKE_CARGO_FAIL:-0}" = 1 ]; then
  exit 42
fi
if [ -n "${FAKE_CARGO_BARRIER_DIR:-}" ]; then
  mkdir -p "$FAKE_CARGO_BARRIER_DIR"
  : > "${FAKE_CARGO_BARRIER_DIR}/started"
  while [ ! -f "${FAKE_CARGO_BARRIER_DIR}/release" ]; do
    sleep 0.05
  done
fi
mkdir -p "${CARGO_TARGET_DIR}/release"
cat > "${CARGO_TARGET_DIR}/release/anvil" <<'BINARY'
#!/usr/bin/env bash
set -euo pipefail
if [ "${1:-}" = "--version" ]; then
  printf '%s\n' "${FAKE_ANVIL_VERSION:-anvil test-main}"
  exit 0
fi
printf '%s|%s|%s|%s\n' "${ANVIL_HOME:-}" "${ANVIL_MCP_PREFERRED:-}" \
  "${ANVIL_NO_SAVE_TIME_DRIVER-unset}" "$*" >> "${FAKE_ANVIL_LOG:?}"
case "${1:-} ${2:-}" in
  'auth whoami')
    [ "${FAKE_AUTH_REQUIRED:-0}" != 1 ]
    ;;
  'intercept stop')
    rm -f "${ANVIL_HOME:?}/fake-live"
    ;;
  'intercept start')
    if [ "${FAKE_DAEMON_DIES:-0}" = 1 ]; then
      exit 0
    fi
    : > "${ANVIL_HOME:?}/fake-live"
    printf '%s\n' "$$" > "${FAKE_DAEMON_PID:?}"
    while [ -f "${ANVIL_HOME}/fake-live" ]; do
      sleep 1
    done
    ;;
  'intercept status')
    [ -f "${ANVIL_HOME:?}/fake-live" ]
    ;;
esac
BINARY
chmod +x "${CARGO_TARGET_DIR}/release/anvil"
FAKE
chmod +x "$fake_cargo"
: > "${tmp}/anvil.log"

run_helper() {
  ANVIL_MAIN_REPO="$source_repo" \
    ANVIL_MAIN_INSTALL_ROOT="$install_root" \
    ANVIL_MAIN_BIN_DIR="$bin_dir" \
    ANVIL_MAIN_STATE_DIR="${TEST_STATE_DIR:-$state_dir}" \
    ANVIL_MAIN_PUBLISHED_STATE_DIR="$published_state" \
    ANVIL_MAIN_TARGET_DIR="$target_dir" \
    ANVIL_MAIN_CARGO="$fake_cargo" \
    ANVIL_MAIN_DAEMON_LAUNCHER=nohup \
    FAKE_ANVIL_LOG="${tmp}/anvil.log" \
    FAKE_DAEMON_PID="${tmp}/fake-daemon.pid" \
    "$helper" "$@"
}

# Candidate state must never follow a link into another anvil home.
published_state="${tmp}/published-state"
mkdir -p "$published_state"
ln -s "$published_state" "$state_dir"
if run_helper --no-restart-daemon > "${tmp}/symlink-state.out" 2>&1; then
  echo 'promotion unexpectedly accepted a symlinked state directory' >&2
  exit 1
fi
[ ! -e "${published_state}/fake-live" ]
rm "$state_dir"

# Candidate state must also be distinct when the published path is selected
# directly rather than reached through a symlink.
if TEST_STATE_DIR="$published_state" run_helper --no-restart-daemon > "${tmp}/published-state.out" 2>&1; then
  echo 'promotion unexpectedly accepted the published state directory' >&2
  exit 1
fi

# A real concurrent promotion is excluded before either process can select a
# release or recycle the daemon.
barrier="${tmp}/cargo-barrier"
FAKE_CARGO_BARRIER_DIR="$barrier" run_helper --no-restart-daemon > "${tmp}/first-concurrent.out" &
concurrent_pid=$!
for _ in 1 2 3 4 5 6 7 8 9 10; do
  [ -f "${barrier}/started" ] && break
  sleep 0.1
done
[ -f "${barrier}/started" ]
if run_helper --no-restart-daemon > "${tmp}/second-concurrent.out" 2>&1; then
  echo 'concurrent promotion unexpectedly acquired the active lock' >&2
  exit 1
fi
grep -Fq 'another anvil-main promotion holds' "${tmp}/second-concurrent.out"
: > "${barrier}/release"
wait "$concurrent_pid"
concurrent_pid=''

# Promotion is pinned to fetched origin/main and leaves the stable command alone.
run_helper --no-restart-daemon > "${tmp}/promote.out"
candidate="${install_root}/${expected_sha}/anvil"
[ -x "$candidate" ]
[ -L "${bin_dir}/anvil-main" ]
[ "$(readlink "${bin_dir}/anvil-main")" = "${install_root}/${expected_sha}/anvil" ]
[ "$(readlink "${install_root}/current")" = "${install_root}/${expected_sha}" ]
[ -x "${target_dir}/${expected_sha}/release/anvil" ]
[ ! -e "${target_dir}/release/anvil" ]
[ "$("$stable_binary")" = 'stable sentinel' ]
python3 - "$state_dir" <<'PY'
import os
import stat
import sys

assert stat.S_IMODE(os.stat(sys.argv[1]).st_mode) == 0o700
PY
grep -Fqx "source_sha=${expected_sha}" "${install_root}/current/provenance.env"
grep -Fqx 'source_ref=origin/main' "${install_root}/current/provenance.env"
grep -Fq "promoted ${expected_sha}" "${tmp}/promote.out"

# Status reports the immutable source and whether the local origin/main moved.
run_helper --status > "${tmp}/status-current.out"
grep -Fq "source SHA: ${expected_sha}" "${tmp}/status-current.out"
grep -Fq 'main status: current' "${tmp}/status-current.out"

printf 'next\n' > "${source_repo}/next.txt"
git -C "$source_repo" add next.txt
git -C "$source_repo" commit -q -m next
git -C "$source_repo" push -q origin main
git -C "$source_repo" fetch -q origin main
next_sha=$(git -C "$source_repo" rev-parse HEAD)
run_helper --status > "${tmp}/status-behind.out"
grep -Fq 'main status: behind origin/main' "${tmp}/status-behind.out"

# A per-SHA install directory cannot escape the immutable install root.
escaped_install="${tmp}/escaped-install"
mkdir -p "$escaped_install"
ln -s "$escaped_install" "${install_root}/${next_sha}"
if run_helper --no-restart-daemon > "${tmp}/symlink-install.out" 2>&1; then
  echo 'promotion unexpectedly accepted a symlinked immutable directory' >&2
  exit 1
fi
[ ! -e "${escaped_install}/anvil" ]
rm "${install_root}/${next_sha}"

# A binary inside an otherwise valid immutable directory may not be a symlink.
mkdir "${install_root}/${next_sha}"
ln -s /bin/true "${install_root}/${next_sha}/anvil"
if run_helper --no-restart-daemon > "${tmp}/symlink-binary.out" 2>&1; then
  echo 'promotion unexpectedly accepted a symlinked immutable binary' >&2
  exit 1
fi
rm -rf "${install_root:?}/${next_sha}"

# Status rejects a forged channel target, hash, or version instead of trusting metadata.
channel_target=$(readlink "${bin_dir}/anvil-main")
ln -sfn /bin/true "${bin_dir}/anvil-main"
if run_helper --status > "${tmp}/status-forged-target.out" 2>&1; then
  echo 'status unexpectedly accepted a forged channel target' >&2
  exit 1
fi
ln -sfn "$channel_target" "${bin_dir}/anvil-main"

provenance="${install_root}/${expected_sha}/provenance.env"
cp "$provenance" "${tmp}/provenance.good"
sed 's/^binary_sha256=.*/binary_sha256=0000/' "${tmp}/provenance.good" > "$provenance"
if run_helper --status > "${tmp}/status-forged-hash.out" 2>&1; then
  echo 'status unexpectedly accepted a forged binary hash' >&2
  exit 1
fi
cp "${tmp}/provenance.good" "$provenance"
sed 's/^version=.*/version=anvil forged/' "${tmp}/provenance.good" > "$provenance"
if run_helper --status > "${tmp}/status-forged-version.out" 2>&1; then
  echo 'status unexpectedly accepted a forged version' >&2
  exit 1
fi
cp "${tmp}/provenance.good" "$provenance"

# A failed build cannot move an already-working channel link.
before=$(readlink "${bin_dir}/anvil-main")
if FAKE_CARGO_FAIL=1 run_helper --no-restart-daemon > "${tmp}/failed.out" 2>&1; then
  echo 'promotion unexpectedly succeeded with a failed build' >&2
  exit 1
fi
[ "$(readlink "${bin_dir}/anvil-main")" = "$before" ]

# An invalid destination for provenance cannot move the selected release.
mkdir -p "${install_root}/${next_sha}/provenance.env"
if run_helper --no-restart-daemon > "${tmp}/failed-provenance.out" 2>&1; then
  echo 'promotion unexpectedly succeeded with an invalid provenance path' >&2
  exit 1
fi
[ "$(readlink "${install_root}/current")" = "${install_root}/${expected_sha}" ]
rm -rf "${install_root:?}/${next_sha}"

# An invalid convenience link is rejected before the authoritative channel
# can move, avoiding a partial-success promotion.
before=$(readlink "${bin_dir}/anvil-main")
rm "${install_root}/current"
mkdir "${install_root}/current"
if run_helper --no-restart-daemon > "${tmp}/invalid-current.out" 2>&1; then
  echo 'promotion unexpectedly replaced a non-symlink current path' >&2
  exit 1
fi
[ "$(readlink "${bin_dir}/anvil-main")" = "$before" ]
rmdir "${install_root}/current"
ln -s "${install_root}/${expected_sha}" "${install_root}/current"

# Missing candidate credentials select the binary but never disturb a daemon.
before_stop_count=$(grep -Fc '|intercept stop' "${tmp}/anvil.log" 2>/dev/null || true)
if FAKE_AUTH_REQUIRED=1 run_helper > "${tmp}/auth-required.out" 2>&1; then
  echo 'promotion unexpectedly restarted an unauthenticated candidate' >&2
  exit 1
fi
grep -Fq 'candidate authentication is required' "${tmp}/auth-required.out"
after_stop_count=$(grep -Fc '|intercept stop' "${tmp}/anvil.log" 2>/dev/null || true)
[ "$after_stop_count" = "$before_stop_count" ]

# Daemon lifecycle receives candidate-only state and the explicit channel path.
ANVIL_NO_SAVE_TIME_DRIVER=1 run_helper > "${tmp}/daemon.out"
[ "$(readlink "${install_root}/current")" = "${install_root}/${next_sha}" ]
run_helper --status > "${tmp}/status-repromoted.out"
grep -Fq 'main status: current' "${tmp}/status-repromoted.out"
grep -Fqx "${state_dir}|${bin_dir}/anvil-main|unset|intercept stop" "${tmp}/anvil.log"
grep -Fqx "${state_dir}|${bin_dir}/anvil-main|unset|intercept start --foreground" "${tmp}/anvil.log"
[ -f "${tmp}/fake-daemon.pid" ]
kill -0 "$(cat "${tmp}/fake-daemon.pid")"

# A launcher that exits before serving cannot be reported as ready.
rm -f "$state_dir/fake-live" "${tmp}/fake-daemon.pid"
if FAKE_DAEMON_DIES=1 run_helper > "${tmp}/daemon-died.out" 2>&1; then
  echo 'promotion unexpectedly accepted a dead daemon' >&2
  exit 1
fi

printf 'promote-main tests passed\n'
