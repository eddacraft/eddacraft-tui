# TUIR-001: eddacraft-tui pre-migration baseline

**Captured:** 2026-05-22
**TUIR module:** [`tui-reintegration`](../modules/tui-reintegration.aps.md)
**Work item:** TUIR-001 — Lock the import baseline
**Companion artefact:** [`2026-05-22-tui-reintegration-baseline/package-list.txt`](./2026-05-22-tui-reintegration-baseline/package-list.txt)

This document records the exact public state of `eddacraft/eddacraft-tui` that
will be imported into `crates/eddacraft-tui/` per ADR-047. It is the durable
record TUIR-008 validation diffs against, and the inputs the TUIR module's
Ready Checklist references.

## Source

| Field             | Value                                                                                 |
| ----------------- | ------------------------------------------------------------------------------------- |
| Repo              | `https://github.com/eddacraft/eddacraft-tui`                                          |
| Visibility        | Public                                                                                |
| Default branch    | `main`                                                                                |
| `main` HEAD       | `36abac71aaff4fdde96d229bd695527a3c080bc7` ("chore: APS Init")                        |
| Branch protection | Enabled — 6 required checks (CodeQL, Check default/all-features/no-default-features, MSRV 1.88.0, Supply chain) |
| License           | Apache-2.0 (`LICENSE`, 11.2K, verbatim)                                               |
| `NOTICE`          | **Not present.** Migration does not create one (D-TUIR-013 + TUIR-002 corrections).   |

### Import point

Migration imports the v0.2.2 tree — the latest published release.

| Field          | Value                                                                |
| -------------- | -------------------------------------------------------------------- |
| Import tag     | `v0.2.2`                                                             |
| Tag SHA        | `5078007b961bcb648ac0d5a190af94d797e01daf` (annotated tag object)    |
| Commit SHA     | `7ca0c4201cb50397ac25797148da1c114003fb0b`                           |
| Commit date    | 2026-05-08T13:56:18+08:00                                            |
| Commit subject | `fix(tree): release builds fail on cfg-gated ids_are_unique`         |

The HEAD-of-main commit (`36abac71`, "chore: APS Init") is not imported — it is
post-release APS scaffolding for the standalone repo that is not part of the
crate.

## crates.io

| Field             | Value                                                       |
| ----------------- | ----------------------------------------------------------- |
| Crate             | `eddacraft-tui`                                             |
| Latest version    | `0.2.2` (max stable; also `max_version`)                    |
| Total downloads   | 22,900 at capture time                                      |
| Owner             | `joshuaboys` (user)                                         |
| `repository` link | `https://github.com/eddacraft/eddacraft-tui` (preserve per D-TUIR-013) |
| `homepage`        | null (not set in `Cargo.toml`)                              |
| `documentation`   | null (docs.rs is the default)                               |

**First post-migration publish:** `0.2.3` (pinned by D-TUIR-005; the next patch
in the existing series; no migration-driven semver bump).

## Tags

All five published tags use the **unprefixed `vX.Y.Z`** form. D-TUIR-011 leaves
them in place on the public mirror untouched; new tags from cutover onwards use
the prefixed `eddacraft-tui-v*` form per D-TUIR-002.

| Tag           | Commit date            | Tag-object SHA                              |
| ------------- | ---------------------- | ------------------------------------------- |
| `v0.1.0-rc.0` | 2026-04-10T14:03:05+08:00 | `d2361107948a153df0a470c22f679c60bb19e119` |
| `v0.1.0`      | 2026-04-10T18:58:41+08:00 | `2ccd5e32242150383e7d228cbe4672783e29a71c` |
| `v0.2.0`      | 2026-05-07T21:13:04+08:00 | `e949e47b3f891f803fccb9a0705973a43094b714` |
| `v0.2.1`      | 2026-05-08T13:14:45+08:00 | `20ce4802770a66e07b5f55a3e7cb7228dbe62953` |
| `v0.2.2`      | 2026-05-08T13:56:18+08:00 | `5078007b961bcb648ac0d5a190af94d797e01daf` |

Corresponding GitHub releases exist for `v0.1.0`, `v0.2.0`, `v0.2.1`, `v0.2.2`
(auto-generated notes per `release.yml`).

## Crate metadata (`Cargo.toml` at v0.2.2)

| Field          | Value                                                       |
| -------------- | ----------------------------------------------------------- |
| `name`         | `eddacraft-tui`                                             |
| `version`      | `0.2.2`                                                     |
| `edition`      | `2024`                                                      |
| `rust-version` | `1.88` (MSRV — preserve per D-TUIR-015)                     |
| `license`      | `Apache-2.0`                                                |
| `description`  | "Shared Ratatui component library for the eddacraft product family" |
| `keywords`     | `["tui", "ratatui", "widgets", "terminal"]`                 |
| `categories`   | `["command-line-interface"]`                                |
| `readme`       | `README.md`                                                 |
| `repository`   | `https://github.com/eddacraft/eddacraft-tui` (preserve)     |

### `[package.metadata.docs.rs]`

Preserved verbatim per D-TUIR-013 so docs.rs build configuration survives the
migration:

```toml
[package.metadata.docs.rs]
all-features = true
rustdoc-args = ["--cfg", "docsrs"]
```

## Feature flags

Three feature flags total (all opt-in; no default features).

| Flag         | Description                                                    | Transitive cost                                                                                                |
| ------------ | -------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `test-utils` | Enables snapshot harness helpers for consumer testing.         | None (empty feature set; just gates internal modules).                                                         |
| `image`      | Enables `ImagePane` widget via `ratatui-image` + `image` deps. | Substantial graph (`ratatui-image`, `image`, `rayon`, `windows` family, `icy_sixel`). Raises effective MSRV.   |
| `big-text`   | Enables `BigBanner` widget via `tui-big-text`.                 | Smaller than `image` but still raises effective MSRV.                                                          |

**MSRV note from the crate's own `[features]` block:** the `image` feature's
transitive `quantette` requires Rust 1.90, exceeding the core 1.88 floor. The
standalone CI verifies the 1.88 floor without these features and uses stable
Rust for the `Check (all-features)` row. The Anvil-side D-TUIR-007 per-feature
gates inherit the same posture.

## Dependencies

```toml
[dependencies]
ratatui      = "0.30"
crossterm    = "0.29"
unicode-width = "0.2"
textwrap     = { version = "0.16", features = ["smawk"] }
animate      = { version = "=0.3.0", features = ["ratatui"] }  # pinned: ships proc-macro code (build-time RCE surface)
ratatui-image = { version = "10", default-features = false, features = ["crossterm"], optional = true }
tui-big-text = { version = "0.8", optional = true }
image        = { version = "0.25", default-features = false, optional = true }

[dev-dependencies]
insta    = { version = "1", features = ["yaml"] }
tempfile = "3"
```

**Zero Anvil-internal crates.** D-TUIR-014's CI guard (`cargo tree -p
eddacraft-tui --prefix=none --no-default-features --edges normal | grep -E
'^(anvil-|eddacraft-anvil-|kindling)'`) returns no matches against the v0.2.2
tree.

## `[lints]` block

Preserved verbatim per D-TUIR-016 (the crate carries its own `[lints]`, NOT
`lints.workspace = true`). More permissive than Anvil's workspace lints
(`missing_errors_doc` and `missing_panics_doc` are allowed here; Anvil's
workspace denies them), which is precisely why D-TUIR-016 opts out — letting
the crate inherit Anvil's lints would change the published crate's build
behaviour.

```toml
[lints.rust]
unsafe_code = "forbid"

[lints.clippy]
all = { level = "deny", priority = -1 }
pedantic = { level = "warn", priority = -1 }
module_name_repetitions = "allow"
must_use_candidate = "allow"
missing_errors_doc = "allow"
missing_panics_doc = "allow"
```

## Public CI surface

The standalone repo ships two workflows. The Anvil-side D-TUIR-007 gate list is
deliberately a superset.

### `.github/workflows/ci.yml` (PR + push to main)

- `check` matrix over three profiles (`default`, `all-features`, `no-default-features`):
  - `cargo fmt --all -- --check` (default row only)
  - `cargo clippy --all-targets`
  - `cargo test`
  - `cargo check --release` (the gate D-TUIR-007 added — catches the
    [`ids_are_unique`-class regression](https://github.com/eddacraft/eddacraft-tui/issues/29)
    fixed in `v0.2.2` itself)
  - `cargo publish --dry-run --all-features` (all-features row only)
- `msrv` job: `cargo build` on Rust 1.88.0 (the declared MSRV).
- `audit` job: `cargo install --locked --version 0.22.1 cargo-audit`,
  `cargo audit`; `cargo install --locked --version 0.19.6 cargo-deny`,
  `cargo deny check`.
- CodeQL via repo default (required on `main`).

### `.github/workflows/release.yml` (tag-triggered)

- Trigger: `push` to tags matching `v[0-9]+.[0-9]+.[0-9]+` (unprefixed form).
- Verifies tag matches `Cargo.toml` version.
- `cargo publish --all-features` with `CARGO_REGISTRY_TOKEN`.
- `gh release create $TAG --title $TAG --generate-notes`.

**Token to revoke at TUIR-008:** `CARGO_REGISTRY_TOKEN` on the standalone repo,
once the first publish from `anvil-001` canonical source succeeds. The
fine-grained replacement (`CRATES_IO_EDDACRAFT_TUI_TOKEN` per D-TUIR-005)
lives in `anvil-001` repo secrets.

## Repo tree

Source under `src/` plus three families (`pretext/`, `theme/`, `widgets/`,
`keyboard/`) and one snapshot dir.

- **Widgets** (20 modules under `src/widgets/`): `big_banner`, `confirm`,
  `container`, `data_table`, `divider`, `editor`, `header`, `help_bar`,
  `image_pane`, `log_panel`, `modal`, `overlay`, `parallel_progress`,
  `pretext`, `progress_bar`, `select`, `spinner`, `status_badge`, `status_bar`,
  `text_input`, `toast`, `tree`, `wrappers`. (`mod.rs` excluded from the count.)
- **Snapshots:** `src/snapshots/eddacraft_tui__shell__tests__snapshot_shell_chrome.snap`
  (1 file). Confirms snapshot colocation under `src/` per D-TUIR-017
  (correction landed in PR #1838).
- **Crate root files:** `Cargo.toml`, `Cargo.lock`, `Cargo.toml.orig`,
  `CHANGELOG.md` (11.6K), `CONTRIBUTING.md`, `LICENSE` (11.2K), `README.md`,
  `RELEASE.md`, `SECURITY.md`, `deny.toml`, `.gitignore`, `.oxfmtrc.json`,
  `.cargo_vcs_info.json`, two workflow files.
- **Docs in tarball:** `docs/README.md`, `docs/animations.md`,
  `docs/council-review-issues.md`.

Full byte-exact published-tarball file list:
[`package-list.txt`](./2026-05-22-tui-reintegration-baseline/package-list.txt)
(60 files). Captured via `cargo package --list --allow-dirty`. This is the
baseline that D-TUIR-007's drift gate diffs against post-migration.

### Repo-management dirs NOT carried to `crates/eddacraft-tui/`

Per the TUIR-002 outcome correction (PR #1838), the standalone repo's
repo-management infrastructure does **not** migrate:

| Dir              | Contents                                                                | Anvil equivalent                       |
| ---------------- | ----------------------------------------------------------------------- | -------------------------------------- |
| `bin/`           | `aps`, `aps.ps1` (cross-platform APS CLI wrapper)                       | `bin/anvil` etc. at `anvil-001` root  |
| `lib/`           | `Lint.psm1`, `Output.psm1`, `Scaffold.psm1`, `lint.sh`, `orchestrate.sh`, `output.sh`, `scaffold.sh`, `rules/` | Anvil planning scaffolds in `tools/`  |
| `plans/`         | `aps-rules.md`, `index.aps.md`, `modules/`, `execution/`                | `anvil-001/plans/` (this repo's APS)   |
| `aps-planning/`  | (sibling planning content for the standalone repo)                      | folded into `anvil-001/plans/`         |
| `tools/`         | `starters/` — the relocated ATTRIB-011 acknowledgements starter         | `anvil-001/tools/starters/acknowledgements` is canonical |
| `docs/`          | `README.md`, `animations.md`, `council-review-issues.md` — three crate-doc files are already IN the published tarball (above); anything outside the tarball is repo-doc and stays behind | n/a — what ships is captured by `package-list.txt` |

The pre-cutover graph for these dirs remains reachable via D-TUIR-010's
`pre-canonical-archive` branch on the public mirror.

## Validation evidence

Captured by re-running cargo against the standalone v0.2.2 tree at SHA
`7ca0c4201cb50397ac25797148da1c114003fb0b`:

- `cargo test --all-features`: **269 unit tests, 11 doc-tests, 0 failed**.
- `cargo test --no-default-features`: **all unit tests passed, 9 doc-tests
  passed, 0 failed** (image / big-text doc-tests gated out as expected).
- `cargo publish --dry-run --all-features`: **packaged 60 files, 440.3 KiB
  (107.3 KiB compressed)**; verify step compiled the packaged crate against the
  full dependency graph (ratatui 0.30, crossterm 0.29, ratatui-image 10.0.8,
  animate 0.3.0, etc.) and succeeded; upload aborted under `--dry-run` as
  expected.
- `cargo package --list --allow-dirty` stored byte-for-byte under
  [`package-list.txt`](./2026-05-22-tui-reintegration-baseline/package-list.txt).

## Downstream consumers

The migration's TUIR-008 validation pass checks the candidate against each
consumer below.

### First-party (in `anvil-001` workspace)

| Crate                                          | Manifest line                                                                        | Notes                                          |
| ---------------------------------------------- | ------------------------------------------------------------------------------------ | ---------------------------------------------- |
| `eddacraft-anvil` (`crates/anvil-cli`)         | `eddacraft-tui = { workspace = true }`                                               | Currently resolves to crates.io 0.2.2.         |
| `eddacraft-anvil-tui` (`crates/anvil-tui`)     | `eddacraft-tui = { workspace = true }` + `{ workspace = true, features = ["test-utils"] }` for tests | Two distinct edges; both flip to path crate at TUIR-003. |
| `workspace-hack` (`crates/workspace-hack`)     | `eddacraft-tui = { version = "0.2", default-features = false, features = ["test-utils"] }` | Regenerate via `cargo hakari` after TUIR-002 (TUIR-003 step). |

Root workspace `Cargo.toml` declares the workspace-dep pin
`eddacraft-tui = "0.2.2"` — TUIR-003 rewrites this from a crates.io version pin
to a path dependency.

### External (separate repos, operator-driven validation)

| Repo                                          | Visibility | TUIR-008 check                                                  |
| --------------------------------------------- | ---------- | --------------------------------------------------------------- |
| [`eddacraft/eddacraft-skills`](https://github.com/eddacraft/eddacraft-skills) | Private (eddacraft org; separate from `anvil-001`) | Maintainer runs `cargo check` against the dry-run candidate via `[patch.crates-io]`; outcome recorded on the TUIR-008 PR description. |

No other documented external consumers at capture time (crates.io page shows
no public reverse dependencies as of 2026-05-22).

## Deltas to fold into `crates/eddacraft-tui/`

Migration is byte-equivalent at the API surface (D-TUIR-006 + Out-of-Scope
clause), so the deltas are envelope / metadata / tooling only:

1. **Tag prefix.** Post-cutover releases use `eddacraft-tui-vX.Y.Z`
   (D-TUIR-002). Old unprefixed tags preserved untouched on the public mirror
   (D-TUIR-011).
2. **`MIRROR-README.md`** added at the crate root with the mirror banner
   (D-TUIR-012). Mirror workflow prepends it onto `README.md` and removes the
   banner file at push time, matching the ATTRIB-011 pattern.
3. **`[lints]` block kept verbatim**, NOT `lints.workspace = true` (D-TUIR-016).
   A CI grep enforces this in the Anvil-side gates.
4. **No new dependencies**, no Anvil-internal crate deps (D-TUIR-014). The
   workspace `Cargo.toml`'s workspace-dep pin moves from a crates.io version to
   `path = "crates/eddacraft-tui"` (TUIR-003).
5. **Workspace membership.** Crate becomes a workspace member but is NOT added
   to `default-members` (TUIR-002 outcome).
6. **No NOTICE file generated.** Standalone has none; do not create one absent
   an attribution need (TUIR-002 outcome, corrected in PR #1838).
7. **Snapshot path:** `src/snapshots/` (insta-default colocation matches
   standalone layout — D-TUIR-017, corrected in PR #1838).
8. **Repo-management dirs** (`bin/`, `lib/`, `plans/`, `aps-planning/`,
   `tools/`, the non-tarball portions of `docs/`) stay behind; Anvil canonical
   paths cover them (TUIR-002 outcome).
9. **`Cargo.toml.orig`** is a `cargo publish`-generated artefact, not a source
   file — exclude from the imported tree.
10. **Workflow files** (`.github/workflows/ci.yml`, `.github/workflows/release.yml`)
    do NOT migrate. The Anvil-side gates per D-TUIR-007 supersede them; the
    mirror gets a smoke-only workflow per the public-mirror side of D-TUIR-007.
11. **`deny.toml`** — standalone repo's copy stays behind. Anvil's existing
    workspace `deny.toml` covers the migrated crate.

## Open items propagated to TUIR follow-ups

- **Mirror PAT** (`EDDACRAFT_TUI_MIRROR_PUSH_TOKEN`, fine-grained, scoped to
  `eddacraft/eddacraft-tui`, `Contents: Read and write`) — needs generating
  and storing in `anvil-001` repo secrets before TUIR-004 promotes to Ready.
- **crates.io token** (`CRATES_IO_EDDACRAFT_TUI_TOKEN`, least-privilege,
  scoped to the `eddacraft-tui` crate, publish-only) — needs generating by
  the crate owner (`joshuaboys`) and storing before TUIR-005 promotes.
- **`pre-canonical-archive` branch plan** (D-TUIR-010) — runbook for the
  one-shot history-preservation force-push needs drafting before TUIR-008.
- **`MIRROR-README.md` draft + `docs/policies/eddacraft-tui-mirror.md` draft**
  — both blocking TUIR-007 promotion.
- **Two-layer rollback path** (dependency rollback + full-migration rollback)
  — documented before TUIR-008 promotion.
