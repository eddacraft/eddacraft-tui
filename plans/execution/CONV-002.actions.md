# CONV-002: Shared source-scan proof

| Type | Authority | Owner | Status | Freshness |
| --- | --- | --- | --- | --- |
| Spec | Advisory | CONV | In Progress | Reviewed 2026-09-05 against the CLI and MCP source-scan paths |

| Upstream | Downstream |
| --- | --- |
| ADR-140; `plans/modules/converged-app-decisions.aps.md` | CLI/MCP shared-command implementation and compatibility fixtures |

## Goal and delivery boundary

Start CONV-002 with the existing source antipattern scan, exercised through
`anvil check` and MCP `anvil_check`. The first increment extracts their shared
regex/AST execution into an internal typed service. It does not close CONV-002:
generated application schemas, identity, compatibility negotiation and semantic
error fixtures still require a subsequent increment.

This is an existing synchronous local scan. It is not the daemon-owned mutation
or approval proof in CONV-003, and supplies no tray lifecycle evidence.
ADR-140's tray-only requirements remain binding for those later proofs.

## Existing-path inventory

| Surface | Current authority and behaviour | Disposition |
| --- | --- | --- |
| CLI `commands/check.rs` | Selects explicit/git-discovered files; resolves enabled checks, extensions, severity, opt-in, config excludes and generated-file filtering; runs regex and AST; retains diagnostics, SARIF tier attribution, output/evidence and numeric exits | Preserve adapter policy; share only the paired source scan |
| MCP `mcp/tools/check.rs` | Validates the server/workspace boundary and contained relative files; uses default antipattern config at error threshold; runs regex and AST; emits existing MCP JSON and `backend: local`, `daemonStatus: not-wired` | Keep validation before the handler and preserve its output |
| MCP registry | `anvil_check` is read-only/open; mutating and execution tools have separate authentication requirements | No permission change or new tool |
| GCTX RPCs | Sealed graph DTOs through existing daemon admission; unavailable/not-ready outcomes and egress budgets | No new RPC; keep with GCTX owners |
| Dashboard plans | Existing bounded plan-read model and generated OpenAPI seam under ADR-104 | Browser remains read-only; no new endpoint |
| Settings | `anvil-settings` owns the revisioned read model and redaction under ADR-132 | No second settings reader/writer |

The two scan adapters are not fully equivalent today. CLI configuration and
file selection exceed MCP's supported inputs; CLI also runs a separate secret
tier. The parity fixture uses the common, explicit-file/default-option subset.
Preserving these differences is required for this extraction, not a declaration
that they are the final application contract.

## Increment 1: Internal shared handler

**Readiness:** Ready for the internal extraction only. No new admission,
concurrency, retry or retention mechanism is introduced; existing scanner and
transport bounds remain unchanged. No default-on converged surface is added.

**Files:**

- `crates/anvil-cli/src/services/antipattern_scan.rs`: paired execution and
  `SourceScanResult` retaining existing `AntipatternCheckResult` and `AstScanOutput`.
- `crates/anvil-cli/src/services/mod.rs`: register the internal service.
- `crates/anvil-cli/src/commands/check.rs`: call it after existing selection.
- `crates/anvil-cli/src/mcp/tools/check.rs`: call it after existing admission.
- `crates/anvil-cli/tests/mcp_serve_stdio.rs`: real CLI/MCP finding parity fixture.
- `crates/anvil-cli/ARCHITECTURE.md`: record the shared implementation seam.

**Acceptance:**

1. Both live adapters call the same typed service; it delegates to existing
   scanners without importing MCP, clap, TUI or transport types.
2. The service preserves full tier results, including AST diagnostics and
   provenance; adapter sorting, redaction and output remain owned in place.
3. A real source fixture proves AST findings and relative locations survive;
   configured regex exclusions still reach the scanner.
4. A subprocess fixture invokes CLI and stdio MCP, confirms matching finding
   identity/severity/location and blocking semantics, and pins the CLI's
   existing exit 0 for an info-only finding.
5. Existing CLI/MCP invalid-input and confinement tests stay green. No AST or
   parser dependency moves into intercept; Cargo manifests and lockfile unchanged.

**Validation:**

```bash
cargo fmt --all -- --check
cargo test -p eddacraft-anvil --bin anvil services::antipattern_scan
cargo test -p eddacraft-anvil --bin anvil mcp::tools::check
cargo test -p eddacraft-anvil --bin anvil commands::check
cargo test -p eddacraft-anvil --test mcp_serve_stdio
cargo test -p eddacraft-anvil --test check_path_rendering
cargo clippy -p eddacraft-anvil --all-targets -- -D warnings
pnpm docs:check
pnpm aps:active-lint
```

If running the full CLI suite, use
`cargo test -p eddacraft-anvil --no-fail-fast` as required by AGENTS.
Council judgement and required PR CI are separate evidence. This environment
has no Rust toolchain: tests were authored before the handler, but no local
red/green execution is claimed. Compilation and executable results must come
from the PR's Rust checks before merge.

## Remaining CONV-002 increments

Before adding an application ingress, make its exact request/result schema and
limits Ready: canonical workspace-relative target and host-established identity,
supported version negotiation, typed semantic errors, generated schema/client
fixtures, explicit supported options and numeric file/byte/output/concurrency
bounds. Inventory existing schema generation before adding a dependency.

The new ingress must not inherit legacy partial-read behaviour as successful
complete coverage: the existing regex disk wrapper skips unreadable/non-UTF-8
files. Test missing files, read failure and incomplete input as explicit outcomes.
Changing existing CLI/MCP behaviour requires a separately reviewed compatibility
disposition, not a silent consequence of this refactor.

Durable admission, retriable mutation, approval consumption, recovery and
daemon/client separation remain CONV-003. CONV-002 is not Done until the owning
module's full shared-contract gates have executable evidence.
