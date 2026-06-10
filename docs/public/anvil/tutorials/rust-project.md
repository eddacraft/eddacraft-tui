---
id: rust-project
title: Analyse a Rust Project
description:
  Point anvil at a real Rust repo — discovery scan, the advisory Rust rule
  catalogue, and what the language profile claims.
sidebar_position: 8
---

# Analyse a Rust Project

Rust is a supported analysis language in the current beta: an AST-aware
antipattern catalogue, `.rs` files in the default scan set, and symbol/import,
entry-point, and layer/boundary analysis. This tutorial walks a real Rust
repository through the discovery path and ends where daily value starts.

## Prerequisites

- anvil installed and authenticated — steps 1 and 2 of the
  [Quickstart](../quickstart.md)
- A Rust project (anything with a `Cargo.toml` and `.rs` sources)

## 1. Two ways in: guided or direct

From the root of your Rust repo, the guided route is the welcome surface:

```bash
anvil welcome
```

`anvil welcome` runs a discovery scan over your repo and walks you through what
it finds. If you prefer the direct route, run the scanner yourself:

```bash
anvil check --all
```

Both routes use the same registry-backed rules; the rest of this tutorial uses
`anvil check` so the output is easy to follow.

## 2. What the Rust catalogue looks for

The Rust rules target the failure modes Rust code actually ships:

- **RS-001** — `unwrap()` or `expect()` in non-test code (info)
- **RS-002** — `panic!()` reached from non-test code (info)
- **RS-003** — `unsafe` block without a `// SAFETY:` comment (info)
- **RS-004** — `Deserialize` struct without `deny_unknown_fields` (info)
- **RS-005** — `todo!()` or `unimplemented!()` shipped (warning)

RS-001 through RS-004 are **AST rules**: anvil parses your Rust source and
matches the syntax tree, not text. For the panic-shaped rules — RS-001 and
RS-002 — that is what lets anvil exclude test code automatically: `#[cfg(test)]`
modules and `tests/`, `benches/`, `examples/`, and `build.rs` targets are not
flagged, because a panic there is a test failure or an idiomatic build-time
abort, not shipped runtime risk. RS-003, RS-004, and RS-005 apply to all scanned
code, test paths included — an undocumented `unsafe` block is worth a comment
wherever it lives.

These rules emit at **advisory severity** (`info`, with RS-005 a `warning`) in
this beta deliberately: on an established codebase, escape-hatch counts run
high, and a blocking first contact would be noise. By default only
`error`-severity findings fail the command (opt in to stricter blocking with
`--severity warning` or `--severity info`) — advisory findings tell you where
the risk concentrates.

## 3. Run the scan and read the findings

Info-severity findings are counted in the default summary; to list each one, add
`--verbose`:

```bash
anvil check --all --verbose
```

A typical Rust repo produces something like:

```
Checked 214 file(s)

Warnings
────────────────────────────────────────
  ⚠ [RS-005] todo!() or unimplemented!() shipped
  src/exporter.rs:88
  Found todo!() or unimplemented!() shipped at line 88

Info
────────────────────────────────────────
  ℹ [RS-001] unwrap() or expect() in non-test code
  src/config.rs:41
  Found unwrap() or expect() in non-test code at line 41

Summary
────────────────────────────────────────
  Total            2
  Warnings         1
  Info             1
  Time             180ms
```

Each finding carries a nudge with the idiomatic fix — for RS-001: propagate with
`?`, or convert deliberately with `ok_or(...)` / `map_err(...)` so the caller
decides. When a finding is genuinely infallible at that point, suppress it in
place with the reason that makes it so:

```rust
let port = bounds_checked_port.unwrap(); // @anvil-ignore RS-001 -- bounds checked above
```

## 4. What the language profile claims

Activation surfaces tell you exactly what coverage you are getting. Run the
read-only probe:

```bash
anvil start --verify
```

The diagnostic includes a per-language breakdown of your repo:

```
  languages:
    Rust (214 files): supported — antipattern + secret checks ship
```

`supported` is a claim with a stated basis, not a badge: for Rust it means the
antipattern catalogue above plus secret detection run over your `.rs` files.
Languages anvil cannot yet cover honestly say so (Python repos report
`unsupported` in this release) — the profile never claims coverage it does not
have.

Architecture and boundary checks for Rust — like every language — run when an
`.anvil/architecture.yaml` is present; see
[Architecture Boundaries](architecture.md) to set one up.

## 5. The daily-value handoff

Discovery shows you what is already there; the daily win is catching the next
escape hatch as it happens. Activate protection:

```bash
anvil start
```

`anvil start` baselines the repo — existing findings are recorded as the current
posture, not re-reported — and wires up save-time and MCP pre-write validation
so new regressions surface as you (or your AI agent) work. To see that loop
catch a save in real time, follow
[Your First Save Caught](first-save-caught.md).
