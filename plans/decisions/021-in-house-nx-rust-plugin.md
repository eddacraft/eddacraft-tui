# ADR-021: In-house `@eddacraft/nx-rust` Nx plugin

## Status

Proposed

> **Renumbered (DOCGOV-004, 2026-05-12):** originally drafted as ADR-026; the
> number was already taken by [ADR-026 — Rust Scanner is
> Authoritative](./026-rust-scanner-authoritative.md), which was merged first.
> This ADR was renumbered to fill the previously unassigned ADR-021 slot.

> **Amended (DEVENV-003, 2026-05-29):** the decision (build an in-house Nx-Rust
> plugin) stands, but the plugin no longer lives **in-repo**. It was extracted
> to the standalone public repo [`eddacraft/nxrust`](https://github.com/eddacraft/nxrust)
> and is consumed from the registry as `@eddacraft/nxrust` (registered in
> `nx.json`). The original vendored `tools/nx-rust/` copy (`@eddacraft/nx-rust`,
> hyphenated) became dead code — referenced by nothing — and was **removed** in
> the DEVENV-003 change. Treat `eddacraft/nxrust` as the source of truth;
> plugin changes (e.g. `CARGO_TARGET_DIR`-aware build outputs) happen there and
> reach anvil via a dependency bump. See
> [ADR-057](./057-dev-environment-hardening.md) §3.

## Date

2026-04-21

## Context

The Anvil monorepo runs 15 TypeScript projects under Nx 22.6.5 with an
Azure Blob remote cache (`@nx/azure-cache`) and custom workspace
generators at `tools/generators/` (`@eddacraft/anvil-generators`,
`PROPRIETARY`). It also contains a Cargo workspace with 9 Rust crates
that are currently invisible to Nx — CI builds them via raw `cargo`
commands in `.github/workflows/rust.yml`, with no cross-crate affected
detection and no remote-cache participation.

The RUSTNX module (`plans/modules/rust-nx-migration.aps.md`) plans to
bring the 9 crates under Nx so they benefit from affected-only CI and
remote caching. Two work items inside RUSTNX — RUSTNX-004 "Scaffold
per-crate project.json wrappers" and RUSTNX-005 "Configure Nx inputs,
outputs, and remote cache for Rust" — were originally scoped as
hand-rolled across all 9 crates. Duplicating that wiring 9 times, and
owning every cargo-options code path by hand, is not the right long-term
shape.

Two off-the-shelf alternatives were evaluated:

- **`cargo-make`** (https://github.com/sagiegurari/cargo-make) — a
  Rust-focused task runner. It doesn't orchestrate the TypeScript side
  of the monorepo, doesn't integrate with `@nx/azure-cache`, and
  doesn't provide `nx affected`. Replacing Nx with it would leave 15
  TS projects unorchestrated.
- **`@monodon/rust`** (https://github.com/Cammisuli/monodon) — a
  community Nx plugin wrapping cargo with `binary`/`library` generators
  and `build`/`test`/`lint`/`run` executors. API shape matches what we
  need. However: the upstream repository ships with no LICENSE file,
  which means no legal grant to use it in a commercial product; the
  project has not cut a release in over a year; and it has a backlog of
  open issues. Commercial and technical risk both unacceptable.

A decision is needed now because RUSTNX is Ready with a 0/9 progress
count, and its Tier 2 work items are blocked on choosing between
hand-rolled wiring and a plugin. This ADR captures that choice so the
rationale is durable.

## Decision

Build an in-house Nx plugin, `@eddacraft/nx-rust`, at `tools/nx-rust/`,
modelled on the shape of `@monodon/rust` (API surface only — no source
copy) and sibling to the existing `@eddacraft/anvil-generators`.

Scope:

- Executors wrapping cargo for `build`, `test`, `check`, `clippy`, and
  `fmt`
- A single `crate` generator for scaffolding new workspace-member
  crates
- A project-graph plugin that parses `crates/*/Cargo.toml` to emit
  Rust-to-Rust dependency edges so `nx affected` is correct

Cargo stays as the build engine. Nx stays as the orchestrator. The
plugin is intra-repo (never published), marked `PROPRIETARY`, depends
on `@nx/devkit ^22.6.5`.

RUSTNX-004 and RUSTNX-005 are rewritten to consume this plugin rather
than hand-roll wiring. Implementation is tracked as module NXRUST
(`plans/modules/nx-rust-plugin.aps.md`).

## Rationale

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **In-house `@eddacraft/nx-rust` plugin (chosen)** | Centralised executor logic; licence-clean (`PROPRIETARY`); matches the existing `@eddacraft/anvil-generators` pattern; we control Nx-version compatibility; can be tuned to our exact Cargo workspace shape | We own the maintenance; Nx major-version upgrades become our problem; some upfront engineering (~8 work items) |
| `@monodon/rust` (rejected) | Off-the-shelf; covers the same API surface | **No LICENSE file in upstream — no legal grant of use**; no release >1 year; large open-issue backlog; unclear Nx 22 compatibility |
| `cargo-make` (rejected) | Healthy upstream; cross-platform task scripting; replaces ad-hoc shell glue | Rust-only — does not substitute for Nx; would leave 15 TS projects unorchestrated; no integration with `@nx/azure-cache`; no `nx affected` |
| Hand-rolled per-crate `project.json` (rejected) | No new package; explicit for readers | Duplicated cargo-wrapping logic across 9 crates; input/output declarations scattered across `nx.json`; every change touches many files |
| Do nothing — keep Rust outside Nx | No work | Leaves RUSTNX blocked; Rust CI keeps paying full cold-cache cost; affected detection never works across TS + Rust |

The in-house plugin trades "off-the-shelf convenience" for "durable
ownership of a licence-clean, correctly-sized tool." Given our Cargo
workspace is stable at 9 crates and the executor surface is small, the
upfront cost is bounded.

## Consequences

- **Positive:**
  - One place to change cargo-invocation logic instead of 9
  - Licence-clean: no third-party licence uncertainty inherits into
    Anvil's commercial product
  - `nx affected` gains correct Rust-to-Rust visibility via the graph
    plugin, unlocking faster PR CI
  - Matches the existing `@eddacraft/anvil-generators` pattern at
    `tools/generators/` — no new architectural precedent
  - Unblocks RUSTNX-004 / -005 with a thinner, consumer-shaped spec

- **Negative:**
  - We own the plugin's maintenance, including Nx-version compatibility
  - Some upfront engineering (~8 work items in module NXRUST)
  - A third custom tools package in `tools/` (after `generators/` and
    `codemods/`) — acceptable but worth noting

- **Risks:**
  - Nx 22 project-graph plugin API drifts vs docs we reference
  - `target/` caching behaviour is notoriously fragile — pilot may
    force narrow `outputs` declarations rather than full build-output
    caching
  - Plugin may lag behind an Nx major upgrade if we don't maintain it

- **Mitigations:**
  - Clone the graph-plugin shape from `@nx/js` in `node_modules` — code
    against the installed API, not external docs
  - Ship a CI smoke test (NXRUST-007) that builds the plugin and runs
    one of its executors on every PR
  - Keep executor logic thin (shell out to cargo, inherit stdio) so the
    surface that must be re-audited on Nx upgrades is small
  - Add "verify `@eddacraft/nx-rust` still builds" to the Nx upgrade
    runbook

## References

- ADR-012 — Single `anvil` Rust binary replaces Node.js CLI
- ADR-014 — Language allocation (TS for orchestration, Rust for
  hot paths)
- `plans/modules/nx-rust-plugin.aps.md` — implementation module
  (NXRUST)
- `plans/modules/rust-nx-migration.aps.md` — consumer module
  (RUSTNX)
- `tools/generators/` — `@eddacraft/anvil-generators` precedent
- `nx.json` — plugin registration site
- Root `Cargo.toml` — 9-member workspace the graph plugin parses
- External:
  - https://github.com/sagiegurari/cargo-make (rejected)
  - https://github.com/Cammisuli/monodon (rejected — no LICENSE)
