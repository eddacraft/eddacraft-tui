# APS dashboard starter kit

> **STATUS: PROPOSED — DO NOT PUBLISH.** This kit copies source that is
> currently `LicenseRef-Proprietary` (`anvil-cli` / `anvil-tui`, per ADR-018).
> Publishing it to any public repository is **blocked on
> [ADR-055](../../../plans/decisions/055-aps-oss-carveout.md)** being Accepted
> with legal sign-off, plus the ADR-055 pre-publication scrub (Apache-2.0
> headers, removal of Anvil-internal references, neutral branding). A 2026-05-27
> Council review blocked the lift on this. Until then this is internal staging
> only. Tracked as APSDASH-003.

A read-only terminal dashboard for canonical
[APS](https://github.com/eddacraft/anvil-plan-spec) plan state: it reads
`plans/index.aps.md` + `plans/modules/*.aps.md`, derives per-module progress and
consistency warnings, and renders them as an interactive Ratatui surface (with
`--json` and `--no-tui` fallbacks).

This is the **seed copy** of Anvil's internal `anvil plan dashboard`. It is
meant to be **copied once and then re-developed** in the adopting repo into a
standalone, shippable component — not kept in sync with Anvil. It is a one-way
fork on purpose (see [Divergence](#divergence-this-is-a-one-way-fork)).

## What ships in this kit

| File                             | State               | Purpose                                                                                  |
| -------------------------------- | ------------------- | ---------------------------------------------------------------------------------------- |
| `src/snapshot.rs`                | compiles as-is      | Reads `plans/` and builds a `PlanStatusSnapshot`. `std` + `serde` + `anyhow` only.       |
| `src/dashboard/mod.rs`           | one edit applied    | The `Surface` state machine + the TUI snapshot types. Render-layer entry.                |
| `src/dashboard/render.rs`        | as-is               | The Ratatui layout (header, module table, work-item list, warnings, detail/help panels). |
| `src/dashboard/event_adapter.rs` | as-is               | Maps `eddacraft-tui` keyboard actions to dashboard state transitions.                    |
| `Cargo.toml.template`            | rename + edit       | Standalone crate manifest pinning the public `eddacraft-tui`.                            |
| `src/run_surface.rs.template`    | rename + adopt      | Minimal standalone TUI run loop (Anvil's is bigger; this is the reduced core).           |
| `src/main.rs.template`           | rename + re-develop | CLI entry: argv parser, JSON/plain/TUI dispatch, and the snapshot→TUI adapter.           |

`.template` files are inert until you rename them (drop the `.template` suffix).
The `.rs` files are faithful copies of the Anvil source so `git blame` and diffs
against upstream stay legible.

## Why this is a clean lift

Everything the render layer needs already lives in the public, source-visible
[`eddacraft-tui`](https://github.com/eddacraft/eddacraft-tui) crate (published
to crates.io): the `Surface` trait, the `render_shell` chrome + `ShellBranding`,
`keyboard::{Action, KeyHandler}`, `theme::{EddaCraftTheme, Theme}`, and the
`Container` / `DataTable` / `StatusBadge` widgets.

In Anvil the dashboard imported `crate::surface::Surface`, but that was only a
re-export of `eddacraft_tui::surface::Surface`. So the **entire** anvil-local
coupling of the render layer was a single `use` line, and it is **already
repointed** in `src/dashboard/mod.rs`. No trait reimplementation is required.

## Adoption checklist

1. **Copy the kit** into the adopting repo. Give it its own subtree prefix so it
   tracks independently of any other starter kit you adopt:

   ```bash
   git subtree add --prefix vendor/aps-dashboard \
     https://github.com/eddacraft/anvil-plan-spec.git main --squash
   ```

   (Or just copy the directory — this kit has no generator, only source.)

2. **Rename the templates:**

   ```bash
   mv Cargo.toml.template          Cargo.toml
   mv src/run_surface.rs.template  src/run_surface.rs
   mv src/main.rs.template         src/main.rs
   ```

3. **Pin `eddacraft-tui`.** Keep `ratatui` / `crossterm` in lockstep with the
   `eddacraft-tui` release you pin (this kit was cut against `eddacraft-tui`
   0.2.x → ratatui 0.30, crossterm 0.29). A mismatched ratatui is the one thing
   that will make the `Surface` / `render_shell` types fail to line up.

4. **Build and run:**

   ```bash
   cargo run -- --root /path/to/a/repo/with/plans   # interactive TUI
   cargo run -- --json --root .                      # machine-readable snapshot
   cargo test                                        # the copied unit tests run as-is
   ```

All 30 copied unit tests pass in the new crate without changes — the dashboard
state-machine and render tests exercise only public `eddacraft-tui` types, and
the `snapshot.rs` builder tests use `tempfile` (the kit's one dev-dependency,
already declared in `Cargo.toml.template`).

## The three seams

Only three things separate the copied source from a standalone binary, and two
are already done:

1. **`Surface` trait — done.** Repointed to `eddacraft_tui::surface::Surface` in
   `src/dashboard/mod.rs`. Nothing to reimplement.

2. **Run loop — provided, reduced.** `src/run_surface.rs.template` is the
   generic `Surface` loop extracted from Anvil's `crates/anvil-cli/src/tui.rs`.
   Anvil's version also carries a panic-restore hook + RAII `TerminalGuard` (so
   a panic mid-loop doesn't leave the terminal in raw mode). That hardening is
   **not** ported here — port `install_panic_hook` + `TerminalGuard` from the
   Anvil source if you want it (recommended for a shipped tool).

3. **Entry + adapter — provided, expect to re-develop.** `src/main.rs.template`
   has the argv parser, the JSON/plain/TUI dispatch, and `to_tui_snapshot`
   (which bridges the builder's `PlanStatusSnapshot` to the render layer's
   `PlanDashboardSnapshot`). This is the layer to reshape for your product's CLI
   conventions.

## Re-develop-to-ship roadmap

The kit builds and runs as-is, but to make it a polished, general component:

- **Collapse the two snapshot types.** There are two near-identical structs —
  `snapshot::PlanStatusSnapshot` (rich, serialised for `--json`) and
  `dashboard::PlanDashboardSnapshot` (flattened for rendering) — bridged by
  `to_tui_snapshot`. Anvil keeps both for historical reasons; a fresh component
  can render directly off one type and delete the adapter.
- **Decouple from Anvil's status dialect.** `snapshot.rs` treats `Merged` and
  `Released/Shipped` as done-states. Those are **Anvil release-lifecycle
  extensions**, not canonical APS (canonical terminal states are `Done` /
  `Complete`). For a general component, make the done-state set the canonical
  pair and expose the extras as configuration.
- **Branding.** `run_surface.rs` defaults to `ShellBranding::Plain`; set your
  own brand/wordmark.
- **Snapshot tests.** Anvil covers the render surface with `insta` snapshot
  tests; the kit ships only the state-machine unit tests. Add render snapshots
  before you depend on the layout staying stable.
- **Enrichment seam.** `PlanStatusSnapshot` carries an empty `enrichments` field
  — Anvil's reserved hook for future GitHub/CI annotations. Wire or remove it as
  suits.

## Divergence: this is a one-way fork

Once copied, this component evolves independently of Anvil. Anvil will not pull
your changes back, and you should not expect Anvil's fixes to flow to you
automatically. That is the intended model — it lets the public component target
**canonical** APS and stay clean, rather than carrying Anvil's internal dialect.
The trade-off is eyes-open: a parser fix made in Anvil's copy will not propagate
here.

The shared dependency, `eddacraft-tui`, is the exception — it is a genuinely
published crate, so depend on it from crates.io and upgrade it normally.

## Provenance

Seeded from Anvil (`eddacraft/anvil-001`):

- `src/snapshot.rs` ← `crates/anvil-cli/src/plan_dashboard.rs`
- `src/dashboard/*` ← `crates/anvil-tui/src/surfaces/plan_dashboard/*`
- `src/run_surface.rs.template` ← reduced from `crates/anvil-cli/src/tui.rs`
- `src/main.rs.template` ← `crates/anvil-cli/src/commands/plan.rs`

The original dashboard was delivered by APSCAN-011; this extraction is tracked
under the APSDASH module in Anvil's `plans/index.aps.md`.
