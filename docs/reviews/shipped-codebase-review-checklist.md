# Shipped Codebase Review Checklist

| Type  | Authority | Owner | Status | Freshness                                                                                                                                                                                             |
| ----- | --------- | ----- | ------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Guide | Advisory  | AICON | Live   | Last reviewed 2026-08-21 at `97899b00a`: DOCRB-006 architecture-pointer sweep; CIB-324 Windows update/version honesty reviewed 2026-08-14. Rechecked after the `v0.9.7-beta` overview freshness bump. |

| Upstream                                                                                                                                                                                                                                                                                                 | Downstream                                           |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------- |
| `CONTEXT.md`, [docs/architecture/overview.md](../architecture/overview.md), [docs/architecture/rust-architecture-overview.md](../architecture/rust-architecture-overview.md), [docs/architecture/quality-model.md](../architecture/quality-model.md), `docs/architecture/*-as-built.md`, Cargo workspace | Council / adversarial review sessions, follow-up APS |

This checklist maps the **shipped Anvil product** into reviewable chunks. Use it
as a living tracker: mark sessions, record findings, and link follow-up work. It
is advisory — code, as-builts, ADRs, and tests remain authoritative.

**Product boundary:** the shipped product is the pure-Rust `anvil` binary
(`crates/`, version `0.9.3-beta` in the workspace). TypeScript under `packages/`
and most of `apps/` is docs, API, or residual tooling — review those only when
the scope is whole-product, not engine-only.

**Related reviews:**

- [`cli-command-truth-review.md`](cli-command-truth-review.md) — CLI docs vs
  runtime registry (CLICT)
- [`kindling-performance-and-integration-assessment.md`](kindling-performance-and-integration-assessment.md)

---

## How to use this checklist

1. Pick the next open chunk in **recommended order** (bottom-up along the
   dependency graph).
2. Read the linked as-built / overview sections first.
3. Walk the paths with the **review focus** and **cross-cutting lenses**.
4. Run the **entry validation** commands and record exit status in the session
   log.
5. Record findings (severity + path + one-line impact) and open APS or GitHub
   Issues for follow-up — do not leave deferred markers in code.
6. Tick the chunk status when the session is complete enough to trust later
   chunks that depend on it.

### Status legend

| Mark  | Meaning                                 |
| ----- | --------------------------------------- |
| `[ ]` | Not started                             |
| `[~]` | In progress / partial                   |
| `[x]` | Session complete for this pass          |
| `[!]` | Blocked (note reason under session log) |

### Cross-cutting lenses (every chunk)

Apply these on every session, not only security-focused ones:

1. **Determinism** — same input → same diagnostics, ordering, and hashes.
2. **Warnings over blocks** — default exit behaviour and enforcement modes.
3. **New edges only** — baseline interaction on every check path.
4. **Surface thinness** — domain logic stays in kernel / checks / policy /
   intercept; CLI, TUI, and MCP are adapters.
5. **Agent safety** — MCP and intercept must not become RCE or secret exfil.
6. **Layering** — no upward or sideways crate dependencies.
7. **Scope guard** — features stay inside
   [`docs/vision/anvil-scope-guard.md`](../vision/anvil-scope-guard.md).

### Full primary CLI validation

When a chunk needs broad confidence in the binary:

```bash
cargo test -p eddacraft-anvil --no-fail-fast
```

The `--no-fail-fast` flag is required so an early test-binary failure cannot
hide integration-test failures (`AGENTS.md`).

---

## Recommended order

```text
1 Foundation types/config
2 Kernel + graph
3 Checks + patterns
4 Policy + architecture
5 Intercept daemon
6 CLI quality surfaces (check / gate / watch / …)
7 Activation + first-run
8 MCP shim
9 TUI
10 Evidence, hooks, packaging
11 Auth, insights, plan / dashboard glue
12 Local dashboard
13 Bundled product data
14+ Optional: cloud, web, TS residual
```

P0-only (security / correctness short pass): **1 → 2 → 3 → 5 → 8 → 6a (gate)**.

### Pre-release short pass (`v0.9.4-beta`)

When the active window is honesty / field follow-up (not a full engine
re-audit), compress the map:

| Session | Scope                                                     | Goal                                           |
| ------- | --------------------------------------------------------- | ---------------------------------------------- |
| A       | Diff triage `v0.9.3-beta..origin/main`                    | Bucket install, daemon, MCP, activation, other |
| B       | Chunk **8b** + intercept **5a/5c** (baseline if no delta) | Agent write-path contract                      |
| C       | Chunk **7** skim + gate/audit/version honesty             | Claims match behaviour                         |
| D       | Changelog, flags, cut hygiene                             | Release-shaped residual risk                   |

Do **not** deep-read foundation/kernel/checks unless the post-tag diff touches
them. Record under [Sessions](#sessions).

---

## Progress board

| Chunk | Name                          | Priority | Status | Last session                          | Findings link                                               |
| ----- | ----------------------------- | -------- | ------ | ------------------------------------- | ----------------------------------------------------------- |
| 1     | Foundation contracts          | P0       | [ ]    | —                                     | —                                                           |
| 2     | Kernel: parse, watch, graph   | P0       | [ ]    | —                                     | —                                                           |
| 3     | Checks pipeline + patterns    | P0       | [ ]    | —                                     | —                                                           |
| 4     | Policy engine + architecture  | P0       | [ ]    | —                                     | —                                                           |
| 5     | Intercept daemon              | P0       | [~]    | 2026-08-09 pre-release (5a/5c skim)   | [Session log](#2026-08-09-v094-beta-pre-release-short-pass) |
| 6     | CLI quality surfaces          | P0       | [~]    | 2026-08-09 pre-release (6a honesty)   | [Session log](#2026-08-09-v094-beta-pre-release-short-pass) |
| 7     | Activation and first-run      | P0       | [~]    | 2026-08-09 pre-release (registration) | [Session log](#2026-08-09-v094-beta-pre-release-short-pass) |
| 8     | MCP shim                      | P0       | [~]    | 2026-08-09 pre-release (8b write)     | [Session log](#2026-08-09-v094-beta-pre-release-short-pass) |
| 9     | TUI surfaces                  | P1       | [ ]    | —                                     | —                                                           |
| 10    | Evidence, hooks, packaging    | P1       | [ ]    | —                                     | —                                                           |
| 11    | Auth, insights, plan glue     | P1       | [~]    | 2026-08-09 pre-release (auth gate)    | [Session log](#2026-08-09-v094-beta-pre-release-short-pass) |
| 12    | Local dashboard               | P1       | [ ]    | —                                     | —                                                           |
| 13    | Bundled product data          | P0       | [ ]    | —                                     | —                                                           |
| 14    | Cloud API (optional)          | P2       | [ ]    | —                                     | —                                                           |
| 15    | Web / docs apps (optional)    | P2       | [ ]    | —                                     | —                                                           |
| 16    | TS residual domain (optional) | P2       | [ ]    | —                                     | —                                                           |
| 17    | APS + adapters (optional)     | P3       | [ ]    | —                                     | —                                                           |
| 18    | Memory stack (optional)       | P2       | [ ]    | —                                     | —                                                           |
| 19    | Driver client (optional)      | P3       | [ ]    | —                                     | —                                                           |
| 20    | ESLint plugin (optional)      | P3       | [ ]    | —                                     | —                                                           |
| 21    | E2E harness (optional)        | P2       | [ ]    | —                                     | —                                                           |
| 22    | Bench / spike (optional)      | P3       | [ ]    | —                                     | —                                                           |

---

## Chunk checklists

### Chunk 1 — Foundation contracts

| Field               | Value                                                                                                                                                                                  |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Priority            | P0                                                                                                                                                                                     |
| Risk                | High (everything depends on this)                                                                                                                                                      |
| Size                | Medium                                                                                                                                                                                 |
| As-built / overview | [`rust-architecture-overview.md`](../architecture/rust-architecture-overview.md) (crate layout/layering), [`quality-model.md`](../architecture/quality-model.md) (check/gate concepts) |

**Paths**

- [ ] `crates/anvil-kernel-types/`
- [ ] `crates/anvil-config/`
- [ ] `crates/anvil-baseline/`
- [ ] `crates/anvil-rules/`
- [ ] `crates/anvil-observability/`
- [ ] `crates/anvil-rayon-init/`
- [ ] `crates/anvil-sarif/`

**Review focus**

- [ ] Determinism of types, IDs, and severity vocabulary
- [ ] Config defaults and enforcement-mode inputs
- [ ] Baseline semantics (“new edges only”)
- [ ] No upward dependencies from foundation crates
- [ ] SARIF mapping preserves finding identity

**Entry validation**

```bash
cargo test -p eddacraft-anvil-kernel-types --no-fail-fast
cargo test -p eddacraft-anvil-config --no-fail-fast
cargo test -p eddacraft-anvil-baseline --no-fail-fast
cargo test -p eddacraft-anvil-rules --no-fail-fast
cargo test -p eddacraft-anvil-observability --no-fail-fast
cargo test -p eddacraft-anvil-sarif --no-fail-fast
```

**Session notes**

- Date / reviewer:
- Outcome: pass | issues filed | blocked
- Findings:

---

### Chunk 2 — Kernel: parse, watch, graph

| Field    | Value                                                                                              |
| -------- | -------------------------------------------------------------------------------------------------- |
| Priority | P0                                                                                                 |
| Risk     | High                                                                                               |
| Size     | Large                                                                                              |
| As-built | [`kernel-as-built.md`](../architecture/kernel-as-built.md); graph specs under `docs/architecture/` |

**Paths**

- [ ] `crates/anvil-kernel/` (`parser/`, `watcher/`, `watch.rs`, `policy/`,
      `protocol/`, `embedded.rs`)
- [ ] `crates/anvil-grammar-wat/`
- [ ] `crates/anvil-graph-cache/`
- [ ] `crates/anvil-gctx-types/`
- [ ] `crates/anvil-gctx-egress/`

**Sub-slices** (tick when complete)

- [ ] 2a Parser + language grammars
- [ ] 2b Watcher / `watch.rs`
- [ ] 2c Graph build + `anvil-graph-cache`
- [ ] 2d GCTX types / egress + kernel protocol

**Review focus**

- [ ] Parse coverage vs claimed languages
- [ ] Cache invalidation and determinism
- [ ] Watch race safety and resource budgets
- [ ] GCTX bounds (no unbounded dumps to agents)
- [ ] Embedded scan path isolation

**Entry validation**

```bash
cargo test -p eddacraft-anvil-kernel --no-fail-fast
cargo test -p eddacraft-anvil-graph-cache --no-fail-fast
cargo test -p eddacraft-anvil-gctx-types --no-fail-fast
cargo test -p eddacraft-anvil-gctx-egress --no-fail-fast
```

**Session notes**

- Date / reviewer:
- Outcome:
- Findings:

---

### Chunk 3 — Checks pipeline + patterns

| Field    | Value                                                      |
| -------- | ---------------------------------------------------------- |
| Priority | P0                                                         |
| Risk     | High (user-visible findings quality)                       |
| Size     | Large                                                      |
| As-built | [`checks-as-built.md`](../architecture/checks-as-built.md) |

**Paths**

- [ ] `crates/anvil-checks/` (`secret/`, `antipattern/`, `command_safety/`,
      `surface/`, `filter.rs`)
- [ ] `crates/anvil-checks-ast/`
- [ ] `crates/anvil-checks-napi/`
- [ ] `patterns/**` (compiled anti-pattern families)

**Sub-slices**

- [ ] 3a Registry / filter / surface framework
- [ ] 3b Secrets
- [ ] 3c Anti-patterns + `patterns/` content
- [ ] 3d Command safety
- [ ] 3e Infra surfaces (Dockerfile, GHA, shell, env, SQL)

**Review focus**

- [ ] False-positive posture and severity consistency
- [ ] Suppression interaction (file + inline)
- [ ] Pattern ownership vs code (no silent drift)
- [ ] AST vs text-path parity where both exist

**Entry validation**

```bash
cargo test -p eddacraft-anvil-checks --no-fail-fast
cargo test -p eddacraft-anvil-checks-ast --no-fail-fast
# optional NAPI bridge:
cargo test -p eddacraft-anvil-checks-napi --no-fail-fast
```

**Session notes**

- Date / reviewer:
- Outcome:
- Findings:

---

### Chunk 4 — Policy engine + architecture boundaries

| Field        | Value                                                                                                                                                   |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Priority     | P0                                                                                                                                                      |
| Risk         | High                                                                                                                                                    |
| Size         | Medium–large                                                                                                                                            |
| Architecture | Policy-engine and invariant sections in [`rust-architecture-overview.md`](../architecture/rust-architecture-overview.md); policy specs in `docs/specs/` |

**Paths**

- [ ] `crates/anvil-policy-engine/`
- [ ] `crates/anvil-policy/`
- [ ] `crates/anvil-architecture/`
- [ ] `policies/` (Rego + fixtures)

**Review focus**

- [ ] Hybrid deterministic-check + OPA path
- [ ] Exit-code and warn-vs-block policy
- [ ] Determinism of eval input and output
- [ ] Architecture “new edges only” with baseline
- [ ] Starter packs / fixtures match engine contracts

**Entry validation**

```bash
cargo test -p eddacraft-anvil-policy-engine --no-fail-fast
cargo test -p eddacraft-anvil-policy --no-fail-fast
cargo test -p eddacraft-anvil-architecture --no-fail-fast
```

**Session notes**

- Date / reviewer:
- Outcome:
- Findings:

---

### Chunk 5 — Intercept daemon (pre-write path)

| Field    | Value                                                            |
| -------- | ---------------------------------------------------------------- |
| Priority | P0                                                               |
| Risk     | Critical (hot write path)                                        |
| Size     | Very large                                                       |
| As-built | [`intercept-as-built.md`](../architecture/intercept-as-built.md) |

**Paths**

- [ ] `crates/anvil-intercept/`
- [ ] `crates/anvil-intercept-proto/`
- [ ] `crates/anvil-intercept-rules/`
- [ ] `crates/anvil-intercept-win32/`
- [ ] `crates/anvil-intercept-macos/`

**Sub-slices** (strongly recommended to split sessions)

- [ ] 5a IPC + protocol (`ipc.rs`, proto crate)
- [ ] 5b Session registry / workspace admission
- [ ] 5c Enforcement + fences + path safety
- [ ] 5d Save-time / mid-edit / overlay scan
- [ ] 5e Graph warm-start + full scan
- [ ] 5f Auth, rate limits, DoS, egress consent
- [ ] 5g Platform adapters (win32 / macos)

**Review focus**

- [ ] Fail-open vs fail-closed under daemon faults
- [ ] Latency budgets and resource limits
- [ ] Path traversal and multi-workspace isolation
- [ ] Fence-store crash recovery
- [ ] Unregistered / unauthenticated client behaviour

**Entry validation**

```bash
cargo test -p eddacraft-anvil-intercept --no-fail-fast
cargo test -p eddacraft-anvil-intercept-proto --no-fail-fast
cargo test -p eddacraft-anvil-intercept-rules --no-fail-fast
```

**Session notes**

- Date / reviewer:
- Outcome:
- Findings:

---

### Chunk 6 — CLI quality surfaces

| Field    | Value                                                                                                                              |
| -------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| Priority | P0                                                                                                                                 |
| Risk     | High (primary UX + CI)                                                                                                             |
| Size     | Very large                                                                                                                         |
| As-built | [`cli-tui-runner-as-built.md`](../architecture/cli-tui-runner-as-built.md), [`quality-model.md`](../architecture/quality-model.md) |
| Related  | [`cli-command-truth-review.md`](cli-command-truth-review.md) for docs-vs-runtime                                                   |

**Paths**

- [ ] `crates/anvil-cli/src/main.rs` (clap dispatch, global args)
- [ ] `crates/anvil-cli/src/commands/{check,gate,watch,audit,doctor,drift,status,export}.rs`
- [ ] `crates/anvil-cli/src/commands/{baseline,architecture,policy/**,validate}.rs`
- [ ] `crates/anvil-cli/src/{output,services,feature_flags,registration}.rs`

**Sub-slices**

- [ ] 6a Gate (`gate.rs`) — workflow judgement heart
- [ ] 6b Watch (`watch.rs`, `watch_save_time.rs`, `watch_driver.rs`)
- [ ] 6c Check / audit / drift / export
- [ ] 6d Doctor / status
- [ ] 6e Baseline / policy / architecture control plane

**Review focus**

- [ ] Quality-model separation (check vs gate vs audit vs doctor)
- [ ] Exit codes and threshold semantics
- [ ] JSON / plain output stability
- [ ] Scan-scope rules (what honours `.gitignore` and what does not)
- [ ] No domain logic duplicated outside engines

**Entry validation**

```bash
cargo test -p eddacraft-anvil --no-fail-fast
# narrower while iterating (examples):
cargo test -p eddacraft-anvil gate -- --nocapture
cargo test -p eddacraft-anvil watch -- --nocapture
```

**Session notes**

- Date / reviewer:
- Outcome:
- Findings:

---

### Chunk 7 — Activation and first-run

| Field    | Value                                                              |
| -------- | ------------------------------------------------------------------ |
| Priority | P0                                                                 |
| Risk     | High (trust / install surface)                                     |
| Size     | Large                                                              |
| As-built | [`activation-as-built.md`](../architecture/activation-as-built.md) |

**Paths**

- [ ] `crates/anvil-cli/src/activation/**`
- [ ] `crates/anvil-cli/src/commands/{start,welcome,init,wizard,uninstall,ensure}.rs`
- [ ] `crates/anvil-cli/src/commands/{mcp_config,mcp_installer,skill*}.rs`

**Review focus**

- [ ] Least privilege on the machine
- [ ] Agent detection correctness
- [ ] Idempotent install / uninstall / ensure
- [ ] No secret leakage in diagnostics
- [ ] Bare `anvil` ensure vs `anvil start` roles (ADR-114)

**Entry validation**

```bash
cargo test -p eddacraft-anvil activation -- --nocapture
cargo test -p eddacraft-anvil ensure -- --nocapture
cargo test -p eddacraft-anvil start -- --nocapture
```

**Session notes**

- Date / reviewer:
- Outcome:
- Findings:

---

### Chunk 8 — MCP shim (agent-facing API)

| Field    | Value                                                          |
| -------- | -------------------------------------------------------------- |
| Priority | P0                                                             |
| Risk     | Critical (agents call this)                                    |
| Size     | Large                                                          |
| As-built | [`mcp-shim-as-built.md`](../architecture/mcp-shim-as-built.md) |

**Paths**

- [ ] `crates/anvil-cli/src/mcp/**` (protocol, tools, resources)
- [ ] Write gates: `tools/validate_write.rs`, `tools/apply_patch.rs`
- [ ] Graph-context tools: `search_symbols`, `symbol_context`, `find_callers`,
      `find_dependents`, `impact_of_change`, `affected_tests`, `query_boundary`
- [ ] Status / check / gate / suppress tools

**Sub-slices**

- [ ] 8a Protocol / versions / dispatch
- [ ] 8b Write gates + enforcement vocabulary
- [ ] 8c Graph-context tools
- [ ] 8d Resources + schema catalogue

**Review focus**

- [ ] Decision vocabulary: `block` / `warn` / `allow` / `gateUnavailable`
- [ ] Schema stability for agent consumers
- [ ] No unbounded graph dumps
- [ ] Daemon-present vs daemon-absent behaviour
- [ ] Auth / enforcement when daemon is available

**Entry validation**

```bash
cargo test -p eddacraft-anvil mcp -- --nocapture
# resource-budget bench lives in anvil-bench when needed:
# cargo test -p anvil-bench mcp_resource_budget -- --nocapture
```

**Session notes**

- Date / reviewer:
- Outcome:
- Findings:

---

### Chunk 9 — TUI surfaces

| Field    | Value                                                                              |
| -------- | ---------------------------------------------------------------------------------- |
| Priority | P1                                                                                 |
| Risk     | Medium                                                                             |
| Size     | Large                                                                              |
| As-built | [`tui-as-built.md`](../architecture/tui-as-built.md), widgets / tutorial as-builts |

**Paths**

- [ ] `crates/anvil-tui/`
- [ ] `crates/eddacraft-tui/`
- [ ] `crates/anvil-cli/src/tui.rs`

**Review focus**

- [ ] No domain logic in widgets (thin surface rule)
- [ ] Keyboard / failure rendering
- [ ] Snapshot tests remain meaningful
- [ ] Shared `eddacraft-tui` API surface does not leak anvil-private types

**Entry validation**

```bash
cargo test -p eddacraft-anvil-tui --no-fail-fast
cargo test -p eddacraft-tui --no-fail-fast
```

**Session notes**

- Date / reviewer:
- Outcome:
- Findings:

---

### Chunk 10 — Evidence, hooks, packaging, secondary engines

| Field    | Value                                                                                                                                  |
| -------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| Priority | P1                                                                                                                                     |
| Risk     | Medium–high                                                                                                                            |
| Size     | Medium                                                                                                                                 |
| As-built | [`capsule-as-built.md`](../architecture/capsule-as-built.md), [`observability-as-built.md`](../architecture/observability-as-built.md) |

**Paths**

- [ ] `crates/anvil-hook/` + CLI `hook` / `hooks`
- [ ] `crates/anvil-run/`
- [ ] `crates/anvil-witness/`
- [ ] `crates/anvil-l4/`
- [ ] `crates/anvil-capsule/`
- [ ] `crates/anvil-attribution/`
- [ ] CLI: `capsule`, `audit_chain`, `l4_validate`, `licenses`, `update/**`

**Review focus**

- [ ] Hook coexistence with other managers (no footguns)
- [ ] Capsule integrity / portability
- [ ] Witness tamper-evidence
- [ ] Update signature path (`commands/update/signature.rs`)
- [ ] L4 claim honesty vs implementation

**Entry validation**

```bash
cargo test -p eddacraft-anvil-hook --no-fail-fast
cargo test -p eddacraft-anvil-run --no-fail-fast
cargo test -p eddacraft-anvil-witness --no-fail-fast
cargo test -p eddacraft-anvil-l4 --no-fail-fast
cargo test -p eddacraft-anvil-capsule --no-fail-fast
cargo test -p eddacraft-anvil-attribution --no-fail-fast
```

**Session notes**

- Date / reviewer:
- Outcome:
- Findings:

---

### Chunk 11 — Auth, insights, plan / dashboard glue in CLI

| Field    | Value  |
| -------- | ------ |
| Priority | P1     |
| Risk     | Medium |
| Size     | Medium |

**Paths**

- [ ] `crates/anvil-cli/src/auth/**`
- [ ] `crates/anvil-cli/src/insights/**`
- [ ] `crates/anvil-cli/src/commands/{auth,insights,plan,dashboard,admin,kindling,edda,ember,gctx}.rs`
- [ ] `crates/anvil-plan-read-model/`
- [ ] `crates/anvil-cli/src/{kindling_runtime,telemetry,graph_base_producer}.rs`

**Review focus**

- [ ] Credential storage and device-flow safety
- [ ] Telemetry opt-in defaults
- [ ] Plan-read-model purity (no I/O side effects in pure model)
- [ ] Admin authority boundaries

**Entry validation**

```bash
cargo test -p eddacraft-anvil-plan-read-model --no-fail-fast
cargo test -p eddacraft-anvil auth -- --nocapture
cargo test -p eddacraft-anvil insights -- --nocapture
```

**Session notes**

- Date / reviewer:
- Outcome:
- Findings:

---

### Chunk 12 — Local dashboard (loopback product surface)

| Field    | Value                                 |
| -------- | ------------------------------------- |
| Priority | P1                                    |
| Risk     | Medium                                |
| Size     | Medium                                |
| Notes    | ADR-104: loopback-only, read-only API |

**Paths**

- [ ] `crates/anvil-dashboard-server/`
- [ ] `apps/dashboard/`

**Review focus**

- [ ] Bind-to-localhost only (no remote exposure)
- [ ] Read-only posture enforced
- [ ] Generated-client seam stays thin
- [ ] No expansion of attack surface beyond local evidence

**Entry validation**

```bash
cargo test -p eddacraft-anvil-dashboard-server --no-fail-fast
# frontend (from apps/dashboard or workspace scripts as appropriate):
# pnpm --filter <dashboard-package> test
```

**Session notes**

- Date / reviewer:
- Outcome:
- Findings:

---

### Chunk 13 — Bundled product data

| Field    | Value                                |
| -------- | ------------------------------------ |
| Priority | P0                                   |
| Risk     | Medium (ships with binary behaviour) |
| Size     | Small–medium                         |

**Paths**

- [ ] `patterns/**`
- [ ] `policies/**`
- [ ] `flags/**` (FLAGCAT — `flags/manifest.json` is the catalogue)
- [ ] `schemas/**`
- [ ] `attribution/**`

**Review focus**

- [ ] Flag defaults for beta / preview gates
- [ ] Pattern false-positive risk
- [ ] Licence allow-list drift
- [ ] Schema versioning (status / insights / workflow events)

**Entry validation**

```bash
# No single crate owns all data; spot-check loaders via engine tests:
cargo test -p eddacraft-anvil-checks --no-fail-fast
cargo test -p eddacraft-anvil-policy-engine --no-fail-fast
# Feature-flag governance guide when changing flags:
# docs/guides/feature-flag-governance.md
```

**Session notes**

- Date / reviewer:
- Outcome:
- Findings:

---

## Optional chunks (not the engine)

Use when the review scope is whole-product rather than the shipped binary.

### Chunk 14 — Cloud API (optional, P2)

- [ ] `apps/anvil-api/`
- [ ] `apps/admin-cli/`
- As-built: [`api-as-built.md`](../architecture/api-as-built.md),
  [`auth-as-built.md`](../architecture/auth-as-built.md)

### Chunk 15 — Web / docs apps (optional, P2)

- [ ] `apps/website/`
- [ ] `apps/docs-public/`, `apps/docs-shell/`, `apps/docs-site/`,
      `apps/docs-public-astro/`
- [ ] `apps/anvil-docs-private/`

### Chunk 16 — TS residual domain (optional, P2)

- [ ] `packages/anvil/{contracts,ports,core,runtime,policy,flags-catalogue,observability}/`
- Notes: still used by API / archive consumers; not the primary runtime.

### Chunk 17 — APS + adapters (optional, P3)

- [ ] `packages/aps/`
- [ ] `packages/adapters/`

### Chunk 18 — Memory stack (optional, P2)

- [ ] `packages/edda-stack/`
- [ ] `packages/kindling-integration/`
- [ ] CLI kindling / edda / ember commands
- Related:
  [`kindling-performance-and-integration-assessment.md`](kindling-performance-and-integration-assessment.md)

### Chunk 19 — Driver client (optional, P3)

- [ ] `packages/anvil-driver-client/`

### Chunk 20 — ESLint plugin (optional, P3)

- [ ] `packages/eslint-plugin-anvil/`

### Chunk 21 — E2E harness (optional, P2)

- [ ] `apps/e2e/`
- Best run **after** chunks 6–8 so surface contracts are already trusted.

### Chunk 22 — Bench / spike (optional, P3)

- [ ] `crates/anvil-bench/`
- [ ] `crates/spike/`
- [ ] `benchmarks/`

---

## Explicitly out of product review

Do not spend product-review time here unless the session is process or docs:

- `plans/`, agent harness config (`.claude/`, skills)
- `docs/` prose (except as-built accuracy while reviewing code)
- `node_modules/`, `target/`, `coverage/`, generated indexes
- Archived Node MCP / VS Code surfaces (not live runtime)

---

## Session log template

Copy under each session (or into a dated subsection below).

```markdown
### YYYY-MM-DD — Chunk N (title)

- Reviewer:
- Scope (sub-slices):
- Commands run (exit codes):
- Findings (severity / path / impact):
  1.
- Follow-up (APS item or issue):
- Chunk board update: [ ] → [x] / [~] / [!]
```

## Sessions

### 2026-08-09 — v0.9.4-beta pre-release short pass

- **Reviewer:** agent (Grok Build), operator-requested
- **Scope:** Sessions A–C of the pre-release short pass (not full P0 map). Diff
  base: `v0.9.3-beta` (`cfe0857cb`) → `origin/main` at pass time.
- **Outcome:** no cut-blocking product defects in the post-tag delta; residual
  risk is APS/release-plan bookkeeping lag and baseline write-path vocabulary
  nuance (documented below).

#### A — Diff triage

| Bucket                         | Post-tag product code? | Notes                                                                 |
| ------------------------------ | ---------------------- | --------------------------------------------------------------------- |
| Install / version              | **Yes**                | `version.rs`, `update.rs` — CIB-315 receipt + platform upgrade advice |
| Daemon membership / activation | **Yes**                | `registration.rs` — wait for durable membership after register ack    |
| MCP write path                 | No                     | Zero commits under `crates/anvil-cli/src/mcp/`                        |
| Intercept                      | No                     | Zero commits under `crates/anvil-intercept*`                          |
| Audit / gate                   | No (product)           | CIB-281 TUI/SARIF scope already **in** `v0.9.3-beta`                  |
| Docs / CI / plans              | Yes                    | Majority of the 19 commits                                            |

**Stat (product crates only):** 3 files, +467 / −115 in `anvil-cli` (`version`,
`update`, `registration`). Full tree: 32 files, mostly docs/CI.

#### B — Write path (Chunk 8b + intercept 5a/5c baseline)

- `validate_write` accepts workspace-relative or absolute-in-root paths;
  absolute paths are aligned via `align_absolute_path_to_canonical_root` before
  `strip_prefix` (symlink / Windows spelling).
- MCP enforcement default is `Interrupt` (`MCP_DEFAULT_ENFORCEMENT`); vetoes use
  `ControlDecision::is_veto` (covers `block` / `fence` / `interrupt`).
- Daemon operational failure: hard stop with `decision: block` and
  `daemonStatus: unavailable` (not warn-through).
- Auth-missing path uses `decision: gateUnavailable` with
  `safeDefault: allow-with-warning` (documented in server instructions).
- Intercept pipeline: pure `Allow` / `Interrupt` evaluation; callers own
  delivery (thin control plane).

**Commands (all exit 0):**

```text
cargo test -p eddacraft-anvil --bin anvil membership_ -- --nocapture
  → 9 passed
cargo test -p eddacraft-anvil --bin anvil upgrade_command -- --nocapture
  → 1 passed
cargo test -p eddacraft-anvil --bin anvil install_method -- --nocapture
  → 4 passed
cargo test -p eddacraft-anvil --bin anvil dist_receipt -- --nocapture
  → 2 passed
cargo test -p eddacraft-anvil --bin anvil repeated_status_read -- --nocapture
  → 1 passed
cargo test -p eddacraft-anvil --bin anvil validate_write -- --nocapture
  → 79 passed
cargo test -p eddacraft-anvil -- version  (integration filters)
  → version_offline + usage_observation ok
```

#### C — Honesty surfaces

- **version / update:** shared `load_dist_receipt` via axoupdater (current +
  legacy app name); Windows upgrade string is one constant shared with `update`
  — unrepresentable divergence.
- **registration:** `await_membership_snapshot` polls up to 2s / 50ms; healthy
  path is one read; absent membership still refuses (CIB-252 honesty preserved).
- **audit security scope:** present on plain, JSON, TUI, SARIF in tree;
  `47e0ae766` is an ancestor of `v0.9.3-beta` (already shipped).
- **Changelog Unreleased:** matches CIB-315 user-visible claims.

#### Findings

| Sev           | Finding                                                                                                                                | Impact                                                                                       | Follow-up                                                                                           |
| ------------- | -------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| Med (process) | **CIB-315** implementation is on `main` with unit tests, but the CIB module item still reads **Ready**                                 | Operators may re-implement or mis-scope 0.9.4 claim                                          | Bookkeeping-branch CIB reconcile only (`plans/project-context.md` multi-writer rule)                |
| Low (process) | **RELEASE-PLAN** still lists **CIB-281** as a 0.9.4 candidate; fix is already in `v0.9.3-beta`                                         | Inflates provisional claim list                                                              | Drop from candidate table when claim locks                                                          |
| Low (observe) | Agent-facing MCP instructions still lead with `block` / `warn` / `allow` / `gateUnavailable`; runtime also emits `interrupt` / `fence` | Agents that match only the string `block` and ignore `isError` / `is_veto` could under-react | Not a post-tag regression; prefer documenting `is_veto` / `isError` in skill copy when next touched |
| Info          | Membership confirm budget is **2s**                                                                                                    | Under extreme daemon lag, honest refusal still possible after wait                           | Acceptable honesty trade-off; warn log records reads/wait                                           |

#### Cut recommendation (provisional 0.9.4)

- Post-tag **product delta is shippable** from this pass’s perspective (install
  honesty + durable-membership wait).
- Before claim lock: reconcile CIB statuses (315 done-on-main; 281 already
  shipped) on a bookkeeping branch; re-run Cross matrix / readiness only when
  claim freezes.
- No need for a full 22-chunk engine review for this window unless field signal
  expands scope.

- **Chunk board update:** 5, 6, 7, 8, 11 → `[~]` (partial pre-release pass)

---

## Suggested multi-session schedule

Assuming roughly half-day to one-day sessions:

| Session | Chunks               | Goal                                    |
| ------- | -------------------- | --------------------------------------- |
| 1       | 1 + 2 skeleton       | Contract and parse/graph trust          |
| 2       | 2 remainder + 3      | Findings quality                        |
| 3       | 4                    | Policy and architecture                 |
| 4–5     | 5 (split sub-slices) | Pre-write safety                        |
| 6       | 6a–6c                | Gate / watch / check                    |
| 7       | 6d–6e + 7            | Ops surfaces + activation               |
| 8       | 8                    | MCP agent contract                      |
| 9       | 9                    | TUI thinness                            |
| 10      | 10–13                | Packaging, evidence, bundled data       |
| 11+     | 12, 14–15…           | Local dashboard and cloud (if in scope) |

---

## Freshness maintenance

When crates, commands, or as-builts move:

1. Update the **Paths** lists and Cargo package names in this file.
2. Bump the metadata **Freshness** date and anchor sources.
3. Prefer linking as-builts over restating implementation detail here.
4. Archive this review to `docs/archive/reviews/` only when the pass is done and
   follow-ups are tracked elsewhere (`docs/reviews/README.md`).
