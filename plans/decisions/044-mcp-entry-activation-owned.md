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

The activation orchestrator already classifies drift between the on-disk
entry and a freshly-built `AnvilEntry`:

- `UpToDate` — no action needed
- `NotPresent` — insert
- `ConfigPresent` — Anvil entry exists but differs
- `UnsafeDrift` — entry shape cannot be safely compared (e.g. invalid
  UTF-8 path)

What the orchestrator does on `ConfigPresent` today is not pinned by an
ADR. This becomes load-bearing in the next release window because the
**MCP server backend is changing shape**:

- The TypeScript MCP shim is being retired (ADR-033)
- The Rust MCP full port (RMCPF module / ADR-030 surface drivers) becomes
  the canonical backend in `v0.7.0-beta`
- Every existing beta user's MCP entry will refer to the old shape on
  their next upgrade

We need a single, explicit contract for "what happens to the user's MCP
config when Anvil's backend changes." Without one, the orchestrator's
behaviour drifts from release to release and beta users either get a
silent upgrade (good outcome, undocumented), a half-broken state
(bad outcome, no recovery path), or noisy re-confirmation prompts every
start (annoying, erodes the planless-first principle).

The new `anvil uninstall` command (landed 2026-05-14) provides a clean
heavy-reset path: `anvil uninstall --global && anvil start`. That path
must remain available regardless of which contract we pick here.

## Decision

The `mcpServers.anvil` entry is **owned by the activation flow**. On every
`anvil start`, drift is detected and the entry is rewritten in place to
match the binary's current `AnvilEntry`. The user sees a one-line notice
when a rewrite happens. No interactive prompt, no migration command, no
deferral.

Concretely:

1. **Single owner.** Only `anvil start` and `anvil mcp-config` may write
   the `mcpServers.anvil` key. Other commands read but never modify it.
   `anvil uninstall --global` removes the key entirely (surgical JSON
   edit; other servers preserved).
2. **Drift policy.** When the activation flow detects `ConfigPresent`
   drift (existing entry differs from the freshly-built `AnvilEntry`),
   it overwrites in place. `UnsafeDrift` (unparseable file, etc.) is
   still surfaced as an error — the orchestrator does not attempt to
   "fix" a broken config file.
3. **User-visible notice.** A single line: `MCP entry for <Claude Code |
   Cursor> updated to the current Anvil backend.` Printed at most once
   per editor surface per `anvil start` invocation. No banner, no
   confirmation prompt.
4. **Opt-out flag.** `anvil start --keep-mcp` skips MCP entry rewrites
   entirely. Intended for users who have customised the entry on purpose
   (e.g. wrapping Anvil's command in a launcher). Default remains
   automatic overwrite.
5. **Heavy reset is always available.** `anvil uninstall --global` +
   `anvil start` is the documented fallback for any user whose entry has
   drifted into `UnsafeDrift` territory or who otherwise wants a clean
   slate. This stays the recovery path of last resort.
6. **Other server entries are never touched.** Anvil owns the
   `mcpServers.anvil` key alone. Sibling entries (`other`, `notion`,
   etc.) and unrelated top-level keys (`theme`, `extensions`) are
   preserved on every write.
7. **Protocol shape changes still get a prompt.** If a future release
   moves the MCP **protocol** (not just the binary backend) — wire
   format, schema version, or tool surface in a way that affects
   editor-side configuration — that triggers an explicit migration
   prompt via `anvil migrate` (DISTRIB-005). This ADR covers the
   transparent backend-swap case only.

## Rationale

The activation flow is already invoked on every meaningful Anvil session
start, already classifies drift, and already knows how to merge entries
without disturbing siblings. Making it the single owner converts a
recurring distribution question ("how do existing users get the new MCP
server?") into a no-op for the user — they run `anvil start` once and
the entry updates.

Silent default is appropriate because the entry is **Anvil's to set**.
We are not changing the user's tooling, only updating our own
self-registered service definition. A confirmation prompt every time
would be the same friction class as prompting before applying a
dependency-bump in `Cargo.lock` — technically correct, practically
annoying, and likely to be dismissed without reading.

The user-visible notice exists for one reason: if a user runs
`anvil start` expecting nothing to happen and the start command rewrites
their config, the notice gives them the breadcrumb needed to understand
"why did `mcp.json` get touched?" without forcing them to dig through
verbose logs.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Activation-owned, silent overwrite with notice** (chosen) | Zero user friction; matches existing planless-first principle; reuses existing drift classifier; user gets new backend on next `anvil start` | Behaviour is invisible without reading the notice — depends on the notice being legible |
| Interactive prompt on detected drift | Visible; user consents | Friction every start after upgrade; conflicts with planless-first; users will mash `y` without reading |
| Manual `anvil migrate` only; activation flow leaves entries alone | Maximally explicit | Existing users stuck on the old backend until they discover and run a migration command; defeats automatic upgrade |
| Do nothing — break existing users | Smallest code change | Beta users see "MCP server failed to start" with no migration path |
| Pin entries to a specific Anvil version; require user to opt into upgrade | Strongest control | Far more complex; introduces version pinning into config files; no real user demand |

## Consequences

- **Positive:** Existing beta users transparently upgrade to the Rust MCP
  server on first `anvil start` after the binary update. No migration
  runbook step required for the common case.
- **Positive:** Sibling MCP entries (other servers, unrelated keys) are
  preserved by contract, not by accident. This is now an explicit
  invariant the orchestrator's tests can pin.
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
- **Risk:** Silent overwrite could mask a real misconfiguration that the
  user introduced on purpose.
- **Mitigation:** Document `--keep-mcp` in the upgrade runbook and the
  `anvil start --help` output. The notice itself names the surface
  affected so the user can react.
- **Risk:** A future release may genuinely need an explicit migration
  prompt (protocol change, not backend change) and the silent default
  conditions users to ignore notices.
- **Mitigation:** This ADR explicitly carves out protocol changes —
  those route through `anvil migrate` (DISTRIB-005) with explicit user
  consent, not the silent path.

## References

- Related ADRs:
  - ADR-001 (planless-first) — silent default aligns with this principle
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
  - `DISTRIB` (Distribution and Self-Update, proposed) — owns
    `anvil migrate` for protocol-change cases
  - `ADOPT-005` (clean uninstall) — the heavy-reset path
  - `TRUST` (Adoption Trust Surface, proposed) — the notice copy should
    align with TRUST's status-line conventions
- Code:
  - `crates/anvil-cli/src/activation/mcp_client/cursor.rs`
  - `crates/anvil-cli/src/activation/mcp_client/claude_code.rs`
  - `crates/anvil-cli/src/activation/orchestrator/mod.rs`
  - `crates/anvil-cli/src/commands/uninstall.rs` (heavy-reset path)
