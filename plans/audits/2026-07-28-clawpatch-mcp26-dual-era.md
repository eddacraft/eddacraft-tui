# Clawpatch audit — MCP 2026-07-28 dual-era support

Date: 2026-07-28
Branch: `feat/mcp26-dual-era-support`
Scope: dual-era protocol host, activation probe, MCP stdio integration tests

## Coverage

Heuristic map only owns the two new/changed test suites for `--since origin/main`.
A manual clawpatch feature (`feat_library_mcp26_dual_era_protocol`) was registered
to review production dual-era sources under `crates/anvil-cli/src/mcp/protocol/**`,
activation, and `commands/mcp.rs`.

Reviewed features:

| Feature | Role | Result |
| --- | --- | --- |
| `feat_test-suite_69a4b0c2c5` | `mcp_activation_probe` | clean after fix |
| `feat_test-suite_dd97196f08` | `mcp_serve_stdio` | clean after fixes |
| `feat_library_mcp26_dual_era_protocol` | dual-era protocol + probe host | clean after fixes |

## Findings fixed this pass

| ID | Severity | Summary | Fix |
| --- | --- | --- | --- |
| `fnd_sig-feat-test-suite-69a4b0c2c5-7_8defb89fd0` | medium | Timeout path left MCP child unreaped | `KillOnDrop` RAII around child |
| `fnd_sig-feat-test-suite-dd97196f08-8_02d2b54544` | low | graph://stats test only checked outcome presence | assert `status == "unavailable"` |
| `fnd_sig-feat-library-mcp26-dual-era-_541ca478bc` | high | Malformed `_meta` fell through to legacy | modern intent = presence of `_meta` |
| `fnd_sig-feat-library-mcp26-dual-era-_7aeab13eb8` | medium | Legacy tools/resources before initialize | require sealed legacy initialise |
| `fnd_sig-feat-library-mcp26-dual-era-_095cf72ef9` | medium | Object/array JSON-RPC ids accepted | reject with `-32600` / null id |
| `fnd_sig-feat-library-mcp26-dual-era-_279ae549b8` | medium | Missing `jsonrpc: "2.0"` / scalar params | framing validation before dispatch |
| `fnd_sig-feat-library-mcp26-dual-era-_b72a295ee6` | medium | Malformed input could still kill via exit | `is_exit_notification` requires JSON-RPC 2.0 bare exit |
| `fnd_sig-feat-test-suite-dd97196f08-d_c7cb66ab64` | low | Mis-indented `send_legacy_initialize` | formatting |

## Accepted / deferred

| ID | Severity | Disposition |
| --- | --- | --- |
| `fnd_sig-feat-library-mcp26-dual-era-_d7112568d5` | high (risk) | **wont-fix** for this branch — intentional RC pin until MCP26-001 final seal |

## Validation evidence

```text
cargo test -p eddacraft-anvil --bin anvil protocol::   # 34 passed (incl. new framing/init/meta tests)
cargo test -p eddacraft-anvil --test mcp_serve_stdio   # 45 passed
cargo test -p eddacraft-anvil --test mcp_activation_probe  # 1 passed
cargo test -p eddacraft-anvil --bin anvil activation::mcp_client  # 62 passed (subset filter)
```

## Notes

- Full historical clawpatch corpus remains ~900 findings; this audit is scoped to dual-era changes only.
- No PR to main until MCP final seal (MCP26-001/010/011) per branch policy.
