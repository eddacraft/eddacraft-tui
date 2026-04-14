# anvil-kernel-types

Shared types for the Anvil Rust kernel — events, graph nodes, and trust levels.

## Modules

- **`events`** — kernel event types (file changes, parse results, policy
  violations)
- **`graph`** — graph node and edge type definitions
- **`trust`** — trust level enums and scoring

## Usage

This crate is a dependency of `anvil-kernel`, `anvil-tui`, and `anvil-cli`. It
contains no logic — only type definitions and serialisation derives.

## Part of

[eddacraft Anvil](../../README.md) monorepo (`crates/anvil-kernel-types`).
