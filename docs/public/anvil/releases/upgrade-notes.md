---
id: upgrade-notes
title: Upgrade Notes
description: Migration guides for major Anvil versions.
sidebar_position: 2
---

# Upgrade Notes

Guides for upgrading between Anvil versions.

## Current Version: 0.1.2-beta

This is the first public beta. There are no breaking migrations yet.

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

Upgrade guides will be added here as new versions ship.

## Getting Help

If you encounter upgrade issues:

1. Check the [Troubleshooting guide](/anvil/operations/troubleshooting)
2. Search [existing issues](https://github.com/EddaCraft/anvil-001/issues)
3. Open a new issue with:
   - Old version
   - New version
   - Error message
   - Steps to reproduce
