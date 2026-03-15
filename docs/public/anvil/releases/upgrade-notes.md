---
id: upgrade-notes
title: Upgrade Notes
description: Migration guides for major Anvil versions.
sidebar_position: 2
---

# Upgrade Notes

Guides for upgrading between Anvil versions.

## Current Version: 0.2.1-beta

## Upgrading to 0.2.1-beta

Drop-in upgrade from any previous version. No configuration changes required.

```bash
npm i -g @eddacraft/anvil-cli@0.2.1-beta
```

Existing authentication tokens and `.anvilrc` settings continue to work
unchanged.

### What's New

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
