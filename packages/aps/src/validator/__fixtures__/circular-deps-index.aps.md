# Project Plan

> Index for testing circular dependency detection.

## Modules

### module-a

- **Path:**
  [./circular-modules/module-a.aps.md](./circular-modules/module-a.aps.md)
- **Scope:** CIRC
- **Dependencies:** module-b

### module-b

- **Path:**
  [./circular-modules/module-b.aps.md](./circular-modules/module-b.aps.md)
- **Scope:** CIRC
- **Dependencies:** module-c

### module-c

- **Path:**
  [./circular-modules/module-c.aps.md](./circular-modules/module-c.aps.md)
- **Scope:** CIRC
- **Dependencies:** module-a
