---
id: rust-rewrite
title: Native Rust migration
description:
  Historical context for the move from the retired package runtime to the native
  anvil binary.
owner: DOCSYNC
upstream:
  - crates/anvil-cli/src/commands/migrate.rs
  - dist-workspace.toml
verified_against: 0.9.0-beta
---

# Native Rust migration

anvil is distributed as a self-contained Rust binary. Historical package-runner
commands from early previews are not part of the supported public workflow.

## What changed for users

- Install and upgrade the native binary through a supported installer or package
  manager.
- Run `anvil` directly.
- Use `anvil --version` to verify which binary is active.
- Keep normal project toolchains for the lint, test, and build commands that
  gates invoke.

## Migrating an old installation

1. Remove the retired package installation with its original package manager.
2. Follow the current [quickstart](../quickstart.md).
3. Run `anvil doctor`.
4. Review project configuration and run `anvil migrate --help` if the installed
   version requests a migration.
5. Verify with `anvil start --verify`.

## Next step

Use the [current upgrade guide](upgrade-notes.md) for later native versions.
