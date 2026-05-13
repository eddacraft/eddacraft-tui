#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)

assert_contains() {
  local haystack=$1
  local needle=$2
  if [[ "$haystack" != *"$needle"* ]]; then
    printf 'Expected output to contain: %s\nActual output:\n%s\n' "$needle" "$haystack" >&2
    exit 1
  fi
}

assert_not_contains() {
  local haystack=$1
  local needle=$2
  if [[ "$haystack" == *"$needle"* ]]; then
    printf 'Expected output not to contain: %s\nActual output:\n%s\n' "$needle" "$haystack" >&2
    exit 1
  fi
}

test_homebrew_install_detected_before_download() {
  local tmp fake_brew fake_bin output status
  tmp=$(mktemp -d)
  fake_brew="$tmp/homebrew"
  fake_bin="$tmp/bin"
  mkdir -p "$fake_brew/bin" "$fake_bin"

  printf '#!/usr/bin/env sh\nprintf "anvil 0.6.2-beta\\n"\n' > "$fake_brew/bin/anvil"
  chmod +x "$fake_brew/bin/anvil"
  printf '#!/usr/bin/env sh\nprintf "curl should not run\\n" >&2\nexit 99\n' > "$fake_bin/curl"
  chmod +x "$fake_bin/curl"

  set +e
  output=$(PATH="$fake_brew/bin:$fake_bin:/usr/bin:/bin" HOMEBREW_PREFIX="$fake_brew" sh "$ROOT/install.sh" 2>&1)
  status=$?
  set -e

  rm -rf "$tmp"

  if [[ $status -ne 0 ]]; then
    printf 'Expected status 0, got %s\nOutput:\n%s\n' "$status" "$output" >&2
    exit 1
  fi
  assert_contains "$output" "Anvil is already installed via Homebrew"
  assert_contains "$output" "brew upgrade eddacraft/tap/anvil"
  assert_not_contains "$output" "curl should not run"
}

test_homebrew_symlink_install_detected_before_download() {
  local tmp fake_brew fake_bin output status
  tmp=$(mktemp -d)
  fake_brew="$tmp/homebrew"
  fake_bin="$tmp/bin"
  mkdir -p "$fake_brew/bin" "$fake_brew/Cellar/anvil/0.6.2/bin" "$fake_bin"

  printf '#!/usr/bin/env sh\nprintf "anvil 0.6.2-beta\\n"\n' > "$fake_brew/Cellar/anvil/0.6.2/bin/anvil"
  chmod +x "$fake_brew/Cellar/anvil/0.6.2/bin/anvil"
  ln -s ../Cellar/anvil/0.6.2/bin/anvil "$fake_brew/bin/anvil"
  printf '#!/usr/bin/env sh\nprintf "curl should not run\\n" >&2\nexit 99\n' > "$fake_bin/curl"
  chmod +x "$fake_bin/curl"

  set +e
  output=$(PATH="$fake_brew/bin:$fake_bin:/usr/bin:/bin" HOMEBREW_PREFIX="$fake_brew" sh "$ROOT/install.sh" 2>&1)
  status=$?
  set -e

  rm -rf "$tmp"

  if [[ $status -ne 0 ]]; then
    printf 'Expected status 0, got %s\nOutput:\n%s\n' "$status" "$output" >&2
    exit 1
  fi
  assert_contains "$output" "Anvil is already installed via Homebrew"
  assert_contains "$output" "brew upgrade eddacraft/tap/anvil"
  assert_not_contains "$output" "curl should not run"
}

test_non_homebrew_install_still_runs_downloaded_installer() {
  local tmp fake_bin fake_installer output status marker
  tmp=$(mktemp -d)
  fake_bin="$tmp/bin"
  fake_installer="$tmp/fake-installer.sh"
  marker="$tmp/installer-ran"
  mkdir -p "$fake_bin"

  printf '#!/usr/bin/env sh\nwhile [ "$#" -gt 0 ]; do\n  if [ "$1" = "-o" ]; then\n    shift\n    cp "$FAKE_INSTALLER" "$1"\n    exit 0\n  fi\n  shift\ndone\nexit 2\n' > "$fake_bin/curl"
  chmod +x "$fake_bin/curl"
  printf '#!/usr/bin/env sh\ntouch "$INSTALL_MARKER"\n' > "$fake_installer"

  set +e
  output=$(PATH="$fake_bin:/usr/bin:/bin" FAKE_INSTALLER="$fake_installer" INSTALL_MARKER="$marker" sh "$ROOT/install.sh" 2>&1)
  status=$?
  set -e

  if [[ $status -ne 0 ]]; then
    printf 'Expected status 0, got %s\nOutput:\n%s\n' "$status" "$output" >&2
    rm -rf "$tmp"
    exit 1
  fi
  if [[ ! -f "$marker" ]]; then
    printf 'Expected fake installer to run\nOutput:\n%s\n' "$output" >&2
    rm -rf "$tmp"
    exit 1
  fi
  assert_contains "$output" "anvil installed successfully"
  rm -rf "$tmp"
}

test_homebrew_install_detected_before_download
test_homebrew_symlink_install_detected_before_download
test_non_homebrew_install_still_runs_downloaded_installer

printf 'install tests passed\n'
