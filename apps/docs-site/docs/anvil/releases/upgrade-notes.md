---
id: upgrade-notes
title: Upgrade Notes
description: Migration guides for major Anvil versions.
sidebar_position: 2
---

# Upgrade Notes

Guides for upgrading between major Anvil versions.

## Upgrading to 1.0

### From 0.9.x

Anvil 1.0 is the first stable release. If you were using the beta:

#### Configuration Changes

```json
// Old (0.9.x)
{
  "checks": {
    "architecture": { ... }
  }
}

// New (1.0)
{
  "gates": {
    "architecture": { ... }
  }
}
```

**Migration:**

```bash
# Rename "checks" to "gates" in anvil.config.json
```

#### CLI Changes

| Old Command           | New Command   |
| --------------------- | ------------- |
| `anvil check`         | `anvil run`   |
| `anvil check --watch` | `anvil watch` |

#### Evidence Format

Evidence format changed. Old evidence files are not compatible.

**Migration:**

```bash
# Clear old evidence
rm -rf .anvil/evidence

# Run fresh
anvil run
```

### From No Previous Version

If you're new to Anvil, start with:

```bash
pnpm add -D @anvil/cli
anvil init
anvil run
```

## Future Versions

This section will be updated with each major release.

## Getting Help

If you encounter upgrade issues:

1. Check the [Troubleshooting guide](/docs/anvil/operations/troubleshooting)
2. Search [existing issues](https://github.com/EddaCraft/anvil-001/issues)
3. Open a new issue with:
   - Old version
   - New version
   - Error message
   - Steps to reproduce
