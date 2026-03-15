#!/bin/sh
# Anvil CLI installer
# https://github.com/EddaCraft/anvil-001
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/EddaCraft/anvil-001/main/install.sh | sh

set -e

PACKAGE="@eddacraft/anvil-cli@latest"
MIN_NODE_MAJOR=20
MIN_NODE_MINOR=19

# --- Helpers ----------------------------------------------------------------

info()  { printf '[*] %s\n' "$1"; }
ok()    { printf '[✓] %s\n' "$1"; }
warn()  { printf '[!] %s\n' "$1"; }
fail()  { printf '[✗] %s\n' "$1" >&2; exit 1; }

# --- OS detection -----------------------------------------------------------

detect_os() {
  case "$(uname -s)" in
    Darwin)  OS="macos" ;;
    Linux)   OS="linux" ;;
    CYGWIN*|MINGW*|MSYS*|Windows_NT)
      warn "Windows detected. Install via npm instead:"
      info "  npm i -g ${PACKAGE}"
      exit 0
      ;;
    *)
      fail "Unsupported operating system: $(uname -s)"
      ;;
  esac

  # WSL is already caught by the Linux branch above; confirm for clarity
  if [ "$OS" = "linux" ] && [ -f /proc/version ] && grep -qi microsoft /proc/version 2>/dev/null; then
    info "WSL detected — proceeding as Linux"
  fi

  ok "Operating system: ${OS}"
}

# --- Node.js check ----------------------------------------------------------

check_node() {
  if ! command -v node >/dev/null 2>&1; then
    fail "Node.js is not installed. Install Node >= ${MIN_NODE_MAJOR}.${MIN_NODE_MINOR}.0 from https://nodejs.org"
  fi

  node_version="$(node -v | sed 's/^v//')"
  node_major="$(printf '%s' "$node_version" | cut -d. -f1)"
  node_minor="$(printf '%s' "$node_version" | cut -d. -f2)"

  if [ "$node_major" -lt "$MIN_NODE_MAJOR" ] 2>/dev/null || \
     { [ "$node_major" -eq "$MIN_NODE_MAJOR" ] && [ "$node_minor" -lt "$MIN_NODE_MINOR" ]; } 2>/dev/null; then
    fail "Node.js ${node_version} found, but >= ${MIN_NODE_MAJOR}.${MIN_NODE_MINOR}.0 is required. Update at https://nodejs.org"
  fi

  ok "Node.js ${node_version}"
}

# --- npm check --------------------------------------------------------------

check_npm() {
  if ! command -v npm >/dev/null 2>&1; then
    fail "npm is not installed. It ships with Node.js — reinstall from https://nodejs.org"
  fi

  ok "npm $(npm -v)"
}

# --- Install ----------------------------------------------------------------

install_cli() {
  info "Installing ${PACKAGE} ..."

  if ! npm i -g "${PACKAGE}"; then
    echo ""
    warn "Global install failed. Common fixes:"
    info "  - Run with sudo: curl -fsSL ... | sudo sh"
    info "  - Fix npm permissions: https://docs.npmjs.com/resolving-eacces-permissions-errors-when-installing-packages-globally"
    exit 1
  fi

  ok "Installed ${PACKAGE}"
}

# --- Verify -----------------------------------------------------------------

verify() {
  if ! command -v anvil >/dev/null 2>&1; then
    warn "anvil command not found in PATH after install"
    info "You may need to restart your shell or add npm's global bin to PATH"
    exit 1
  fi

  installed_version="$(anvil --version 2>/dev/null || echo "unknown")"
  ok "anvil ${installed_version}"
}

# --- Main -------------------------------------------------------------------

main() {
  echo ""
  echo "  Anvil CLI Installer"
  echo "  ==================="
  echo ""

  detect_os
  check_node
  check_npm
  install_cli
  verify

  echo ""
  ok "Installation complete"
  echo ""
  info "Get started:"
  info "  anvil login"
  echo ""
}

main
