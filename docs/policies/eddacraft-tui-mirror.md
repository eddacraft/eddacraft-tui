# `eddacraft-tui` Mirror, CI Gates, and Backport Policy

| Type  | Authority     | Owner                                                                                                                 | Status | Freshness                                                                                            |
| ----- | ------------- | --------------------------------------------------------------------------------------------------------------------- | ------ | ---------------------------------------------------------------------------------------------------- |
| Guide | Authoritative | TUIR ([`plans/modules/tui-reintegration.aps.md`](../../plans/modules/tui-reintegration.aps.md)) and ADR-047 / ADR-050 | Live   | Last reviewed 2026-05-23 against [TUIR-006 / TUIR-007](../../plans/modules/tui-reintegration.aps.md) |

| Upstream                                                                                                                                                                                                                | Downstream                                                                                                                                                                                                                                                                                          |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [ADR-047](../../plans/decisions/047-eddacraft-tui-canonical-source-mirror.md), [ADR-050](../../plans/decisions/050-eddacraft-tui-runner-and-cli-policy.md), [TUIR module](../../plans/modules/tui-reintegration.aps.md) | [`crates/eddacraft-tui/CONTRIBUTING.md`](../../crates/eddacraft-tui/CONTRIBUTING.md), [`crates/eddacraft-tui/SECURITY.md`](../../crates/eddacraft-tui/SECURITY.md), [`crates/eddacraft-tui/MIRROR-README.md`](../../crates/eddacraft-tui/MIRROR-README.md), Anvil-side and mirror-side CI workflows |

This is the canonical statement of how the `eddacraft-tui` crate is governed
across its two surfaces — the canonical source in this monorepo at
`crates/eddacraft-tui/` and the public read-only mirror at
[`eddacraft/eddacraft-tui`](https://github.com/eddacraft/eddacraft-tui). It pins
which CI gates run where, what the backport contract is, and how drive-by
changes against the mirror are handled.

The policy is referenced from `crates/eddacraft-tui/CONTRIBUTING.md` **once
TUIR-007 lands** (in flight at PR #1886) and — after the mirror sync per
D-TUIR-012 — from the public `README.md` banner.

## Topology At A Glance

```text
crates/eddacraft-tui/        ← canonical source (this repo)
  → eddacraft/eddacraft-tui  ← read-only public mirror (force-pushed
                                from canonical; per D-TUIR-004)
  → crates.io                ← external distribution channel
                                (published from canonical; per D-TUIR-005)
```

- Canonical source is the **only** writable surface. All issues, PRs, and design
  discussion land in this repo.
- The public mirror is rewritten by automation on every change to
  `crates/eddacraft-tui/**` on `main`; direct commits and source PRs on the
  mirror are **not accepted**.
- Release tags use the prefixed form `eddacraft-tui-vX.Y.Z` (per D-TUIR-002);
  they are protected and never rewritten by the mirror job.
- Pre-cutover history on the public mirror is preserved on the
  `pre-canonical-archive` branch (per D-TUIR-010); old unprefixed `v0.x.y` tags
  remain in place untouched (per D-TUIR-011).

## CI Gate Contracts

The crate is gated on two sides. The Anvil side is **authoritative**; the mirror
side is **smoke-only**. Duplicating Anvil's full matrix on the mirror would be
theatre — Anvil already runs every gate on every PR before the change reaches
the mirror.

### Anvil side (this repo)

These gates run on every PR that touches `crates/eddacraft-tui/**` and must pass
before merge. They are NOT eddacraft-tui-specific workflow files — the
workspace-wide jobs in
[`.github/workflows/rust.yml`](../../.github/workflows/rust.yml) cover them,
with one dedicated step added for the per-crate all-features build (the
workspace gate runs default features per crate, so optional features like
`image`, `big-text`, and `test-utils` would otherwise go untested).

| Gate                                                    | Where it lives                                                | Why                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| ------------------------------------------------------- | ------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `cargo fmt --all --check`                               | `rust.yml` `format` job                                       | Workspace clippy with `-D warnings` does NOT run rustfmt; tests + clippy can be green while CI's Format check fails. Caught real regressions (PR #1724).                                                                                                                                                                                                                                                                                                  |
| `cargo clippy --workspace --all-targets -- -D warnings` | `rust.yml` `clippy` job                                       | Per-crate `-p` invocations miss doc-markdown errors in sibling crates (PR #1603 bit twice). Workspace-wide `--all-targets` covers tests, benches, and examples too.                                                                                                                                                                                                                                                                                       |
| `cargo test --workspace`                                | `rust.yml` `test` job                                         | Consumer-side regression catch. Exercises eddacraft-tui via anvil-tui + anvil-cli paths (per the TUIR-003 path-crate switch).                                                                                                                                                                                                                                                                                                                             |
| `cargo test -p eddacraft-tui --all-features`            | `rust.yml` `test` job (dedicated step)                        | Tests the optional features (`image`, `big-text`, `test-utils`) that the workspace gate would skip. Matches the standalone repo's `Check (all-features)` coverage.                                                                                                                                                                                                                                                                                        |
| `cargo check -p eddacraft-tui --all-features --release` | `rust.yml` `test` job (dedicated step)                        | Catches the `ids_are_unique`-class regression where a `#[cfg(debug_assertions)]` helper called from `debug_assert!` breaks release builds (the standalone repo's [issue #29](https://github.com/eddacraft/eddacraft-tui/issues/29), fixed in v0.2.2). Default workspace `check` runs the dev profile only. `--all-features` is load-bearing — the original `ids_are_unique` site was inside the `big-text` feature, so a feature-off check would miss it. |
| `cargo deny check`                                      | `rust.yml` `deny` job (CICD-007 conditional)                  | Workspace `deny.toml` covers eddacraft-tui's licence / advisory surface (no per-crate `deny.toml` migrated — TUIR-002 baseline correction). Runs only when manifests, lockfile, toolchain, or rust.yml itself changed on the PR (CICD-007 skip condition) — pure source-only changes to `crates/eddacraft-tui/**` skip this gate.                                                                                                                         |
| `cargo hakari verify`                                   | `rust.yml` `hakari` job                                       | Workspace-hack must stay current with eddacraft-tui's dep aggregation; TUIR-003 ran `cargo hakari generate` after the path-crate switch and verify has been clean since.                                                                                                                                                                                                                                                                                  |
| Acknowledgements freshness                              | `rust.yml` `acknowledgements-diff` job (CICD-007 conditional) | Cargo.lock + `ACKNOWLEDGEMENTS.md` are one atomic change (dev-workflow rule 13). Runs only when manifests, lockfile, or toolchain changed (CICD-007 skip condition) — pure source-only changes skip this gate.                                                                                                                                                                                                                                            |
| `cargo doc --no-deps -p eddacraft-tui`                  | (TODO — not yet a dedicated CI step)                          | Docs.rs build proxy. Currently exercised only via local runs during work-item validation. Adding a dedicated step is a small follow-up not blocking TUIR-006.                                                                                                                                                                                                                                                                                             |
| Package-list drift vs. baseline                         | (TODO — not yet a dedicated CI step)                          | `cargo package --list -p eddacraft-tui` diffed against `plans/specs/2026-05-22-tui-reintegration-baseline/package-list.txt` catches "we shipped an Anvil-only file" and "we stopped shipping a README" in one check. Currently exercised manually per TUIR-002 validation; promoting to CI is a follow-up.                                                                                                                                                |
| Snapshot tests (`INSTA_UPDATE=no`)                      | Implicit in `rust.yml` `test` job                             | Snapshot drift fails the build. Updates land via `cargo insta review` in PR.                                                                                                                                                                                                                                                                                                                                                                              |

**On enforcement scope:** every gate above ALSO runs on PRs that don't touch
`crates/eddacraft-tui/**` — the workspace jobs in `rust.yml` have no path
filter. That's intentional: the same gate must catch a sibling-crate change that
breaks eddacraft-tui's build. The "PRs touching `crates/eddacraft-tui/**`"
framing in TUIR-006's outcome describes the _minimum_ enforcement surface, not a
filter.

### Mirror side (`eddacraft/eddacraft-tui`)

The mirror's CI is intentionally minimal — a smoke gate against the mirrored
tree, not a re-run of Anvil's matrix. It exists to catch two failure modes the
Anvil side cannot:

1. The mirror push left the tree in a state that does not build from a fresh
   checkout (banner-swap step regression, missing file, mis-applied subtree
   split).
2. The crate's published tarball no longer packages cleanly from the mirrored
   layout.

| Gate                                     | Why                                                                                                                                                                                                                                                                                                                       |
| ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `cargo test` (no flags)                  | Smoke check against the mirrored tree as a fresh-checkout consumer would see it.                                                                                                                                                                                                                                          |
| `cargo publish --dry-run --all-features` | Tarball gate — proves the mirror's contents would publish to crates.io without error. Distinct from Anvil's tarball gate (which runs against the canonical source).                                                                                                                                                       |
| Mirror drift check (D-TUIR-018)          | Scheduled `mirror-drift-check.yml` on the **Anvil** side compares the canonical subtree (post-banner-swap) against the mirror's `main`. Any non-empty diff fails the job and opens / refreshes a tracked issue. Catches force-push collisions (D-TUIR-009), missed `paths:` triggers, and silent banner-swap regressions. |

**Mirror does not run:** `cargo fmt --check`, workspace clippy with
`-D warnings`, `cargo deny check`, hakari verify, ack freshness — all of those
are Anvil-authoritative and would be theatre on the mirror.

The mirror-side workflow file ships with the canonical source at
`crates/eddacraft-tui/.github/workflows/` (so the mirror sync carries it out
flat-rooted, per D-TUIR-004) once TUIR-004 lands the mirror workflow itself.

## Backport and Conflict Policy

**To be filled by TUIR-007**
([`plans/modules/tui-reintegration.aps.md` TUIR-007](../../plans/modules/tui-reintegration.aps.md#tuir-007-document-mirror-policy-and-update-public-surfaces)).

Will cover:

- Drive-by source PR against the mirror — auto-close template, redirect to
  canonical source path + contribution guide.
- Mirror force-push collision (someone pushed to the mirror directly) — next
  scheduled mirror run wins; lost work is the pusher's responsibility.
- Emergency security fix routing — canonical first, mirror propagates, crates.io
  publish follows; no "fix on mirror first" path is sanctioned.
- Tag conflict on the mirror — mirror automation refuses to overwrite existing
  tags; operator resolves manually.

## Where Decisions Live

- [ADR-047](../../plans/decisions/047-eddacraft-tui-canonical-source-mirror.md)
  — canonical-source + public-mirror model.
- [ADR-050](../../plans/decisions/050-eddacraft-tui-runner-and-cli-policy.md) —
  `runner` feature flag + CLI/parser policy (post-TUIR, TUIN-scoped).
- [TUIR module](../../plans/modules/tui-reintegration.aps.md) — the migration
  plan, including D-TUIR-007 (CI gate split, ratified here), D-TUIR-009
  (backport / mirror conflict policy, expanded in the section above when
  TUIR-007 lands), D-TUIR-012 (banner mechanism), D-TUIR-018 (mirror drift
  verification).
