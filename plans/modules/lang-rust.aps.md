<!--
APS Module: Rust Language Support
==================================
Extends Anvil's analysis to Rust codebases.
See: plans/aps-rules.md
-->

# Rust Language Support

| ID     | Owner | Status      |
| ------ | ----- | ----------- |
| RSTLAN | —     | Draft |

## Purpose

Extend Anvil to analyse Rust codebases. Rust's strong type system and ownership
model mean some anti-patterns (like `any`) don't apply, but architecture drift,
unsafe blocks, and suppression directives are real concerns. Rust's module system
(`mod`, `use`, `pub`) maps well to Anvil's layer/boundary model.

## In Scope

- Rust file extensions: `.rs`
- Import extraction: `use std::collections::HashMap`,
  `use crate::module::Type`, `mod module_name`, `pub use`,
  `extern crate` (legacy)
- Rust-specific anti-pattern detectors:
  - `unsafe` blocks (deliberate safety bypass)
  - `#[allow(...)]` directives (linter suppression)
  - `unwrap()` / `expect()` in non-test code (panic risk)
  - `todo!()` / `unimplemented!()` macros in production code
  - `as` type casts (potential data loss)
  - `#[cfg(test)]` patterns outside test modules
- Suppression syntax using `//` comments (already supported)
- Entry point detection: `fn main()` in `src/main.rs`, `Cargo.toml` bin targets
- Module boundary detection via `mod` declarations and `pub` visibility

## Out of Scope

- Cargo dependency resolution (cargo metadata integration)
- Macro expansion analysis
- Lifetime and borrow checker analysis
- Unsafe code auditing beyond detection
- proc-macro crate analysis

## Interfaces

**Depends on:**

- `save-time-trust` — runner and warning schema
- `antipattern-library` — scanner infrastructure
- `architecture-safety` — edge detector, layer detector
- `suppressions` — suppression parser
- `html-css-support` — configurable extensions infrastructure (HTMLCSS-001)

**Exposes:**

- Rust anti-pattern definitions
- Rust import extraction regexes (`use`, `mod`, `extern crate`)
- Rust entry point and module detection

## Prerequisites

- HTMLCSS-001 (configurable extensions) must be complete

## Estimated Scope

- **Anti-patterns:** 6 new patterns
- **Edge detection:** 3-4 new import regexes + crate-relative path resolution
- **Suppression:** None needed — Rust uses `//` comments which are already
  supported
- **Entry points:** New detection for `Cargo.toml` and `fn main()`
- **Effort:** 1-2 weeks

## Tasks

Tasks will be defined when this module moves to Ready status. Expected
breakdown:

- RSTLAN-001: Rust `use`/`mod` import extraction regexes
- RSTLAN-002: Rust anti-pattern catalogue
- RSTLAN-003: Rust entry point and module detection (`Cargo.toml`, `fn main()`)
- RSTLAN-004: Rust crate-relative path resolution (`crate::`, `super::`,
  `self::`)
- RSTLAN-005: Tests and documentation

## Risks

| Risk                           | Impact | Mitigation                                    |
| ------------------------------ | ------ | --------------------------------------------- |
| `use` paths need crate context | Medium | Read `Cargo.toml` for crate name mapping      |
| `unsafe` false positives       | Medium | Allowlist for FFI crates; severity: info       |
| Macro-generated imports missed | Low    | Regex can't see macro expansion; document      |

## Open Questions

- [ ] Should `unsafe` detection distinguish `unsafe fn` from `unsafe {}` blocks?
- [ ] How to handle workspace vs single-crate Cargo projects?
- [ ] Should `clippy::allow` attributes be treated like `#[allow()]`?
