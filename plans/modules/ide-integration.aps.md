<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# IDE Integration

| Scope | Owner | Priority | Status |
| ----- | ----- | -------- | ------ |
| IDE   | —     | medium   | Draft  |

## Purpose

Surface Anvil warnings directly in the developer's editor at file-save time. The
primary feedback loop — developers see issues before they leave the file.

## In Scope

- VS Code extension scaffold
- On-save trigger integration
- Diagnostics panel integration
- Inline warning display

## Out of Scope

- JetBrains plugin (separate module)
- Auto-fix actions (v2)
- CodeLens decorations (v2)

## Interfaces

**Depends on:**

- `save-time-trust` — analysis runner and warning schema

**Exposes:**

- VS Code extension package
- Language server protocol (LSP) diagnostics

## Acceptance Criteria

- [ ] Extension installs from VSIX
- [ ] Warnings appear on file save
- [ ] Warnings show in Problems panel
- [ ] Clicking warning navigates to location

## Tasks

_Tasks to be defined when module status is Ready._
