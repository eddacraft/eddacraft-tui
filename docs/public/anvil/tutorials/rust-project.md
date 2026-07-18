---
id: rust-project
title: Check a Rust project
description:
  Run a first named-file and changed-file anvil check in a Rust project.
---

# Check a Rust project

**For:** Rust developers

**Time:** 10 minutes

**Outcome:** inspect a Rust result without changing Cargo configuration

## Before you begin

Complete the [quickstart](../quickstart.md) and open a Rust project.

## 1. Check one source file

```text
anvil check src/main.rs --format plain
```

For a library crate, choose an existing file such as `src/lib.rs`. Success is a
finding list or an explicit clean result.

## 2. Check the current change

```text
anvil check --changed --format plain
```

Explicit file paths take precedence over changed-file flags, so keep these as
separate commands.

## 3. Run a development gate

```text
anvil gate --profile dev --format plain
```

anvil does not replace `cargo check`, `cargo test`, or `cargo clippy`. Run the
normal Rust verification as well.

## Coverage boundary

Rust participates in parsing, architecture evidence, and compiled rules shown by
the [support matrix](../reference/support.md) and
[rule catalogue](../reference/rules.md).

## Next step

Define [architecture boundaries](../first-project.md) for a layered crate.
