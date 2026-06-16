# Supply-Chain Policy

| Type  | Authority     | Owner | Status | Freshness                                         |
| ----- | ------------- | ----- | ------ | ------------------------------------------------- |
| Guide | Authoritative | SEC   | Live   | First filed 2026-06-16 against `main` for SEC-004 |

| Upstream                                                                                                                                 | Downstream                                                                                            |
| ---------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| [`deny.toml`](../../deny.toml), [`licences.toml`](../../licences.toml), [`.github/workflows/rust.yml`](../../.github/workflows/rust.yml) | [`dependency-audit-posture.md`](./dependency-audit-posture.md), supply-chain-attestation (SCA) module |

## Overview

This guide documents the supply-chain rules Anvil already enforces, so a change
to the enforcement config has a referenced rationale rather than an unexplained
edit. The runtime enforcement lives in [`deny.toml`](../../deny.toml) (Rust, via
the `cargo-deny` CI job) and in the lockfile discipline both package managers
apply in CI. This guide is the **policy** half of supply-chain security; the
**attestation/SBOM** half (build provenance) is owned by the
supply-chain-attestation (SCA) track — the boundary is called out below.

## Lockfile policy

- **Frozen in CI.** Both ecosystems install with a frozen lockfile —
  `pnpm install --frozen-lockfile` for JS and Cargo's locked resolution for Rust
  — so CI never silently resolves a different dependency set than the committed
  lockfiles (`pnpm-lock.yaml`, `Cargo.lock`).
- **Who updates.** Lockfile bumps land through Dependabot PRs (npm, cargo,
  github-actions; see the
  [dependency-audit posture](./dependency-audit-posture.md)) or a deliberate
  manual bump, reviewed like any other change. A drive-by lockfile change
  unrelated to the PR's purpose should be split out.

## `deny.toml` rules

[`deny.toml`](../../deny.toml) is the Rust supply-chain gate (enforced by the
`cargo-deny` job in
[`.github/workflows/rust.yml`](../../.github/workflows/rust.yml)). It carries
four policy axes:

- **`[advisories]`** — fails on a RUSTSEC advisory. To unblock CI while a fix is
  worked, add a _time-boxed_ ignore entry with a tracking-issue URL and an
  expected-fix date; prefer bumping the dependency. This is an escape hatch, not
  a permanent bypass.
- **`[licenses]`** — the allowlist of permitted SPDX licences. The `allow` array
  is **generated** from [`licences.toml`](../../licences.toml) (kept in sync
  with `about.toml`); edit `licences.toml`, never `deny.toml` directly, when
  adding or removing a licence. `[[licenses.clarify]]` records per-crate
  clarifications.
- **`[bans]`** — explicitly disallowed crates and duplicate-version policy.
- **`[sources]`** — which registries/git sources are permitted, so a dependency
  cannot be pulled from an unexpected origin.

### Amending the rules

Changing a `deny.toml` axis is a deliberate act: state the reason in the PR,
prefer the narrowest change (a single time-boxed advisory ignore over a broad
suppression), and for licences edit the `licences.toml` source so the allowlist
stays generated.

## Registries

The workspace resolves from the public registries (crates.io for Rust, the npm
registry for JS) under the `[sources]` constraints above. A private/internal
registry has **not** been adopted; if that changes it must be recorded here and
reflected in `deny.toml`'s `[sources]`.

## Transitive-dependency posture

Transitive dependencies are governed by the same gates as direct ones — the
advisory, licence, ban, and source rules apply across the whole resolved graph,
not just top-level manifests. Trivy's lockfile scan and `cargo-deny`'s
graph-wide evaluation are the enforcement points.

## Boundary with attestation (SCA)

This guide covers **policy** (what is allowed and how it is enforced). Build
**provenance** — SBOM generation, signed attestations, and release artefact
digests — is owned by the supply-chain-attestation (SCA) track and is out of
scope here. The two together form supply-chain security: policy gates what
enters the graph; attestation proves what the build produced.
