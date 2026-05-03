# Council Review Issues — branch `claude/research-rust-tui-features-fvwJJ`

Generated 2026-05-03 from a 5-reviewer council pass (pragmatic, general, security, ops, adversarial) over the branch diff against `main`. Gate verdict: **WARN** (BLOCK on strict threshold). Total issues: 35 (16 must-fix, 13 should-fix, 6 consider).

Each issue is shaped so it can be pasted into `gh issue create` directly. Suggested label policy:

- `must-fix` → `priority:high` + `release-blocker`
- `should-fix` → `priority:medium`
- `consider` → `priority:low` + `tech-debt`
- Add a component label per area (`area:overlay`, `area:datatable`, `area:tree`, `area:toast`, `area:modal`, `area:wrappers`, `area:helpbar`, `area:theme`, `area:ci`, `area:supply-chain`, `area:docs`).

---

## At-a-glance index

| ID | Pri | Title | File |
|---|---|---|---|
| MF-01 | must | Toast height uses `chars().count()` instead of unicode-width | `src/widgets/toast.rs:100` |
| MF-02 | must | ToastStack height-sum overflows u16 | `src/widgets/toast.rs:222` |
| MF-03 | must | OverlayStack scrim dims previously-rendered layers | `src/widgets/overlay.rs:145` |
| MF-04 | must | DataTableState exposes ratatui TableState as public field | `src/widgets/data_table.rs:65` |
| MF-05 | must | TreeState exposes `expanded` / `cursor` as public fields | `src/widgets/tree.rs:69` |
| MF-06 | must | DataTableState::new selects row 0 even on empty table | `src/widgets/data_table.rs:77` |
| MF-07 | must | Deduplicate `dim_area` and `apply_scrim` (with Rect clamp) | `src/widgets/wrappers.rs:118`, `src/widgets/overlay.rs:159` |
| MF-08 | must | Deduplicate `severity_style` between modal and toast | `src/widgets/modal.rs:90`, `src/widgets/toast.rs:72` |
| MF-09 | must | OverlayStack should `impl Widget`/`StatefulWidget` | `src/widgets/overlay.rs:144` |
| MF-10 | must | CI publish dry-run skips feature-gated code | `.github/workflows/ci.yml:31` |
| MF-11 | must | Add `#[non_exhaustive]` to new public enums | `keyboard/handler.rs:4`, `widgets/toast.rs:32`, `widgets/overlay.rs:37`, `widgets/data_table.rs:35` |
| MF-12 | must | Add `[package.metadata.docs.rs]` and `doc(cfg(...))` for feature-gated widgets | `Cargo.toml`, `src/widgets/big_banner.rs`, `src/widgets/image_pane.rs` |
| MF-13 | must | Declare `rust-version` matching edition 2024 MSRV | `Cargo.toml` |
| MF-14 | must | Tree::visible_nodes is unbounded recursion → stack-overflow DoS | `src/widgets/tree.rs:126` |
| MF-15 | must | Decide policy on `rattles` (replace or pin+audit) | `Cargo.toml:15` |
| MF-16 | must | Decide policy on `animate*` (drop prelude re-export, pin, or replace) | `Cargo.toml:17`, `src/lib.rs:42` |
| SF-01 | should | Pin caret-versioned 0.x deps and add `cargo audit` + `cargo deny` to CI | `Cargo.toml`, `.github/workflows/ci.yml` |
| SF-02 | should | Document `image` feature transitive cost | `Cargo.toml`, `docs/` |
| SF-03 | should | Replace fragile `grep`/`sed` in release.yml with `cargo pkgid` | `.github/workflows/release.yml:28` |
| SF-04 | should | Tree mutators key on duplicate IDs — document or prevent | `src/widgets/tree.rs:82` |
| SF-05 | should | DataTable silently ignores bad SortIndicator and short widths | `src/widgets/data_table.rs:162`, `:196` |
| SF-06 | should | Tree::visible_nodes runs three times per frame | `src/widgets/tree.rs:148` |
| SF-07 | should | ToastStack carries unused `theme` field | `src/widgets/toast.rs:230` |
| SF-08 | should | HelpBar::separator should take `&'a str`, not `&'static str` | `src/widgets/help_bar.rs:48` |
| SF-09 | should | Stale doc count "12 reusable Ratatui widgets" | `src/lib.rs:24` |
| SF-10 | should | BigBanner builds widget per render and may panic on multi-char graphemes | `src/widgets/big_banner.rs:91` |
| SF-11 | should | ImagePane doc example demonstrates `.unwrap()` | `src/widgets/image_pane.rs:18` |
| SF-12 | should | Modal::title / Modal::footer use `.into()` where `Some(...)` is clearer | `src/widgets/modal.rs:65` |
| SF-13 | should | Add adversarial edge-case tests across widgets | `src/widgets/` |
| C-01 | consider | Land this work as 5 PRs next time | branch-level |
| C-02 | consider | Public `Role` enum is unused by any widget | `src/theme/traits.rs:7` |
| C-03 | consider | DataTable rows shape forces caller `String` allocation per frame | `src/widgets/data_table.rs:122` |
| C-04 | consider | ImagePane.title silently discarded when `bordered(false)` | `src/widgets/image_pane.rs:80` |
| C-05 | consider | Tree::render uses plain `+` for u16 y arithmetic | `src/widgets/tree.rs:193` |
| C-06 | consider | Theme trait does not document the fg/bg contract | `src/theme/traits.rs` |

---

# must-fix (16)

## MF-01 — Toast height uses `chars().count()` instead of unicode-width

**Severity:** major · **Area:** widgets/toast · **Reviewers:** pragmatic, general, adversarial

**Problem.** `Toast::measured_height` computes wrap height from `self.message.chars().count()`. Ratatui lays out text by display columns (unicode-width). For CJK, emoji, or combining-mark content, scalar count under-reports columns; the toast renders at insufficient height and the bottom rows are silently clipped.

**File:** `src/widgets/toast.rs:100`

**Fix.** Replace `self.message.chars().count()` with `unicode_width::UnicodeWidthStr::width(self.message)`. The crate is already a direct dependency (used in `shell.rs` and `parallel_progress.rs`).

**Acceptance.**

- Add a unit test that constructs a Toast with a multi-column CJK message and asserts `measured_height(width=10)` matches the rendered height.
- Existing toast tests still pass.

---

## MF-02 — ToastStack height-sum overflows u16

**Severity:** major · **Area:** widgets/toast · **Reviewers:** adversarial, security

**Problem.** `ToastStack::render` totals heights into a `u16` at line 222. With minimum toast height of 3 and an unbounded `push`, totalling beyond ~21,800 toasts overflows: debug builds panic, release builds wrap silently and anchor the stack at the wrong Y.

**File:** `src/widgets/toast.rs:222`

**Fix.**

1. Accumulate in `u32` and saturate to `u16::MAX` before the subtraction used for bottom-anchored placement.
2. Add a `pub fn max(self, n: usize) -> Self` builder on `ToastStack` that drops oldest beyond `n`. Also addresses the unbounded-queue concern from security.

**Acceptance.**

- Unit test pushes 30,000 minimal toasts and asserts no panic in debug mode.
- Unit test verifies `max(5)` keeps the latest five.

---

## MF-03 — OverlayStack scrim dims previously-rendered layers

**Severity:** major · **Area:** widgets/overlay · **Reviewers:** adversarial

**Problem.** `OverlayStack::render` iterates layers in order. For each layer with `scrim(true)` it calls `apply_scrim(frame, area)` over the entire parent area — which already contains pixels written by prior layers in the same call. Stacking two scrim'd dialogs darkens the first dialog rather than only the underlying app.

**File:** `src/widgets/overlay.rs:145-155`

**Fix.** Either (a) apply scrim once before the loop if any layer requests it, then render layers without per-layer scrim, or (b) apply scrim only to the area outside the union of prior layer rects. Option (a) is simpler.

**Acceptance.** Test that renders two scrim'd layers and asserts the first layer's cells do not have `Modifier::DIM` set.

---

## MF-04 — DataTableState exposes ratatui TableState as public field

**Severity:** major · **Area:** widgets/data_table · **Reviewers:** pragmatic, general, ops

**Problem.** `pub table_state: TableState` at `src/widgets/data_table.rs:65` leaks ratatui's internal cursor representation directly to consumers. Callers can call `state.table_state.select(Some(999))` and bypass the wrap/clamp logic in `move_up` / `move_down`. The pub field also locks the field name and type forever — any future ratatui breaking change to `TableState` becomes a breaking change to eddacraft-tui.

**File:** `src/widgets/data_table.rs:65`

**Fix.**

1. Change to `pub(crate) table_state: TableState`.
2. Add `pub fn selected(&self) -> Option<usize>` and `pub fn select(&mut self, index: Option<usize>)` accessors.
3. Add `pub fn scroll_offset(&self) -> usize` if downstream consumers need ratatui-level scroll.

**Acceptance.**

- Public API search (`grep -rn 'table_state\.' --include='*.rs'`) shows no external access.
- Existing tests pass; cursor-clamp invariants now enforced at the API boundary.

---

## MF-05 — TreeState exposes `expanded` / `cursor` as public fields

**Severity:** major · **Area:** widgets/tree · **Reviewers:** pragmatic, general, ops

**Problem.** `TreeState.expanded: HashSet<String>` and `TreeState.cursor: usize` are both public at `src/widgets/tree.rs:69-70`. Setting `cursor = items.len()` bypasses render-time clamping; mutating `expanded` directly bypasses any future invariant such as "parent expanded before child".

**File:** `src/widgets/tree.rs:69-70`

**Fix.**

1. Change both fields to `pub(crate)`.
2. Add a `pub fn cursor(&self) -> usize` accessor.
3. Add a `pub fn from_expanded(ids: impl IntoIterator<Item = String>) -> Self` constructor for callers persisting state.
4. All mutation goes through existing `move_up` / `move_down` / `toggle` / `expand` / `collapse`.

**Acceptance.**

- Public API search shows no external access to either field.
- New constructor test verifies persistence round-trip.

---

## MF-06 — DataTableState::new selects row 0 even on empty table

**Severity:** major · **Area:** widgets/data_table · **Reviewers:** adversarial

**Problem.** `DataTableState::new()` at `src/widgets/data_table.rs:77-80` unconditionally calls `table_state.select(Some(0))`. Callers cannot distinguish "empty table, nothing selected" from "row 0 selected". Construction case is untested.

**File:** `src/widgets/data_table.rs:77`

**Fix.** Default the initial selection to `None`. Add a separate `pub fn with_selection(index: usize) -> Self` if the row-0-default is ever wanted explicitly. Update `move_down` / `move_up` to handle `None` by selecting row 0 on first move when rows exist.

**Acceptance.**

- Test: `DataTableState::new()` returns state where `selected() == None`.
- Test: `move_down(5)` from `None` selects row 0 (or wraps appropriately).
- Existing tests adjusted as needed.

---

## MF-07 — Deduplicate `dim_area` and `apply_scrim` (with Rect clamp)

**Severity:** major · **Area:** widgets · **Reviewers:** pragmatic, general, security

**Problem.** Two near-identical functions exist:

- `fn dim_area(area: Rect, buf: &mut Buffer)` at `src/widgets/wrappers.rs:118`
- `fn apply_scrim(frame: &mut Frame, area: Rect)` at `src/widgets/overlay.rs:159`

Both iterate every cell and insert `Modifier::DIM`, with a `buf_area.contains(...)` per-cell guard. Future changes must be applied in two places. The per-cell guard is also wasteful — the area should be intersected with the buffer area once.

**Fix.**

1. Move a single `pub(crate) fn dim_area(area: Rect, buf: &mut Buffer)` into `src/widgets/mod.rs`.
2. Compute the intersection with `buf.area` once at the top via `Rect::intersection`.
3. `apply_scrim` becomes `dim_area(area, frame.buffer_mut())`.
4. Delete the duplicate.

**Acceptance.** One implementation; both call sites updated; existing tests pass.

---

## MF-08 — Deduplicate `severity_style` between modal and toast

**Severity:** major · **Area:** widgets/theme · **Reviewers:** pragmatic, general

**Problem.** Identical six-arm match over `BadgeStatus` exists at `src/widgets/modal.rs:90-98` and `src/widgets/toast.rs:72-79`. When a `BadgeStatus` variant is added or the mapping changes, both must be updated in lockstep with no compile-time enforcement.

**Fix.** Add an inherent method `BadgeStatus::style<T: Theme>(&self, theme: &T) -> Style` (or a free `pub(crate) fn badge_status_style`) in `widgets/status_badge.rs`. Replace both call sites with the new helper.

**Acceptance.** Both files import the shared helper and contain no local copy of the mapping. All existing modal and toast tests pass.

---

## MF-09 — OverlayStack should `impl Widget` / `StatefulWidget`

**Severity:** major · **Area:** widgets/overlay · **Reviewers:** general

**Problem.** Every other widget in the crate is rendered via the standard `Widget`/`StatefulWidget` traits. `OverlayStack` exposes a bespoke `pub fn render(self, frame: &mut Frame, area: Rect)` at `src/widgets/overlay.rs:144`. This breaks composition with ratatui layout helpers and the new wrapper widgets (Hideable, Padded), and surprises consumers who reach for `frame.render_widget(stack, area)` first.

**Fix.** OverlayStack genuinely needs `&mut Frame` (it calls `render_widget` per layer). Recommended approach: keep the bespoke method but rename it to `render_to_frame`. Add a doc-prominent `Widget` impl that either supports the simple single-layer path against `&mut Buffer`, or panics with a clear message pointing the consumer to `render_to_frame`. Update module-level docs and the prelude entry to match.

**Acceptance.**

- Doc example uses the new method name and compiles.
- A test asserts that the `Widget` impl path either works or fails loudly with a helpful message.

---

## MF-10 — CI publish dry-run skips feature-gated code

**Severity:** critical (operations) · **Area:** ci · **Reviewers:** ops

**Problem.** `.github/workflows/ci.yml:31` runs `cargo publish --dry-run` without `--features`. The `image` and `big-text` feature-gated widgets are not exercised by the only CI gate that simulates a publish.

**File:** `.github/workflows/ci.yml:31`

**Fix.** Change to `cargo publish --dry-run --all-features`. Also add a separate matrix job that runs `cargo check --no-default-features` to catch feature-leak regressions.

**Acceptance.** CI shows three check passes: default, all-features, no-default-features.

---

## MF-11 — Add `#[non_exhaustive]` to new public enums

**Severity:** major · **Area:** api-stability · **Reviewers:** ops

**Problem.** New public enums lack `#[non_exhaustive]`:

- `Action` — `src/keyboard/handler.rs:4` (15 variants)
- `ToastPlacement` — `src/widgets/toast.rs:32` (6 variants)
- `Placement` — `src/widgets/overlay.rs:37` (3 variants)
- `SortDirection` — `src/widgets/data_table.rs:35` (2 variants)

Pre-existing `CheckStatus` (`parallel_progress.rs:29`) and `LogLevel` (`log_panel.rs:19`) correctly use it. Without `#[non_exhaustive]`, adding a variant after v0.1 is a breaking change.

**Fix.** Add `#[non_exhaustive]` above each enum declaration. Update internal `match` arms with `_ => unreachable!()` only where the crate has full knowledge; otherwise leave them exhaustive.

**Acceptance.** `cargo build --all-features` and `cargo clippy -- -D warnings` both clean.

---

## MF-12 — Add `[package.metadata.docs.rs]` and `doc(cfg(...))` for feature-gated widgets

**Severity:** major · **Area:** docs · **Reviewers:** ops

**Problem.** No `[package.metadata.docs.rs]` section in `Cargo.toml`. docs.rs builds with default features only, so `BigBanner` and `ImagePane` are invisible on docs.rs. Also, no `#[cfg_attr(docsrs, doc(cfg(feature = "...")))]` on those types — even when the structs do appear, users cannot see which feature gates them.

**Fix.** Add a `[package.metadata.docs.rs]` table with `all-features = true` and `rustdoc-args = ["--cfg", "docsrs"]`. Annotate each feature-gated public type with `#[cfg_attr(docsrs, doc(cfg(feature = "...")))]`.

**Acceptance.** `cargo doc --all-features --open` shows `BigBanner` and `ImagePane` with feature badges.

---

## MF-13 — Declare `rust-version` matching edition 2024 MSRV

**Severity:** major · **Area:** packaging · **Reviewers:** ops

**Problem.** `Cargo.toml` has `edition = "2024"` but no `rust-version`. Edition 2024 requires Rust 1.85+; without an MSRV declaration, downstream users on older toolchains get cryptic errors.

**Fix.** Add `rust-version = "1.85"` to the `[package]` table. Add a CI job pinned to that exact toolchain (not `stable`) to catch MSRV drift.

**Acceptance.** `cargo +1.85.0 build` succeeds. CI matrix includes the pinned-toolchain row.

---

## MF-14 — Tree::visible_nodes is unbounded recursion → stack-overflow DoS

**Severity:** major · **Area:** widgets/tree · **Reviewers:** security

**Problem.** `visible_nodes` at `src/widgets/tree.rs:126-138` recurses unconditionally over user-provided `TreeNode` data with no depth limit. Three call sites, the worst inside `StatefulWidget::render` — meaning the crash happens during the draw loop. A 50,000-deep chain (filesystem path tree, parsed JSON, etc.) overflows the host process stack.

**Fix.** Convert to an iterative walker with an explicit `Vec` stack. On pop, push the current node into the output and, if expanded, push its children slice back onto the work stack with depth + 1. No recursion, no depth limit needed (memory-bounded by tree size, not stack).

**Acceptance.**

- Regression test that builds a 10,000-deep chain of nodes and asserts `visible_count` returns without panic.
- All existing tree tests pass.

---

## MF-15 — Decide policy on `rattles` (replace or pin+audit)

**Severity:** major · **Area:** supply-chain · **Reviewers:** security

**Problem.** `rattles 0.2` at `Cargo.toml:15` is a young, single-maintainer crate, used solely as the `rattle!` macro at `src/widgets/spinner.rs:11-18` to define a static frame list. The whole surface used here can be a `const FRAMES` array in roughly ten lines.

**Decision required.** Pick one:

- **(A) Replace with in-tree static.** Removes the dep entirely.
- **(B) Pin exactly to a specific patch version** and add to a vendored audit list.

Recommendation: A. The macro provides no leverage worth a dep.

**Acceptance.**

- If A: `rattles` removed from `Cargo.toml` and `Cargo.lock`. Spinner tests pass.
- If B: `Cargo.toml` shows the exact pin. `SECURITY.md` notes the trust tier.

---

## MF-16 — Decide policy on `animate*` (drop prelude re-export, pin, or replace)

**Severity:** major · **Area:** supply-chain · **Reviewers:** security

**Problem.** `animate 0.3.0` (with `animate-core`, `animate-macros`) at `Cargo.toml:17` is low-public-footprint. The proc-macro crate is build-time RCE surface — a malicious patch executes on every `cargo build`. Worse, `src/lib.rs:42` re-exports `animate::tick` and `animate::is_animating` from the prelude, locking eddacraft-tui's downstream API to whatever `animate` exposes.

**Decision required.** Pick one:

- **(A) Replace.** The actual usage is one easing function, one `Lerp` impl, and a `tick`/`is_animating` global clock. Roughly 100 lines in-tree. Removes proc-macro attack surface and global mutable state.
- **(B) Pin exactly + drop prelude re-export.** Wrap `tick`/`is_animating` in eddacraft-tui-owned shims so the dep is swappable.
- **(C) Pin exactly + keep prelude re-export.** Locks downstream API. Not recommended pre-publish.

Recommendation: B if A is too much work this sprint.

**Acceptance.**

- `Cargo.toml` shows the chosen disposition.
- `src/lib.rs` no longer re-exports `animate::*` from the prelude (or the chosen alternative is in place).
- `SECURITY.md` updated.

---

# should-fix (13)

## SF-01 — Pin caret-versioned 0.x deps and add `cargo audit` + `cargo deny` to CI

**Severity:** minor · **Area:** ci/supply-chain · **Reviewers:** security

**Problem.** All new deps use caret ranges on 0.x versions (`Cargo.toml:14-19`). `^0.2` permits any 0.2.x patch automatically — including any author-published malicious patch. CI has no advisory scanning.

**Fix.**

1. For low-trust deps (`rattles`, `animate*`), pin exact (see MF-15 / MF-16).
2. Add a CI step that installs and runs `cargo-audit`.
3. Add a CI step that installs and runs `cargo-deny check` against a `deny.toml` with at least `[advisories]` and `[bans]` sections.
4. Update `SECURITY.md:47` to differentiate trust tiers between `ratatui`/`crossterm` (mature) and `rattles`/`animate` (young).

---

## SF-02 — Document `image` feature transitive cost

**Severity:** minor · **Area:** docs · **Reviewers:** ops

**Problem.** Enabling the `image` feature pulls 76 transitive crates including duplicate `rustix` (0.38 vs 1.x), the full `windows` family, `rayon`, `icy_sixel`, and `base64-simd`. Consumers should know this before opting in.

**Fix.** Add a section to the `Cargo.toml` features doc-comment and a row in `docs/README.md` describing the transitive cost: build-time, binary size, the duplicate `rustix`, and the Windows crate family. Recommend evaluating whether full image decoding is needed before opting in.

---

## SF-03 — Replace fragile `grep`/`sed` in release.yml with `cargo pkgid`

**Severity:** minor · **Area:** ci · **Reviewers:** ops

**Problem.** `.github/workflows/release.yml:28-30` parses `cargo metadata` JSON with a `grep`/`head`/`sed` pipeline. The first `"version":` key in the JSON is not guaranteed to be the package's. If a dep injects a top-level `version` key, the wrong version is extracted and the tag/manifest gate passes incorrectly.

**Fix.** Replace with `cargo pkgid` and a small post-process that extracts the version after the `#`. Stable and explicit.

---

## SF-04 — Tree mutators key on duplicate IDs — document or prevent

**Severity:** minor · **Area:** widgets/tree · **Reviewers:** adversarial

**Problem.** `TreeState.expanded` is a `HashSet<String>` keyed by node `id`. If two `TreeNode`s share an id, `toggle` / `expand` / `collapse` act on both simultaneously. No documentation, no debug-assert, no test.

**File:** `src/widgets/tree.rs:82`

**Fix.** Either (a) document "ids must be unique" as an invariant on `TreeNode` and add a `debug_assert!` traversal at construction time, or (b) key the expanded set on a path (`Vec<usize>`) instead of id string. Option (a) is much cheaper.

**Acceptance.** Test that exercises duplicate ids and verifies the documented behaviour.

---

## SF-05 — DataTable silently ignores bad SortIndicator and short widths

**Severity:** minor · **Area:** widgets/data_table · **Reviewers:** adversarial

**Problem.**

- `data_table.rs:162-170`: if `SortIndicator.column >= headers.len()`, no column matches and no glyph renders. No log, error, or panic.
- `data_table.rs:196-205`: if `widths.len() < headers.len()`, ratatui silently allocates zero width to the uncovered columns — they become invisible.

**Fix.** Add `debug_assert!` checks in the relevant builder methods (`sort`, `widths`) so misuse is caught loudly in development. Add a doc-comment note that release builds will silently misbehave on invalid input — or upgrade the builders to return `Result` if total safety is preferred.

---

## SF-06 — Tree::visible_nodes runs three times per frame

**Severity:** minor · **Area:** widgets/tree · **Reviewers:** pragmatic, ops

**Problem.** `visible_count`, `selected_id`, and `render` each independently allocate a `Vec<Visible>` and traverse the tree. For thousands of nodes this is wasteful.

**File:** `src/widgets/tree.rs:148-173`

**Fix.** Compute once in `render`, cache on `TreeState` (or return from a private helper that callers share). At minimum, document the cost on `visible_count` / `selected_id`.

---

## SF-07 — ToastStack carries unused `theme` field

**Severity:** minor · **Area:** widgets/toast · **Reviewers:** pragmatic, security, ops

**Problem.** `ToastStack` has a `theme: &'a T` field but its render method ends with `let _ = self.theme;` and a comment "kept for future styling". Carries a lifetime constraint for no benefit.

**File:** `src/widgets/toast.rs:230`

**Fix.** Either (a) use the theme to style the gap rows (fill with base background), making the field live, or (b) drop the field; if needed later, re-add when used. Option (a) is preferable — consistent stack background looks more polished.

---

## SF-08 — HelpBar::separator should take `&'a str`, not `&'static str`

**Severity:** minor · **Area:** widgets/helpbar · **Reviewers:** general

**Problem.** `HelpBar::separator` takes `&'static str` at `src/widgets/help_bar.rs:48`. The struct already carries a `'a` lifetime through `bindings: &'a [Binding]` and `theme: &'a T`, so the static restriction is gratuitous. Callers cannot pass a runtime-constructed or borrowed separator.

**Fix.** Change the field and builder param to `&'a str`. Existing tests pass literals — no test changes needed.

---

## SF-09 — Stale doc count "12 reusable Ratatui widgets"

**Severity:** minor · **Area:** docs · **Reviewers:** general

**Problem.** `src/lib.rs:24` claims "12 reusable Ratatui widgets". This branch adds eight or more (DataTable, Tree, Modal, Toast, ToastStack, OverlayStack, Layer, HelpBar, Hideable, Disableable, Padded, BigBanner, ImagePane).

**Fix.** Replace with a maintenance-free phrasing: "a growing suite of reusable Ratatui widgets" or remove the count.

---

## SF-10 — BigBanner builds widget per render and may panic on multi-char graphemes

**Severity:** minor · **Area:** widgets/big_banner · **Reviewers:** general

**Problem.** `BigBanner::render` at `src/widgets/big_banner.rs:91` calls `BigText::builder()...build()` inside the hot render path. The upstream `tui-big-text 0.8` has a known TODO for multi-char graphemes that may panic. The crate-wide `missing_panics_doc` lint is suppressed, so there is no compiler reminder.

**Fix.**

1. Add a doc note on `BigBanner` warning that non-ASCII / multi-codepoint graphemes can panic until upstream resolves the TODO.
2. Optional: cache the built widget on first render if performance shows up in profiling.

---

## SF-11 — ImagePane doc example demonstrates `.unwrap()`

**Severity:** minor · **Area:** docs · **Reviewers:** security, ops

**Problem.** `src/widgets/image_pane.rs:18-24` doc example uses three `.unwrap()` calls inside a `no_run` block. Examples become idiom for downstream users, and `.unwrap()` on attacker-controlled paths is a panic-on-bad-image bug.

**Fix.** Rewrite the example to return `Result<(), Box<dyn std::error::Error>>` and use `?`. Add one line: "Validate image dimensions and source before constructing a `Protocol` — the `image` crate exposes parser advisories; keep the dep current."

---

## SF-12 — Modal::title / Modal::footer use `.into()` where `Some(...)` is clearer

**Severity:** minor · **Area:** widgets/modal · **Reviewers:** general

**Problem.** `Modal::title(self, title: &'a str)` writes `self.title = title.into();` where the field is `Option<&'a str>`. Same pattern in `Modal::footer`, `Toast::icon`, `ImagePane::title`. The `Into<Option<T>>` blanket relies on a less-common impl, obscuring whether the API can take `None`.

**File:** `src/widgets/modal.rs:65`

**Fix.** Replace with explicit `Some(title)` / `Some(footer)` / `Some(icon)`. No behaviour change; reads cleaner.

---

## SF-13 — Add adversarial edge-case tests across widgets

**Severity:** minor · **Area:** widgets · **Reviewers:** adversarial

**Problem.** Adversarial reviewer named the following untested scenarios; each represents real public-API surface.

- `DataTable` with `headers=[]`, `rows=[]` (zero-column, zero-row)
- `DataTable::render` into a `Rect` with `width=0` or `height=0`
- `DataTable::widths()` with a slice of the wrong length (validates SF-05)
- `SortIndicator` with `column = usize::MAX` (validates SF-05)
- `Tree::selected_id` when `nodes` is empty
- `TreeState::move_up(0)` and `move_down(0)`
- `Toast` with an empty `message`
- `Toast::measured_height` at `width` 0, 1, 2, 3
- `ToastStack` rendered into a zero-width area
- `ToastStack` with bottom placement where the single toast is taller than the area height
- `HelpBar` rendered into an area with `height > 1`
- `Modal` rendered at exactly 2×2 (boundary of the `< 2 || < 2` guard)
- `OverlayStack` with `Layer::placement` size 0×0
- `OverlayStack` with two consecutive `scrim(true)` layers (validates MF-03)

**Fix.** Add one test per case in the relevant `#[cfg(test)] mod tests` block.

---

# consider (6)

## C-01 — Land this work as 5 PRs next time

**Severity:** nit · **Area:** process · **Reviewers:** pragmatic

Suggested split for future large-feature branches: (1) theme + palette, (2) wrappers, (3) overlay + modal + toast, (4) datatable + tree, (5) helpbar + feature-flagged. Dependency order is clean; each PR is reviewable in roughly an hour.

---

## C-02 — Public `Role` enum is unused by any widget

**Severity:** nit · **Area:** theme · **Reviewers:** pragmatic

`src/theme/traits.rs:7-18` defines `Role` and `Theme::role_style`, but no widget uses it — they all call `theme.title()`, `theme.status_ok()`, etc. directly. Either remove `Role` from the prelude until widgets adopt it, or add a doc comment marking it as a forward extensibility hook.

---

## C-03 — DataTable rows shape forces caller `String` allocation per frame

**Severity:** nit · **Area:** widgets/data_table · **Reviewers:** pragmatic, ops

`DataTable` rows accept `&'a [Vec<String>]` at `src/widgets/data_table.rs:122`. Downstream callers with typed models will rebuild a `Vec<Vec<String>>` every frame just to feed the widget. Consider `&'a [&'a [&'a str]]` or `IntoIterator<Item: AsRef<str>>` for hot paths. A pre-stable API change is much cheaper now than after consumers build against it.

---

## C-04 — ImagePane.title silently discarded when `bordered(false)`

**Severity:** nit · **Area:** widgets/image_pane · **Reviewers:** general

`src/widgets/image_pane.rs:80` only renders the title in the bordered branch. Setting a title on an unbordered pane silently does nothing. Either render title as a floating label even without a border, or `debug_assert!` against the combination, or document the constraint clearly.

---

## C-05 — Tree::render uses plain `+` for u16 y arithmetic

**Severity:** nit · **Area:** widgets/tree · **Reviewers:** adversarial

`src/widgets/tree.rs:193` uses plain `+` rather than `saturating_add`. Safe under `Rect::new` invariants, but `Rect` fields are `pub` and constructable directly via struct literal, bypassing the clamp. Every other coordinate arithmetic in this PR uses saturating ops; this line should match.

---

## C-06 — Theme trait does not document the fg/bg contract

**Severity:** nit · **Area:** theme · **Reviewers:** adversarial

Tests across new widgets call `.unwrap()` on `theme.status_error().fg` etc. This is safe for `EddaCraftTheme`, but a downstream `impl Theme` returning `Style::default()` (no fg/bg) would panic in tests. Either add a doc requirement on the trait that implementors must set fg/bg, or relax the tests to handle absent values.

---

## How to convert this into GitHub issues

This document is the source of truth. Create issues only for items the team commits to in the current iteration; defer the rest as backlog. For bulk creation, write a small shell loop that calls `gh issue create --title ... --body ... --label ...` per row in a TSV exported from the index table.
