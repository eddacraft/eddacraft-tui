---
id: upgrade-notes
title: Upgrade Notes
description: Migration guides for major anvil versions.
sidebar_position: 2
---

# Upgrade Notes

Guides for upgrading between anvil versions.

## Current Version: 0.4.0-beta

## Upgrading to 0.4.0-beta

Drop-in upgrade from `0.3.3-beta` for most users. Two behavioural changes
require attention:

- **`anvil watch --exclude` now takes glob patterns, not bare directory
  names.** A previous `--exclude vendor` no longer excludes files under
  `vendor/`; use `--exclude 'vendor/**'` instead. The CLI prints a
  warning when a likely-bare-name pattern is detected.
- **`anvil doctor --json` output shape changed** from a bare array to
  `{ "checks": [...], "notifications": [...] }`. Consumers iterating the
  array must switch to `data.checks`.

```bash
# Upgrade via the installer
curl -fsSL https://install.eddacraft.ai | sh

# Or via the built-in updater
anvil update

# Or via Homebrew
brew upgrade eddacraft/tap/anvil
```

```powershell
# Windows (PowerShell installer)
irm https://install.eddacraft.ai/windows | iex

# Or via WinGet
winget upgrade eddacraft.anvil

# Or via Scoop
scoop update anvil
```

### What's New in 0.4.0-beta

- **`anvil watch --patterns / --exclude`** — user-supplied glob filter
  on the watch loop. Previously the flags were declared but never read;
  watch silently used a hardcoded scope.
- **Post-init auto-analysis** — `anvil init` now runs an inline first
  scan and surfaces a real signal (top warnings + counts) rather than
  pointing at `anvil doctor`.
- **Doctor structured remediation** — every `anvil doctor` check emits
  a concrete remediation field (link, command, or auto-fix prompt);
  no check terminates at "see README".
- **`anvil watch` startup banner** — prints active include / exclude
  scope so the active filter is visible at a glance.
- **Workspace hardening** — cargo-hakari workspace-hack, cargo-deny
  policy, third-party notices via cargo-about (RUSTNX).

## Upgrading to 0.3.3-beta

Drop-in upgrade from `0.3.2-beta`. No configuration migration is required.

```bash
# Upgrade via the installer
curl -fsSL https://install.eddacraft.ai | sh

# Or via the built-in updater
anvil update

# Or via Homebrew
brew upgrade eddacraft/tap/anvil
```

```powershell
# Windows (PowerShell installer)
irm https://install.eddacraft.ai/windows | iex

# Or via WinGet
winget upgrade eddacraft.anvil

# Or via Scoop
scoop update anvil
```

### What's New in 0.3.3-beta

- **Windows distribution** — WinGet landed and Scoop became part of the
  documented install/upgrade story.
- **Admin operations** — the separate `anvil-admin` operator CLI gained
  list/show/invite/audit/revoke and migration tooling.
- **Windows UX fixes** — onboarding, discovery, and key handling improved.

## Upgrading to 0.3.2-beta

Drop-in upgrade from `0.3.1-beta`. No configuration migration is required.

```bash
# Upgrade via the installer
curl -fsSL https://install.eddacraft.ai | sh

# Or via the built-in updater
anvil update

# Or via Homebrew
brew upgrade eddacraft/tap/anvil
```

## Upgrading to 0.3.1-beta

Drop-in upgrade from 0.3.0-beta. No configuration changes required.

```bash
# Upgrade via the installer (overwrites existing binary)
curl -fsSL https://install.eddacraft.ai | sh

# Or via Homebrew
brew upgrade eddacraft/tap/anvil

# Or via the built-in updater
anvil update
```

### What's New in 0.3.1-beta

- **Docs domain cutover** — `docs.eddacraft.ai` now served via a dedicated proxy
  with a Nordic terminal-themed landing page.
- **Welcome screen fixes** — first-user onboarding flows restored after
  regressions in 0.3.0-beta.
- **Auth error messages** — clearer error messages during login and device-code
  flows.
- **TUI version display** — shell footer now shows the correct version.

No breaking changes. All existing configuration, credentials, and workflows
continue to work without modification.

## Upgrading to 0.3.0-beta

`0.3.0-beta` was the release where anvil became a native Rust binary. Current
docs assume a fresh install on the Rust CLI rather than a staged migration from
the legacy Node.js package.

```bash
# Install the native binary
curl -fsSL https://install.eddacraft.ai | sh
```

If an older npm-installed `anvil` is still earlier on your `PATH`, remove
`@eddacraft/anvil-cli` and re-run `anvil --version` so you know the native
binary is the command being executed.

Your `.anvilrc` and `.anvil/` directory work without changes.

For full details, see [The Switch to Rust](./rust-rewrite.md).

### What's New

- **Native binary** — 5–10x faster scanning, 80% less memory in watch mode, no
  Node.js dependency.
- **Kernel engine** — foreground watch mode, incremental parsing, and real-time
  semantic graph updates in the native Rust runtime.
- **Ratatui TUI** — rebuilt interactive surfaces with the eddacraft Terminal
  Standard design system.
- **Welcome & onboarding** — first-run interactive experience; run
  `anvil welcome` anytime.
- **New commands** — `anvil new`, `anvil wizard`, `anvil audit`, `anvil drift`,
  `anvil validate`, `anvil gate-config`.
- **Structured exit codes** — `0` (pass), `1` (error), `2` (gate fail), `3`
  (auth required), `4` (config error).
- **Beta auth** — device-flow and OTP authentication with OS keychain storage.

### Breaking Changes

- **Installation method** — install anvil as a native binary via the installer,
  Homebrew, WinGet, or Scoop.
- **CI workflows** — replace `pnpm anvil` / `npx anvil` with direct `anvil`
  calls. Remove Node.js setup steps if anvil was the only reason they existed.
- **Docs access** — the `/anvil` documentation is now gated behind GitHub OAuth
  for beta users. Sign in with the GitHub account tied to your beta invite.
  Public eddacraft docs (APS, Kindling, edda-stack) remain open.

## Upgrading to 0.2.1-beta

Drop-in upgrade from any previous 0.2.x version. No configuration changes
required.

### What's New in 0.2.1

- **Project memory** — anvil now tracks patterns and decisions in your codebase
  via the Edda memory system and Ember proposal engine.
- **Security hardening** — input validation and subprocess execution
  improvements across the platform.
- **Dependency patches** — minimatch, axios, svgo, tar, and others.

No breaking changes. The new memory features are opt-in and do not affect
existing scanning behaviour.

## Upgrading to 0.1.2-beta

This was the first public beta. No breaking migrations from alpha beyond the
configuration key change below.

### Note for Early Alpha Testers

If you used an internal alpha build, the top-level configuration key changed
from `"checks"` to `"gates"`:

```json
// Old (alpha)
{
  "checks": {
    "architecture": { ... }
  }
}

// Current (0.1.x-beta)
{
  "gates": {
    "architecture": { ... }
  }
}
```

Run `anvil init --force` to regenerate your configuration, or rename the key
manually in `.anvilrc`.

## Future Versions

Upgrade guides are added here as new versions ship.

## Getting Help

If you encounter upgrade issues:

1. Check the [Troubleshooting guide](/anvil/operations/troubleshooting)
2. Search [existing issues](https://github.com/eddacraft/anvil/issues)
3. Open a new issue with:
   - Old version
   - New version
   - Error message
   - Steps to reproduce

---

**See also:** [Changelog](/anvil/releases/changelog),
[The Switch to Rust](/anvil/releases/rust-rewrite)
