# ADR-033: Archive IDE/MCP Surfaces; Retire TS Scanner Now

## Status

Proposed

## Date

2026-04-28

## Context

Anvil's TypeScript scanner, TypeScript suppression parser, and the
shared scanner-parity harness exist for one reason: the in-process
integration surfaces — the VSCode extension
(`packages/vscode-extension/`) and the TypeScript MCP server
(`packages/mcp-server/`) — cannot shell out to the CLI and have
historically embedded the TS engine directly. ADR-026 made the Rust
scanner authoritative on that constraint, and ADR-029 narrowed it
further by refusing to add new comment styles to the TS suppression
parser. ADR-030 then superseded the napi cutover plan with a
daemon-driver path (DRVR), and accepted — explicitly — a 2+ month
window where the TS scanner remains in the codebase while INTD and
DRVR build out.

Three things have changed since ADR-030 landed:

1. **The Rust MCP launch shim (RMCP) is essentially done.** RMCP is
   In Progress 7/8 — `anvil mcp serve --stdio` exists in Rust, the
   pre-write validation tool surface is wired, the canonical
   diagnostic shape is published, and `anvil mcp install` lands the
   same binary into Cursor and Claude Code. The launch-critical MCP
   path no longer needs the TypeScript MCP server.
2. **RMCPF is queued to take MCP's full feature footprint into
   Rust.** The next-release work item carries the inventory,
   parity, and migration burden. The TypeScript MCP server is no
   longer where MCP feature work lands.
3. **The VSCode extension is not on the H1 ship and not on the H2
   ship.** Per `plans/next-steps.md`, H1 ships against the
   in-process Rust CLI surfaces; H2 ships RTAI on the daemon. The
   editor driver is DRVR-003, blocked on INTD, and explicitly
   sequenced after H2's headline cut. No release window between
   now and DRVR-003 has the VSCode extension as a demo target.

The "TS stays alive *for the surfaces*" carve-out therefore has no
active surface to justify it. The TS scanner, the TS suppression
parser, the parity harness, and the per-PR CI cost of all three are
overhead paid for paused consumers.

ADR-030 chose to absorb that cost on the assumption that the
surfaces would be carried through the INTD→DRVR window. That
assumption is what this ADR revises: the surfaces are paused, not
carried, for the duration of that window. The cost line collapses
with them.

This ADR was triggered while reviewing ADR-027 / ADR-028 / ADR-029
for explicit approval per the release plan. ADR-028 and ADR-029 in
particular reason about a "soon-to-be-retired TS scanner" as if its
retirement were on a known schedule pinned to napi-rs / DRVR. That
schedule is now closer than those ADRs assumed, and the trigger is
different: surfaces pausing, not migration completing.

## Decision

The VSCode extension and TypeScript MCP server are **archived**, and
the TypeScript scanner stack that exists to feed them is **archived
alongside them**. They originally used the project's then-current
`archive/<name>/` convention (precedent: `archive/anvil-cli-node/`
from ADR-012, `archive/anvil-tui-ink/` from ADR-011a). As of the
2026-06-21 archive cleanup, those historical packages live in the
sibling `eddacraft/anvil-archive` repository instead of this workspace,
so no pnpm/Nx exclusion glob is required here.

Concretely:

1. **VSCode extension archived.** `packages/vscode-extension/` →
   `archive/anvil-vscode-extension/` via `git mv`. No feature work,
   no release, no CI build, no marketplace publish. The archived
   package retains its full structure (src, package.json, tsconfig)
   and carries an Archived banner in its README following the
   `anvil-cli-node` precedent. The extension returns as a **new**
   `packages/`-side package when an IDE driver path lands
   (DRVR-003 against the intercept daemon, a napi-rs embedding, or
   another mechanism — that path-of-return decision is reopened,
   not constrained here). The archive is reference material for
   that work; the new package is not expected to be a literal
   un-archive of this one.

2. **TypeScript MCP server archived.** `packages/mcp-server/` →
   `archive/anvil-mcp-server/` via `git mv`. No new features, no
   release. The launch-critical MCP path runs through RMCP
   (`anvil mcp serve --stdio`) in the single Rust binary. Existing
   MCP feature parity is RMCPF's responsibility, executed against
   the Rust binary, **not** by extending the archived TS package.
   RMCPF reads the archived `archive/anvil-mcp-server/src/` as
   frozen contract source — that's the "frozen reference" framing
   in literal form.

3. **TypeScript scanner archived.** The antipattern scanner at
   `packages/anvil/core/src/antipattern/` (scanner, registry
   loader, supporting modules) is moved to
   `archive/anvil-ts-scanner/antipattern/` and its inbound imports
   from active packages are removed. Its only consumers were the
   now-archived surfaces and the parity harness.

4. **TypeScript suppression parser archived.** The parser at
   `packages/anvil/core/src/suppression/parser.ts` is moved to
   `archive/anvil-ts-scanner/suppression/parser.ts`. The Rust
   suppression parser in
   `crates/anvil-checks/src/antipattern/scanner.rs` becomes the
   sole implementation. ADR-029's "no new comment styles in TS"
   rule is moot once there is no active TS parser; ADR-029's
   underlying decision (Rust authoritative, schema shared) is
   unaffected.

5. **Parity harness archived.** `tests/scanner-parity/` (TS side)
   moves to `archive/anvil-ts-scanner/scanner-parity/`. The Rust
   side (`crates/anvil-checks/tests/scanner_parity.rs`) is deleted
   — there is no second engine to compare against, so the test has
   no meaning. With one engine, parity testing collapses into the
   regular `anvil-checks` test surface.

6. **napi binding crate stays as-is.** `crates/anvil-checks-napi/`
   remains `"private": true` and remains in CI as a build canary so
   the Rust↔Node interop path does not rot before the surfaces
   return. It is **not** publishable, **not** the on-ramp for any
   active consumer, and is a candidate for deletion if its CI cost
   exceeds its option value during the pause window.

7. **CI scope reduces.** Workflows that build, test, or lint the
   paused TS packages and the retired scanner / parity / TS
   suppression parser switch off. The Rust test matrix, the
   workspace-wide lint of remaining TS packages, and the napi
   build canary are unaffected.

8. **TSRET re-pointed.** TSRET-005 (delete TS scanner + retire
   parity harness) executes under this ADR rather than waiting on
   DRVR-003/-004. TSRET-006 (transition-window engine-version
   diagnostics) is dropped — the transition window collapses.
   TSRET as a module reaches its terminal state once 005 lands.

9. **DRVR re-pointed.** DRVR-003 (VSCode editor driver) and
   DRVR-004 (MCP driver) move from "blocked on INTD-002 et al" to
   "deferred until surfaces resume". The rest of DRVR (shared
   client, protocol, capability/trust rules, telemetry contracts)
   continues on its existing INTD-blocked schedule and remains the
   intended return path for VSCode when the surfaces come back.

10. **RTAI and INTD scope unaffected.** Real-time AI-output
    validation lands on the daemon as planned; pausing the IDE/MCP
    surfaces does not pause the daemon work or the headline H2
    capability. RTAI's MCP demo path runs through RMCP, not
    through the paused TS MCP server.

## Rationale

The original justification for keeping TS code alive — "in-process
surfaces need it now and can't shell out" — required those surfaces
to be active. They are not. Carrying dual-engine cost (every rule
change re-validated against TS, parity harness CI on every PR,
TS-side regex divergence as user-visible UX risk) for surfaces
nobody is shipping is the worst trade in the architecture: full
cost, zero realised benefit.

Three forcing functions made this the right call now rather than
"after DRVR":

- **RMCP collapses MCP's TS dependency.** With `anvil mcp serve`
  shipping in Rust, no demo-relevant MCP path runs through the TS
  server. The TS package becomes a museum piece on the day RMCP
  ships.
- **VSCode is not the demo.** H1's pre-flight items and H2's RTAI
  capability both run on Rust surfaces. Keeping the editor extension
  alive in case a demo materialises adds cost without changing what
  ships.
- **The archive is reversible cheaply.** Source is preserved
  intact under `archive/<name>/`; git history is unbroken across
  the `git mv`; the napi crate stays compiling. When DRVR-003
  lands or another return path is chosen, the extension comes back
  as a **new** active package with a clean target — informed by
  the archive but not bound to it — rather than as an unmaintained
  dependency on TS code rotting in `packages/`.

ADR-030 stays the right answer for *how* surfaces return (drivers
on the daemon, LSP-shaped, not per-surface napi embeddings). This
ADR is about *when* TS engine code dies relative to the surfaces
returning — and the answer is: now, regardless.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Pause IDE/MCP surfaces; retire TS scanner / suppression parser / parity harness now** *(chosen)* | Collapses dual-engine cost during a window that produces no surface release; ADR-026/-029/-030 carve-outs become moot; TSRET reaches terminal state without waiting on DRVR; CI cost line drops for paused packages | Surfaces are unavailable until a return path lands; users tracking the VSCode extension see it go quiet; reverting requires either DRVR-003 landing or an explicit un-pause decision |
| Hold ADR-030 sequencing; carry TS scanner through DRVR build (status quo) | Surfaces remain importable; no signalled pause for users | Pays full dual-engine cost across the entire INTD→DRVR window for surfaces nobody is shipping; parity harness stays load-bearing; rule changes still cost 2x |
| Pause surfaces but keep TS scanner / parser / parity harness alive | Surfaces explicitly off, but engine code remains a takeable dependency for any future surface | Pays the cost line without the consumers that justified it; TS code rots without a CI consumer that exercises it in anger |
| Retire TS scanner without pausing surfaces | Same end-state for engine code | Breaks VSCode and MCP server builds without naming the breakage; "unintentionally paused" is worse than "intentionally paused" |
| Land the napi cutover (TSRET-003/-004 un-superseded) before retiring TS code | Single-engine in-process for surfaces; no pause | Re-opens the ADR-030 decision the project just made; per-platform npm publication, install-test matrix, provenance work all return; defeats the purpose of pausing |

## Consequences

- **Positive:**
  - Dual-engine maintenance cost goes to zero immediately. Rule
    changes land once, in the Rust scanner, against one fixture
    surface.
  - ADR-026 §7 (TS scanner stays for in-process surfaces) and
    ADR-029 (no new comment styles in TS parser) become moot rather
    than load-bearing — both ADRs' decisions are unaffected, but
    their carve-outs no longer constrain new work.
  - TSRET reaches a terminal state without waiting on DRVR. The
    parity harness (`tests/scanner-parity/`) retires.
  - CI shrinks: workflows that build/test the paused TS packages
    and the parity harness switch off. PR turnaround improves.
  - The intent is legible: paused surfaces are paused on purpose.
    A user who finds the VSCode extension unmaintained can read
    this ADR and the module-level pause notes rather than guessing
    whether it is in flight.
  - RMCP's role as the launch MCP path is reinforced: the TS MCP
    server stops being a parallel option for any active flow.

- **Negative:**
  - VSCode extension users (small audience today) lose their
    in-editor diagnostics until DRVR-003 or another return path
    lands. The extension lives under `archive/` and does not
    build, install, or publish.
  - The MCP server's full historical feature surface is reachable
    only via RMCPF's port from this point on — anyone running the
    TypeScript MCP server today against newer rule sets gets no
    upgrade path until RMCPF ships. The archived
    `archive/anvil-mcp-server/` is the contract source RMCPF reads.
  - The napi crate continues to compile but has no consumer
    exercising its API surface in production. Drift risk during
    the archive window — mitigated by the build canary, not
    eliminated.
  - The archive matches the established pattern
    (`anvil-cli-node`, `anvil-tui-ink`); the cost of an archive
    that becomes permanent is just dead bytes in `archive/` — the
    pattern absorbs that fine.

- **Risks:**
  - The pause window extends past expectations and the napi
    binding rots into something that doesn't compile against a
    newer `anvil-checks`. Mitigation: the build canary in CI fails
    fast if the binding stops compiling.
  - A user discovers the unmaintained VSCode extension before they
    discover the README explaining the pause, files an issue,
    bounces. Mitigation: extension `README` and marketplace
    listing (if still present) carry the pause notice as a top
    line; npm `package.json` description updated.
  - Reverting the pause without DRVR-003 means re-introducing the
    TS scanner code from history rather than from a maintained
    branch. Acceptable: the alternative — keeping it alive without
    consumers — is the more expensive failure mode.

- **Mitigations:**
  - Both paused packages get a top-of-`README.md` notice citing
    this ADR and naming the return path (DRVR-003 / DRVR-004 /
    RMCPF).
  - The napi build canary stays in CI for the duration of the
    pause; if it goes red and isn't fixed within one release
    window, the crate is removed under a follow-up ADR rather than
    silently held alive.
  - When IDE/MCP return, the return ADR cites this one as the
    pause that preceded it.

## Amendments to prior ADRs

This ADR amends — but does not supersede — the following:

- **[ADR-026](./026-rust-scanner-authoritative.md):** §7 (TS scanner
  stays for in-process surfaces) and §8 (long-term retirement after
  napi-rs) are amended. The TS scanner is retired now under this
  ADR; the carve-out for in-process surfaces collapses with the
  pause. ADR-026's authoritative-Rust decision is unaffected.
- **[ADR-028](./028-markdown-governance-crate.md):** Rationale that
  cited "soon-to-be-retired TS scanner" is strengthened (retirement
  is happening, not pending). Decision unchanged: markdown
  governance lives in `crates/anvil-markdown-governance/`.
- **[ADR-029](./029-suppression-parser-authority.md):** The "no new
  comment styles in TS parser" rule is moot once the TS parser is
  retired. The Rust-authoritative decision is unaffected and is
  now the only implementation rather than the authoritative one of
  two.
- **[ADR-030](./030-surface-drivers-supersede-napi-cutover.md):**
  The Sequencing Decision (Option A) — "INTD picks up after
  v0.4.0-beta; TS scanner remains for the INTD→DRVR window" — is
  amended. INTD sequencing is unchanged; the TS-scanner-remains
  half is supplanted. TSRET-005 unblocks now under ADR-033 rather
  than under DRVR-003/-004. DRVR-003/-004 remain the intended
  return path for surfaces but are deferred until the surfaces
  resume.

ADR-027 (Pack architecture) is unaffected: pack substrate access,
crate location, activation model, and tier gating are independent of
which scanner consumers exist.

## References

- Related ADRs: [ADR-026](./026-rust-scanner-authoritative.md)
  (Rust scanner authoritative — amended),
  [ADR-028](./028-markdown-governance-crate.md) (rationale
  strengthened), [ADR-029](./029-suppression-parser-authority.md)
  (carve-out moot — amended),
  [ADR-030](./030-surface-drivers-supersede-napi-cutover.md)
  (sequencing amended)
- APS modules:
  - [anvil-ts-scanner-retirement](../modules/anvil-ts-scanner-retirement.aps.md)
    (TSRET — TSRET-005 unblocks under this ADR; TSRET-006 dropped)
  - [surface-drivers](../archive/modules/surface-drivers.aps.md) (DRVR —
    DRVR-003/-004 deferred until surfaces resume; remainder
    unaffected)
  - [rust-mcp-launch-shim](../archive/modules/rust-mcp-launch-shim.aps.md)
    (RMCP — owns the launch MCP path during the pause)
  - [rust-mcp-full-port](../modules/rust-mcp-full-port.aps.md)
    (RMCPF — re-pointed: starts from "TS MCP server is paused"
    rather than "actively migrating from")
  - [intercept-daemon](../archive/modules/intercept-daemon.aps.md) /
    [intercept-rules](../archive/modules/intercept-rules.aps.md) /
    [intercept-launcher](../archive/modules/intercept-launcher.aps.md)
    (INTD/INTR/INTL — unaffected)
- Code (archived under this ADR — surface packages):
  `packages/vscode-extension/` → `archive/anvil-vscode-extension/`,
  `packages/mcp-server/` → `archive/anvil-mcp-server/`
- Code (archived under this ADR via TSRET-005 — engine and
  cascade): all paths land under `archive/anvil-ts-scanner/`.
  - `packages/anvil/core/src/antipattern/` → `core-antipattern/`
  - `packages/anvil/core/src/suppression/` → `core-suppression/`
    (whole directory; `service.ts` and `store.ts` consume the
    parser internally, so they archive together)
  - `packages/anvil/core/src/drift/` → `core-drift/` (drift was
    scoped to anti-pattern + suppression deltas; with both
    archived, drift has nothing to capture)
  - `packages/anvil/core/src/explain/antipattern-explainer.ts`
    and its test → `core-explain-antipattern.ts` /
    `core-explain-antipattern.test.ts`. `explain-service.ts`
    keeps its boundary explanations and survives slimmed
  - `packages/anvil/runtime/src/gate/` → `runtime-gate/`
    (gate-runner + all checks)
  - `packages/anvil/runtime/src/export/constraint-collector*`
    and the formatters folder → `runtime-export/`
  - `tests/scanner-parity/` (TS side) → `scanner-parity/`
- Code (deleted under this ADR via TSRET-005):
  `crates/anvil-checks/tests/scanner_parity.rs` — Rust-side parity
  test, no second engine to compare against
- Type extraction:
  `packages/anvil/core/src/warnings/types.ts` (new) carries a
  minimal `Warning` / `Location` / severity / category / confidence
  shape so active consumers (`warnings/warning-id`,
  `explain/explain-service`) keep a typed handle. Full zod
  schemas, fingerprint helpers, and producer types stay in the
  archive — they are re-derivable from the Rust scanner's
  emitted JSON if needed.
- Code (retained as build canary):
  `crates/anvil-checks-napi/`
- Archive convention reference: `archive/anvil-cli-node/` (Node
  CLI archived under ADR-012) and `archive/anvil-tui-ink/` (Ink
  TUI archived under ADR-011a) — same original `git mv` pattern
  and README Archived banner; these historical packages now live in
  `eddacraft/anvil-archive`, outside this pnpm/Nx workspace.
- Strategic frame:
  [`plans/next-steps.md`](../next-steps.md) — H1/H2/H3 sequencing,
  RTAI as headline, RMCP as launch MCP path
