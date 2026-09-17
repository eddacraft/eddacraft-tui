<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Settings Safe Preferences

| ID      | Owner | Priority | Status   | Progress |
| ------- | ----- | -------- | -------- | -------- |
| SETPREF | —     | medium   | Done     | 6/6      |

**Last reviewed:** 2026-09-16 — SETPREF-001..006 Merged via #4741 (closed
#4735–#4740); module Done. Validation re-run on `main` at `e7024b7a6`: the
`settings_write_class_a`, `settings_scope`, `settings_reset`, `settings_persist`,
`settings_concurrency` and `settings_safe_write` tests pass (the unwritable-target
case needs a non-root runner, as in CI). Operator locked this module as the
`v0.12.0-beta` next-minor claim (not the active `v0.11.1-beta` patch). SETINS
Done via #4696. Class A v0.1 keys: `interface.compact`, `interface.timestamps`,
`interface.motion`, `interface.hints`. Canonical writer: user scope →
settings-service `user-config`. Safe-write policy:
[`2026-09-16-settings-class-a-safe-write.md`](../specs/2026-09-16-settings-class-a-safe-write.md).
Created 2026-08-06 from the operator-supplied `/settings` specification v1.1
([`plans/specs/2026-08-06-settings-truth-surface.md`](../specs/2026-08-06-settings-truth-surface.md),
spec §22 Slice 2). Completing this module closes `/settings` v0.1 (spec §24).

> **Entry gate.** SETPREF starts only after the inspect surface ships. It is the
> first module in the programme that writes configuration, and it introduces the
> settings service as the **only** write path for settings-surface mutations.
> Two write routes already exist outside it: `anvil config set` (rewrites rule
> modes in the project config) and the `anvil init` / `anvil start` bootstrap
> writes. They are known, not newly discovered defects: bootstrap writes stay
> out of this programme's scope, and `anvil config set` must route through (or
> be subsumed by) the settings service no later than SETGOV, since rule modes
> are Class C content. Any other write route discovered during this work is a
> defect, not an option.

## Purpose

Let users change low-risk interface preferences directly in `/settings` — with a
visible target scope, an atomic write, a reset that previews what will be
inherited, and concurrent-change detection — while operational and governance
values stay read-only until the governed-mutation path exists.

This is the smallest honest mutation slice: it proves the write path end to end
on changes that cannot weaken protection.

## In Scope

- Class A (interface preference) editing through the settings service:
  `Space` toggles Class A booleans, `Enter` begins an allowed edit
- Explicit target-scope behaviour with user scope as the documented default, and
  stated consequence when editing an inherited value creates an override
- Reset (`d`) with a preview of the value and source that becomes resolved after
  the declaration is removed
- Atomic persistence that preserves permissions, comments and formatting where
  the writer can do so safely
- Concurrent-modification detection before apply
- Accessibility preferences (motion, compact mode, timestamps, hints) as
  editable Class A entries
- Safe-write hardening: symlinks, path traversal, permission boundaries,
  interrupted and concurrent writes

## Out of Scope

- Class B and C changes of any kind — they stay inspect-only here and enter the
  proposal flow in SETGOV; `d` on a Class B/C declaration previews nothing and
  explains the controlling authority
- Class D secrets — never edited from this surface
- Non-interactive mutation (`anvil settings set`/`unset`) — the spec introduces
  no CLI mutation contract, and any future one needs its own specification
- MCP mutation tools
- Audit recording (SETGOV) — Class A changes produce no audit claim in v0.1

## Interfaces

**Depends on:**

- [settings-truth-contract](../archive/modules/settings-truth-contract.aps.md) (SETCON) — target
  writer identification, scope validation, consequence class, service boundary
- [settings-inspect-surface](./settings-inspect-surface.aps.md) (SETINS) — rows,
  detail panel and footer affordances the edit flow attaches to
- `crates/anvil-config` — canonical writers per scope
- `crates/anvil-tui` — edit affordances and confirmation surfaces

**Exposes:**

- Class A mutation API on the settings service (used later by SETGOV for the
  governed path's shared plumbing)
- Reset-with-preview behaviour reused by governed reset in SETGOV

**Coordinates with:**

- [unified-config-format](../archive/modules/unified-config-format.aps.md) (UCFG) — writer
  behaviour for the config file format
- [activation-tui](../archive/modules/activation-tui.aps.md) (ACTTUI) — consent posture: nothing
  writes without an explicit keypress on a surface that names the write

## Constraints

- **One writer** — the settings service; no interface touches config files.
- **Named scope** — every write states its target scope before it happens; the
  UI never infers the write target from the winning source.
- **No partial writes** — failed validation produces no write at all; a failed
  write leaves the original source untouched.
- **Reversible** — every Class A change can be reverted through the same path.
- **Class boundary is enforced, not advisory** — a Class B/C key reaching the
  Class A path is a hard error, not a downgrade.

## Acceptance Criteria

- [x] Class A preferences can be changed and reverted through the settings service
- [x] The target scope is visible, with user scope as the documented default
- [x] Editing an inherited value states the override consequence before applying
- [x] Reset previews the newly inherited value and source before removing the
      selected declaration
- [x] Concurrent modification prevents apply, preserves the source, and leaves
      the pending edit available for review or retry
- [x] Failed validation produces no partial configuration write
- [x] Tests cover symbolic links, path traversal, permissions, interrupted
      writes and concurrent writes

## Ready Checklist

Change status to **Ready** when:

- [x] SETINS-002 and SETINS-006 are Done
- [x] Canonical writer per Class A scope is declared in the catalogue:
      user scope → settings-service `user-config` (default write scope)
- [x] Safe-write policy (symlink, traversal, replace semantics) is documented:
      [`2026-09-16-settings-class-a-safe-write.md`](../specs/2026-09-16-settings-class-a-safe-write.md)
- [x] Class A key set agreed with the operator:
      `interface.compact`, `interface.timestamps`, `interface.motion`,
      `interface.hints`

## Work Items

### SETPREF-001: Class A mutation path

- **Intent:** Establish the single authorised write path and prove it on
  interface preferences.
- **Expected Outcome:** `Space` toggles Class A booleans and `Enter` begins an
  allowed edit, both routed through the settings service; the change validates
  before persistence, shows immediate feedback, and remains reversible; a
  non-Class-A key reaching this path is rejected rather than downgraded; no
  interface writes a configuration file directly.
- **Non-scope:** Class B/C flows; audit records
- **Dependencies:** SETCON-010, SETINS-002
- **Validation:** `cargo test -p eddacraft-anvil-config -- settings_write_class_a`; `cargo test -p eddacraft-anvil-settings -- settings_write_class_a`
- **Files:** `crates/anvil-config/src/settings_write.rs`, `crates/anvil-settings/src/mutate.rs`, `crates/anvil-tui/src/surfaces/settings/mod.rs`, `crates/anvil-cli/src/commands/settings.rs`
- **Confidence:** medium
- **Status:** Merged 2026-09-16 via PR #4741

### SETPREF-002: Target scope and override disclosure

- **Intent:** Make every write's destination explicit and its inheritance
  consequence visible.
- **Expected Outcome:** Each Class A edit names its target scope, defaulting to
  user scope unless the user selects another supported scope; editing an
  inherited value states that an override will be created before it is created;
  the write target comes from the catalogue's canonical writer, never from the
  winning source; read-only sources explain their approved update path instead of
  offering an edit.
- **Non-scope:** Session-scope overrides with expiry (SETGOV)
- **Dependencies:** SETPREF-001
- **Validation:** `cargo test -p eddacraft-anvil-settings -- settings_scope`
- **Files:** `crates/anvil-settings/src/mutate.rs`, `crates/anvil-tui/src/surfaces/settings/mod.rs`
- **Confidence:** medium
- **Status:** Merged 2026-09-16 via PR #4741

### SETPREF-003: Reset with inheritance preview

- **Intent:** Make removing a declaration a previewed decision rather than a
  keypress.
- **Expected Outcome:** `d` opens a reset preview for the declaration at a
  selected writable scope and never itself persists; the preview names the value
  and source that will become resolved afterwards; Class A reset can be
  confirmed; Class B/C reset stays read-only here and explains that it enters the
  proposal flow later.
- **Non-scope:** Governed reset (SETGOV)
- **Dependencies:** SETPREF-002
- **Validation:** `cargo test -p eddacraft-anvil-config -- settings_reset`; `cargo test -p eddacraft-anvil-settings -- settings_reset`
- **Files:** `crates/anvil-config/src/settings_write.rs`, `crates/anvil-settings/src/mutate.rs`, `crates/anvil-tui/src/surfaces/settings/mod.rs`
- **Confidence:** medium
- **Status:** Merged 2026-09-16 via PR #4741

### SETPREF-004: Atomic persistence

- **Intent:** Never leave a configuration source in a half-written state.
- **Expected Outcome:** Writes are atomic and preserve file permissions;
  comments and formatting survive where the supported writer can do so safely;
  a failed validation writes nothing; a failed write leaves the original source
  untouched; the operation reports persistence success separately from any
  claim about activation.
- **Non-scope:** Multi-file transactional writes and recovery protocol (SETGOV)
- **Dependencies:** SETPREF-001
- **Validation:** `cargo test -p eddacraft-anvil-config -- settings_persist`
- **Files:** `crates/anvil-config/src/settings_write.rs`
- **Confidence:** medium
- **Status:** Merged 2026-09-16 via PR #4741

### SETPREF-005: Concurrent-change detection

- **Intent:** Refuse to overwrite a source someone else changed underneath us.
- **Expected Outcome:** The service captures the source revision when the edit
  begins and rechecks it immediately before apply; a concurrent modification
  prevents the apply, preserves the source, surfaces what changed, and leaves the
  pending edit available for review or retry rather than discarding it.
- **Non-scope:** Proposal staleness rules (SETGOV)
- **Dependencies:** SETPREF-004
- **Validation:** `cargo test -p eddacraft-anvil-config -- settings_concurrency`
- **Files:** `crates/anvil-config/src/settings_write.rs`
- **Confidence:** medium
- **Status:** Merged 2026-09-16 via PR #4741

### SETPREF-006: Safe-write hardening

- **Intent:** Prove the write path holds under hostile and interrupted
  filesystem conditions.
- **Expected Outcome:** Writes obey
  [`2026-09-16-settings-class-a-safe-write.md`](../specs/2026-09-16-settings-class-a-safe-write.md);
  tests cover symlinked sources, traversal attempts, unwritable targets,
  interrupted writes and concurrent writers; failures produce a clear
  diagnostic that embeds no unredacted source values or paths.
- **Non-scope:** Recovery for multi-resource transactions (SETGOV)
- **Dependencies:** SETPREF-004
- **Validation:** `cargo test -p eddacraft-anvil-config -- settings_safe_write`
- **Files:** `crates/anvil-config/src/settings_write.rs`, `plans/specs/2026-09-16-settings-class-a-safe-write.md`
- **Confidence:** medium
- **Status:** Merged 2026-09-16 via PR #4741
