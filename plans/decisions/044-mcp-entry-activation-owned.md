# ADR-044: Anvil MCP Entries Are Activation-Owned; Backend Swaps Overwrite In Place

## Status

Proposed

## Date

2026-05-14

## Context

`anvil start` writes a `mcpServers.anvil` entry into the user's editor MCP
config files (`~/.cursor/mcp.json` and `~/.claude.json`, plus matching
per-workspace files where applicable). The entry's `command` and `args`
describe how editors should spawn the Anvil MCP server.

### Current drift policy (already in code)

The activation orchestrator at
`crates/anvil-cli/src/activation/orchestrator/install.rs` already
classifies drift between the on-disk entry and a freshly-built
`AnvilEntry`, and already has an explicit policy table:

| `DriftClass`  | Interactive default       | Non-interactive default   | Notes                                            |
|---------------|---------------------------|---------------------------|--------------------------------------------------|
| `NotPresent`  | pre-selected in picker    | auto-install              | fresh write; nothing to merge                    |
| `SafeDrift`   | pre-selected in picker    | auto-install              | recognised Anvil entry differs — rewrite in place |
| `UpToDate`    | not shown                 | skip                      | nothing to do                                    |
| `UnsafeDrift` | hidden from picker, refused | skipped with note         | foreign tool / unparseable — **never overwrite** |

The interactive picker is pre-selected for `SafeDrift`, meaning the user
sees the editor surface in a checklist with the box already ticked and
confirms with Enter. `UnsafeDrift` is never offered for overwrite at any
level (filtered out of the picker, and the install gate independently
refuses it). Other server entries (`mcpServers.other`, etc.) and
unrelated top-level keys (`theme`, `extensions`) are preserved on every
write via surgical JSON merge.

### What `anvil start` does NOT overwrite (current behaviour)

The MCP entry is the *only* surface where `anvil start` overwrites
on detected drift. Every other Anvil-touched file is **write-once**:

| File                          | Write policy                       | Refresh path                          |
|-------------------------------|------------------------------------|---------------------------------------|
| `.anvilrc`                    | write-only-if-missing              | delete the file, re-run `anvil start` |
| `.anvil/baseline.json`        | write-only-if-missing (LAUNCH-010) | `rm .anvil/baseline.json`             |
| `.anvil/architecture.json`    | write-only-if-missing              | delete and re-run                     |
| `mcpServers.anvil` (editors)  | drift-classify (see table above)   | `anvil start` (this ADR)              |

The MCP entry is special because it is a **self-registration in someone
else's config file**. Unlike project-local files, the user did not
author the Anvil entry — Anvil did, as a way for editors to find it.

### Why this needs an ADR now

What `anvil start` does on `SafeDrift` today is not pinned by an ADR. It
becomes load-bearing in the next release window because the **MCP server
backend is changing shape**:

- The TypeScript MCP shim is being retired (ADR-033)
- The Rust MCP full port (RMCPF module / ADR-030 surface drivers) becomes
  the canonical backend in `v0.7.0-beta`
- Every existing beta user's MCP entry will refer to the old shape on
  their next upgrade

Without a pinned contract, the orchestrator's behaviour drifts from
release to release and existing users either get an undocumented silent
upgrade (right outcome by accident), a half-broken state (wrong
outcome, no recovery path), or noisy re-confirmation prompts every
start (annoying, erodes the planless-first principle).

The new `anvil uninstall` command (landed 2026-05-14) provides a clean
heavy-reset path: `anvil uninstall --global && anvil start`. That path
must remain available regardless of which contract we pin here.

### Discovery — when do users actually re-run `anvil start`?

The above only delivers a transparent backend swap **if users run
`anvil start` after upgrading the binary**. They will not always do
this on their own. Today's flow:

- `brew upgrade eddacraft/tap/anvil` updates the binary; nothing prompts
  the user to re-activate
- `anvil update` (curl-installer / library path) updates the binary;
  same gap
- The first MCP-using editor session after upgrade will silently spawn
  the **old**-shape command from the stale config until `anvil start`
  is re-run

This discovery problem is owned by the **update flow** (DISTRIB module),
but the activation contract pinned here only makes sense if the update
flow plays its part. Treating discovery as out of scope here is fine;
treating it as someone else's problem is not.

## Decision

The `mcpServers.anvil` entry is **owned by the activation flow**. On
every `anvil start`, drift is classified per the existing policy table
and the entry is rewritten when classified as `SafeDrift`. The user
sees a single-line notice when a rewrite happens. Interactive flows
offer a picker (amended 2026-07-11: candidates start unticked — see
Amendments); non-interactive flows auto-apply.

This ADR **codifies the existing behaviour** rather than introducing a
new pattern. The amendments below pin the contract so it cannot drift,
and add a discovery requirement so the contract is actually exercised
on user upgrades.

Concretely:

1. **Single owner.** Only `anvil start` and `anvil mcp-config` may write
   the `mcpServers.anvil` key. Other commands read but never modify it.
   `anvil uninstall --global` removes the key entirely (surgical JSON
   edit; other servers preserved).
2. **Drift policy (pinned).** The orchestrator's existing
   `DriftClass` → action mapping is now contract:
   - `NotPresent`: install (interactive offered unticked — amended
     2026-07-11; non-interactive auto)
   - `SafeDrift`: rewrite in place (interactive offered unticked —
     amended 2026-07-11; non-interactive auto)
   - `UpToDate`: skip
   - `UnsafeDrift`: **never overwrite**. Skipped with note. Recovery
     is `anvil uninstall --global && anvil start`.
3. **User-visible notice (renderer follow-up).** When the orchestrator
   rewrites an entry, it should print a single line per affected
   surface:
   `MCP entry for <Claude Code | Cursor> updated to current Anvil
   backend.` No banner, no extra prompt beyond the existing picker.
   In non-interactive mode the notice is the only signal the user
   gets, so it must be terse and grep-friendly.
   **Current renderer state:** the activation orchestrator today
   emits a multi-line `install:` block with messages like
   "rewrote drifted entry" rather than this specific single-line
   notice. The wording above is the **contract this ADR establishes
   for the v0.7.0-beta cut**, not a description of today's output.
   The renderer change is a small ADTRUST-aligned follow-up (target:
   ADTRUST-001 / ADTRUST-006 render path) and must land in the same
   release that ships the Rust MCP backend so the pinned wording
   matches the shipped product. Until the renderer follow-up lands,
   the contract is "rewrite happens, with visible output naming the
   affected surface" — the exact wording is the deliverable.
4. **Interactive picker stays.** Existing interactive flows continue
   to present the picker. Amended 2026-07-11: candidates start
   **unticked**, so applying an editor-config write takes an explicit
   tick rather than a one-keystroke accept; Enter with nothing ticked
   writes nothing. The picker remains a single prompt, not a
   re-confirm-every-start flow.
5. **Opt-out flag.** `anvil start --keep-mcp` skips MCP entry rewrites
   entirely (`SafeDrift` is treated as `UpToDate` for the duration of
   that invocation). Intended for users who have customised the entry
   on purpose (e.g. wrapping Anvil's command in a launcher). Default
   remains automatic rewrite.
6. **Heavy reset is always available.** `anvil uninstall --global` +
   `anvil start` is the documented fallback for any user whose entry
   has drifted into `UnsafeDrift` territory or who otherwise wants a
   clean slate. This stays the recovery path of last resort.
7. **Other server entries are never touched.** Sibling entries
   (`other`, `notion`, etc.) and unrelated top-level keys (`theme`,
   `extensions`) are preserved on every write. The orchestrator's
   merge tests pin this invariant.
8. **Protocol shape changes still get a prompt.** If a future release
   moves the MCP **protocol** — wire format, schema version, or tool
   surface in a way that affects editor-side configuration — that
   triggers an explicit migration prompt via `anvil migrate`
   (DISTRIB-005). This ADR covers the transparent backend-swap case
   only.
9. **Discovery is the update flow's responsibility.** This ADR is only
   honest if existing users actually run `anvil start` after a binary
   upgrade. The DISTRIB module MUST:
   - Have `anvil update` print a final-line hint:
     `Run \`anvil start\` in each Anvil-enabled project to pick up the
     new backend.` (DISTRIB-001 follow-up.)
   - Have `anvil version --check` flag stale MCP entries when a newer
     binary is on disk than the entry was last written against
     (DISTRIB-002 follow-up).
   - Long-term: persist a `binary_version` marker in the MCP entry's
     `env` block so `anvil status` and `anvil doctor` can detect a
     stale entry and emit a one-line hint to run `anvil start`. The
     marker is informational only — drift classification still happens
     by comparing `command`/`args` shapes.

### Out of Scope

- **Project-local Anvil files** (`.anvilrc`, `.anvil/baseline.json`,
  `.anvil/architecture.json`) remain write-once. This ADR does not
  authorise rewriting them on drift.
- **Foreign MCP entries** (other servers in the same config file)
  remain untouched. The surgical JSON-merge contract is invariant.
- **Editor-supplied `mcpServers.anvil` entries** (i.e. cases where an
  editor or organisation has pre-populated an Anvil entry without
  using `anvil start`) classify as `UnsafeDrift` because they will not
  match the recognised Anvil shape. They are refused, not overwritten.

## Rationale

The activation flow already does the right thing for the MCP entry; the
gap was that the right thing was not pinned anywhere reviewable. This
ADR turns a piece of orchestrator code that "happens to" do the
sensible thing into a contract that can be tested, regression-protected,
and cited by future work.

Keeping the interactive pre-selected picker matches user expectations
formed by the rest of `anvil start` (which today does present some
visibility for MCP installs). The one-line notice is the
non-interactive equivalent and the breadcrumb for "why did `mcp.json`
get touched?" without forcing the user to read verbose logs.

The discovery requirement is the load-bearing addition. Without it,
backend swaps land in the binary but never reach the user's editor
until the next time they happen to run `anvil start`. The pre-existing
silent-overwrite path was sufficient when Anvil's MCP entry never
meaningfully changed shape; it is not sufficient now that the entry's
backend is about to swap from a Node-based shim to a Rust binary.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Codify existing behaviour: pre-selected picker interactive, auto non-interactive, with notice + discovery** (chosen) | No code change to install path; matches current UX; pinned by ADR + tests; discovery requirement closes the upgrade gap | Discovery work falls on DISTRIB; ADR depends on cross-module follow-through |
| Fully silent overwrite, remove the interactive picker | Lowest friction | Reduces user visibility; users may not notice their MCP config changed; conflicts with current install UX users have learned |
| Interactive prompt on every detected drift | Maximum visibility | Friction every start after upgrade; users will mash `y`; conflicts with planless-first |
| Manual `anvil migrate` only; activation flow leaves entries alone | Maximally explicit | Existing users stuck on the old backend until they discover and run a migration command; defeats automatic upgrade |
| Do nothing — break existing users | Smallest code change | Beta users see "MCP server failed to start" with no migration path |
| Pin entries to a specific Anvil version; require user to opt into upgrade | Strongest control | Far more complex; introduces version pinning into config files; no real user demand |

## Consequences

- **Positive:** Existing beta users transparently upgrade to the Rust MCP
  server on first `anvil start` after the binary update. The MCP entry
  contract is pinned and testable, not just emergent.
- **Positive:** Sibling MCP entries (other servers, unrelated keys) are
  preserved by contract, not by accident. The orchestrator's tests can
  cite this ADR.
- **Positive:** Project-local Anvil files (`.anvilrc`, baseline,
  architecture) keep their write-once semantics — this ADR does not
  open the door to broader config rewriting.
- **Positive:** Aligns with the planless-first principle — the user does
  not need a plan, prompt, or migration command for routine backend
  upgrades.
- **Positive:** `anvil uninstall --global` becomes the universal "reset
  my MCP config" answer when anything weird happens, simplifying support.
- **Negative:** Users who have manually customised the `mcpServers.anvil`
  entry lose their customisation on next `anvil start` unless they pass
  `--keep-mcp`. This trade-off is acceptable because the entry is
  documented as Anvil-managed and the opt-out flag exists.
- **Negative:** The user-visible notice becomes load-bearing copy. It
  must be terse, accurate, and easy to grep for in CI logs.
- **Negative:** Discovery depends on DISTRIB landing the hints described
  in decision point 9. Without those, users still need to run
  `anvil start` manually after upgrade, and only those who do get the
  new backend.
- **Risk:** Silent backend swap could mask a real misconfiguration that
  the user introduced on purpose.
- **Mitigation:** Document `--keep-mcp` in the upgrade runbook and the
  `anvil start --help` output. The notice itself names the surface
  affected so the user can react.
- **Risk:** A future release may genuinely need an explicit migration
  prompt (protocol change, not backend change) and the silent default
  conditions users to ignore notices.
- **Mitigation:** This ADR explicitly carves out protocol changes —
  those route through `anvil migrate` (DISTRIB-005) with explicit user
  consent, not the silent path.
- **Risk:** DISTRIB-001/-002 do not land before the Rust MCP backend
  ships, leaving the discovery gap open.
- **Mitigation:** Until DISTRIB-001 ships, the `v0.7.0-beta` release
  notes and migration runbook explicitly call out "run `anvil start`
  after upgrading to pick up the new MCP backend." This is a
  documentation patch, not a product fix, and it is sufficient for the
  initial cut but not durable.

## References

- Related ADRs:
  - ADR-001 (planless-first) — silent default for Anvil-owned entries
    aligns with this principle
  - ADR-002 (warnings over blocks) — applies in spirit; entries we own
    are not user code
  - ADR-030 (surface drivers supersede napi cutover) — drives the
    backend swap that motivates this ADR
  - ADR-033 (park IDE MCP, retire TS scanner) — the TS MCP server is the
    backend being replaced
  - ADR-036 (daemon scope and boundaries) — execution scope context for
    MCP server placement
- APS modules:
  - `RMCPF` (Rust MCP full port) — the backend swap this ADR governs
  - `DISTRIB` (Distribution and Self-Update, proposed) — owns the
    discovery follow-ups (`anvil update` hint, `anvil version --check`
    stale-entry detection, optional binary-version marker) and
    `anvil migrate` for protocol-change cases
  - `ADOPT-005` (clean uninstall, shipped 2026-05-14) — the heavy-reset
    path
  - `ADTRUST` (Adoption Trust Surface, proposed) — the notice copy should
    align with ADTRUST's status-line conventions; the binary-version
    marker informs `anvil status` and `anvil doctor`
- Code:
  - `crates/anvil-cli/src/activation/mcp_client/cursor.rs`
  - `crates/anvil-cli/src/activation/mcp_client/claude_code.rs`
  - `crates/anvil-cli/src/activation/orchestrator/install.rs`
    (current `DriftClass` table — see module-level doc comment)
  - `crates/anvil-cli/src/activation/orchestrator/mod.rs`
  - `crates/anvil-cli/src/commands/uninstall.rs` (heavy-reset path)

## Amendments

### 2026-07-11 — interactive picker defaults unticked (CIB-184)

The original decision (and the "chosen" row of the alternatives table)
kept the interactive picker **pre-selected**. That default was reversed
by the consent posture adopted for the release user journeys: repo- and
editor-config writes take an explicit, named, unticked consent
(CIB-165 set the precedent for the workflow picker; ACTTUI-009 / PR
#3263 wired the same posture through the activation TUI; the
first-run council review C-009 flagged the live MCP picker as the
remaining inconsistency; CIB-184 / PR #3279 applied it).

What changed: `NotPresent` and `SafeDrift` candidates are still
offered interactively, but start unticked, and Enter with nothing
ticked writes no MCP config. Everything else in this ADR is
unchanged: single ownership, the non-interactive auto-install policy,
the `UnsafeDrift` refusal, `--keep-mcp`, and the heavy-reset path.

References: `plans/reviews/2026-07-09-acttui-first-run-journeys.md`
(C-009), `plans/specs/2026-07-11-release-user-journeys-conductor.md`
(consent constraints), CIB-165 owner decision 2026-07-04.
