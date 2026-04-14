---
id: upgrade-notes
title: Upgrade Notes
description: Migration guides for major anvil versions.
sidebar_position: 2
---

# Upgrade Notes

Guides for upgrading between anvil versions.

## Current Version: 0.3.1-beta

## Upgrading to 0.3.1-beta

Drop-in upgrade from 0.3.0-beta. No configuration changes required.

```bash
# Upgrade via the installer (overwrites existing binary)
curl -fsSL https://install.eddacraft.ai | sh

# Or via Homebrew
brew upgrade eddacraft/tap/anvil

# Or via the built-in updater
anvil-update
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

**Major change:** anvil is now a native Rust binary. The Node.js package
(`@eddacraft/anvil-cli`) is deprecated and will not receive further updates.

```bash
# Install the native binary
curl -fsSL https://install.eddacraft.ai | sh

# Remove the old Node.js package (global)
npm uninstall -g @eddacraft/anvil-cli
# or if installed as a project dependency:
# pnpm remove @eddacraft/anvil-cli
```

Your `.anvilrc` and `.anvil/` directory work without changes. Authentication
tokens are migrated automatically on first run.

For full details, see [The Switch to Rust](./rust-rewrite.md).

### What's New

- **Native binary** — 5–10x faster scanning, 80% less memory in watch mode, no
  Node.js dependency.
- **Kernel engine** — persistent daemon with incremental parsing and real-time
  semantic graph updates.
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

- **Installation method** — `npm i -g @eddacraft/anvil-cli` no longer works. Use
  the install script or Homebrew.
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
