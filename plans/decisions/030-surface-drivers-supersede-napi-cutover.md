# ADR-030: Surface Drivers on the Intercept Daemon Supersede the napi Cutover

## Status

Proposed

## Date

2026-04-23

## Context

ADR-026 made the Rust scanner authoritative and explicitly preserved the
TypeScript scanner in `packages/anvil/core/src/antipattern/` for the
in-process surfaces that cannot shell out to the CLI: the VSCode
extension and the MCP server. The `anvil-ts-scanner-retirement` module
(TSRET) was written against that frame. Its plan:

- **TSRET-001:** napi-rs binding spike — *landed*.
- **TSRET-002:** cross-platform prebuilds and npm publication of
  `@eddacraft/anvil-checks-native` — workflow built and green, remaining
  work narrowed to publish flip + out-of-band install tests + provenance
  decision (see that work item).
- **TSRET-003:** VSCode extension swaps `@eddacraft/anvil-core/antipattern`
  imports for the napi package — *not started*.
- **TSRET-004:** MCP server does the same — *not started*.
- **TSRET-005:** delete TS scanner + retire the parity harness —
  *not started*.

Since that plan was written, two adjacent specifications have landed:

1. **ADR-015 — Intercept Loop Enforcement.** Introduces a long-running
   Rust daemon (`anvil-intercept`) with an NDJSON / JSON-RPC 2.0
   interface over Unix domain sockets (Linux/macOS) or named pipes
   (Windows). The daemon reuses `anvil-kernel`'s watcher and
   `anvil-checks`'s scanners as `InterceptRule` implementations. APS
   modules INTD / INTL / INTR are drafted for v1; editor and MCP
   surfaces are explicitly out of scope for v1 but named as the target
   v2 drivers.
2. **Driver framework design + ADR** (`plans/specs/anvil-driver-framework/`).
   Generalises the intercept loop into a driver taxonomy — local shell,
   remote shell, process, tmux, editor, web-session, and MCP drivers —
   all riding the same control plane and telemetry lanes. Positions MCP
   as a "secondary or fallback driver, not the foundational control
   plane," and treats the editor as a first-class driver with its own
   capability set (warn, save block, session fence).

Both documents land the daemon as the authoritative host of the scan /
enforcement pipeline. That reframes TSRET: the question is no longer
"how do VSCode and MCP each embed the Rust scanner in-process" but
"how do VSCode and MCP become drivers that attach to the shared
daemon." The napi-cutover plan optimises for the wrong axis.

## Decision

TSRET-003 and TSRET-004 are **superseded** by a new APS module,
`surface-drivers` (code: **DRVR**). TSRET-005 is retained — the TS
scanner still has to die — but its dependency chain is re-pointed from
"consumers cut over to napi" to "consumers cut over to drivers on the
intercept daemon."

Concretely:

1. VSCode extension becomes an **editor driver**. Its diagnostic path
   connects to the daemon via JSON-RPC 2.0 over Unix domain socket /
   named pipe; violations arrive over the telemetry lane. Nothing in
   the extension imports `@eddacraft/anvil-core/antipattern` or
   `@eddacraft/anvil-checks-native`. Where the LSP standard covers
   the interaction (diagnostics, code actions), the extension uses
   LSP; where it does not (suppression state, gate results, nudge
   metadata), custom JSON-RPC methods layer on top.
2. MCP server becomes an **MCP driver** (fallback class, per the
   driver-framework ADR). Its `check.tool.ts`, `fix.tool.ts`, and
   related surfaces route through the daemon instead of calling the TS
   `GateRunner` in-process. The existing MCP wire contract with agents
   is preserved; the implementation behind it changes.
3. The TSRET-002 residual closeout changes: because the daemon is the
   runtime bundling point, `@eddacraft/anvil-checks-native` does **not**
   need to ship to npm. The crate stays `"private": true`. The napi
   binding is retained only as an internal acceleration path if it
   remains useful for the CLI; otherwise it is a candidate for deletion
   once DRVR lands. OOB install tests and provenance are accordingly
   deferred rather than required.
4. TSRET-005 unchanged in spirit but retargeted: delete the TS scanner
   once no surface imports it. That is driven by DRVR landing, not by
   napi uptake.

The intercept daemon remains on its own roadmap (INTD / INTL / INTR).
DRVR is a *consumer* of the daemon; work items are blocked by the
daemon's stable IPC surface.

## Rationale

The existing TSRET plan solves the duplication problem at the
import-site level. The driver framework solves it at the execution
boundary: no surface imports scanner code; everything talks to the
daemon. The latter is strictly better because:

- The daemon already scans for enforcement purposes. Adding editor /
  MCP read paths is additive, not a parallel engine.
- Every additional editor (Cursor, JetBrains, Zed, Neovim) gets Anvil
  for free if the editor path is LSP-shaped, versus building a
  per-editor napi wrapper.
- MCP's own ADR in `plans/specs/anvil-driver-framework/` explicitly
  downgrades it from foundational to fallback driver; routing it
  through the daemon aligns code with that decision.
- Scan caches, pattern registry, and observability hooks live in one
  place and one process rather than being re-instantiated per
  consumer.
- The napi crate's published-npm distribution story (per-platform
  sub-packages, provenance, install-test matrix) goes away. The CLI /
  daemon binary, already shipped via cargo-dist, is the single
  artefact.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Drivers on intercept daemon (chosen)** | Aligns surfaces with existing INT* and driver-framework plans; every LSP client gets Anvil; MCP positioned per its own ADR | Blocked on INTD v1 stable IPC surface; custom JSON-RPC extensions needed where LSP doesn't cover suppressions/gates |
| TSRET as written (napi in-process) | Smallest diff from current state; unblocks immediately | Two engines on the roadmap doing the same work (napi hot path + intercept daemon); editor reach stays VSCode-only; npm publication stack for a package nobody external consumes |
| Typed napi surface (no JSON) | Faster hot path than JSON-over-IPC, no marshalling | Still one-consumer-at-a-time; does not unify with daemon; locks a Rust↔TS type boundary |
| Keep both engines, back out ADR-026 | Zero migration | Rule changes cost 2x forever; parity harness forever; contradicts ADR-026 explicitly |
| Shell to CLI subprocess from every surface | One binary to test | Subprocess startup latency unacceptable for VSCode save-time diagnostics |

## Consequences

- **Positive:**
  - TSRET-005 (delete TS scanner) becomes trivial once DRVR lands —
    no surface has an import to swap.
  - Editor reach expands to every LSP client at no additional cost.
  - The intercept daemon gains its first non-enforcement consumer,
    exercising the IPC surface in anger before driver-framework v2.
  - `@eddacraft/anvil-checks-native` no longer needs an npm
    publishing story; OOB install tests and provenance decisions can
    be deferred or dropped.
  - Observability hooks (Kindling `gate_evaluated`, `decision_made`)
    have one emission point rather than one per consumer.
- **Negative:**
  - DRVR is blocked on INTD landing a stable IPC surface. Until then,
    VSCode and MCP stay on the TS scanner — a slightly longer
    dual-engine window than the napi cutover would have produced.
  - Custom JSON-RPC methods are required beyond stock LSP (suppression
    state, gate results, nudge metadata). Maintenance surface for
    Anvil, not borrowed from the LSP ecosystem.
  - The MCP server will need a Rust-side helper or an in-process TS
    adapter against the daemon's JSON-RPC. Either is work that the
    napi cutover would have deferred.
- **Risks:**
  - INTD v1 scope excludes editors/MCP deliberately. If INT* slips,
    DRVR slips with it. Mitigation: DRVR work items gated on specific
    INTD deliverables (session register, change stream, violation
    stream) rather than on the whole module.
  - LSP and intercept-loop semantics are different (passive
    diagnostics vs active enforcement). The editor driver must
    surface both without conflating them. Mitigation: module
    explicitly separates "read-only diagnostic mode" from
    "enforcement-participating mode" as distinct driver capabilities.
- **Mitigations:**
  - TSRET-002 residual remains open but with narrowed scope: keep the
    napi crate `"private": true`, skip publish, skip provenance,
    retain the build matrix in CI so the binding doesn't rot.
  - TSRET-005 blocks on DRVR rather than on any particular TSRET-00X
    predecessor, so deletion happens once, cleanly.

## References

- Related ADRs: [ADR-026](./026-rust-scanner-authoritative.md),
  [ADR-015](./015-intercept-loop-enforcement.md)
- Design specs:
  [anvil-driver-framework](../specs/anvil-driver-framework/anvil-driver-framework-adr.md),
  [anvil-driver-framework design](../specs/anvil-driver-framework/anvil-driver-framework-design-spec.md),
  [adf-summary](../specs/adf-summary.md)
- APS modules: **DRVR** (new — supersedes TSRET-003/-004),
  **TSRET** (this ADR adjusts -002 scope and -005 dependencies),
  **INTD / INTL / INTR** (upstream dependencies), **KERN** (graph +
  watcher substrate; Phase 5 daemon-mode work items KERN-050..052
  remain valid and are reused here)
