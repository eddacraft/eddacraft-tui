---
id: upgrade-notes
title: Upgrade Notes
description: Migration guides for major Anvil versions.
sidebar_position: 2
---

# Upgrade Notes

Guides for upgrading between Anvil versions.

## Current Version: 0.3.0-beta

## Upgrading to 0.3.0-beta

**Major change:** Anvil is now a native Rust binary. The Node.js package
(`@eddacraft/anvil-cli`) is deprecated and will not receive further updates.

```bash
# Install the native binary
curl -fsSL https://install.eddacraft.ai | sh

# Remove the old Node.js package
pnpm remove @eddacraft/anvil-cli
# or: npm uninstall @eddacraft/anvil-cli
```

Your `.anvilrc` and `.anvil/` directory work without changes. Authentication
tokens are migrated automatically on first run.

For full details, see [The Switch to Rust](./rust-rewrite.md).

### What's New

- **Native binary** — 5-10x faster scanning, 80% less memory in watch mode, no
  Node.js dependency.
- **Kernel engine** — persistent daemon with incremental parsing and real-time
  semantic graph updates.
- **Ratatui TUI** — rebuilt interactive surfaces with the EddaCraft Terminal
  Standard design system.
- **Structured exit codes** — `0` (pass), `1` (error), `2` (gate fail),
  `3` (auth required), `4` (config error).
- **Cross-platform auth** — device-flow authentication with OS keychain storage.

### Breaking Changes

- **Installation method** — `npm i -g @eddacraft/anvil-cli` no longer works.
  Use the install script or Homebrew.
- **CI workflows** — replace `pnpm anvil` / `npx anvil` with direct `anvil`
  calls. Remove Node.js setup steps if Anvil was the only reason they existed.

## Upgrading to 0.2.1-beta

Drop-in upgrade from any previous 0.2.x version. No configuration changes
required.

### What's New in 0.2.1

- **Project memory** — Anvil now tracks patterns and decisions in your codebase.
  New commands: `anvil edda`, `anvil ember`, `anvil stack`.
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
2. Search [existing issues](https://github.com/EddaCraft/anvil-001/issues)
3. Open a new issue with:
   - Old version
   - New version
   - Error message
   - Steps to reproduce
