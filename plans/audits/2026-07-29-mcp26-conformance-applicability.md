# MCP26 conformance applicability and local evidence

| Field | Value |
| --- | --- |
| Type | APS verification evidence |
| Work item | MCP26-010 |
| Date | 2026-07-29 |
| Branch | `feat/mcp26-dual-era-support` |
| Status | Local evidence green; manual platform workflow and actual-client evidence pending before merge |

## Official runner applicability

The pinned modern runner is
`@modelcontextprotocol/conformance@0.2.0-alpha.10`, source commit
`49103de6ed70804e940637bf3e9e29e4a3f54e64`. The selected legacy runner is
`@modelcontextprotocol/conformance@0.1.16`.

Both official server runners accept an HTTP URL and assert Streamable HTTP
behaviour. anvil exposes stdio only and does not advertise HTTP, SSE,
authentication, prompts, Tasks, sampling, elicitation, subscriptions, or
list-change capabilities. Running an HTTP bridge would test the bridge rather
than anvil's transport. The literal official server scenarios are therefore
not applicable; applicable protocol semantics are covered by the repository's
black-box stdio fixtures.

## Applicable protocol evidence

The stdio suite covers:

- discovery without `initialize`;
- per-request version and capability metadata;
- optional `clientInfo`, with string `name` and `version` required when
  present;
- direct modern `tools/list`, `tools/call`, `resources/list`, and
  successful `resources/read`;
- `resultType`, server identity, and private cache fields;
- unsupported-version error `-32022` with requested and supported versions;
- the sealed legacy versions `2025-11-25`, `2025-06-18`, `2025-03-26`,
  and `2024-11-05`;
- malformed, unselected-era, and oversized frame rejection.

The E2E smoke drives modern discovery and direct tool requests, then separately
retains the legacy initialise lifecycle. Both resource-budget benchmark drivers
use modern discovery and per-request metadata; the MCP budget records time to
the first modern response.

## Client and platform boundary

The existing twelve client configuration shapes remain unchanged. Repository
fixtures prove the shared stdio contract, not execution of proprietary GUI
binaries. Linux local evidence is recorded below. Normal feature PRs do not run
the Rust macOS Arm / Windows x64 test matrix; after push, dispatch
`.github/workflows/rust.yml` manually for this branch and record its run before
merge.

## Local evidence

- `cargo test -p eddacraft-anvil --test mcp_serve_stdio` with isolated
  `XDG_RUNTIME_DIR` — 47 passed
- `cargo test -p eddacraft-anvil --bin anvil 'mcp::protocol::'` — 28 passed
- `cargo test -p eddacraft-anvil --test mcp_activation_probe` — 1 passed
- `pnpm --filter @eddacraft/anvil-e2e test:smoke` — 20 passed
- `cargo check -p anvil-bench --benches` and
  `cargo clippy -p anvil-bench --benches -- -D warnings` — passed
- `mcp_resource_budget` — passed; first modern response 6.699 ms,
  99.49% steady CPU and 15.89 MiB peak RSS against 200% / 96 MiB budgets
- `concurrent_processes` — passed; 312.87% steady CPU and 264.29 MiB peak RSS
  against 800% / 700 MiB budgets
- `cargo clippy -p eddacraft-anvil --bin anvil -- -D warnings` — passed
- `pnpm format:check`, `pnpm docs:check`, `pnpm docs:public:check`,
  `pnpm docs:public:commands`, `pnpm aps:active-lint`, and
  `pnpm aps:index:check` — passed
- `git diff --check origin/main` — passed

`pnpm validate:changed` could not initialise Nx's lockfile plugin in this
worktree because `node_modules/.modules.yaml` is absent. Its constituent
MCP26-focused Rust, E2E, formatting, documentation, and APS checks above were
run directly. Required CI remains authoritative for the repository-wide gate.

## Merge gate

Do not advance MCP26-010 beyond In Progress until required PR CI and the
manually dispatched Rust macOS Arm / Windows x64 matrix are green. Actual-client
manual evidence is not represented by config fixtures: record the supported
client/version/result matrix before merge, or obtain an explicit operator
deferral and keep the public claim limited to configuration and stdio contracts.
