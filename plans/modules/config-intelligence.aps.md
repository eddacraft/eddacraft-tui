<!--
APS Module: Config Intelligence
====================================
Extracts dependency graphs and project structure from config files.
See: plans/aps-rules.md
-->

# Config Intelligence

| ID     | Owner | Status | Progress |
| ------ | ----- | ------ | -------- |
| CFGINT | —     | Draft  | 0/7      |

**Last reviewed:** 2026-04-26

> **Audit note (2026-04-26):** Tier B (queued). Earlier audit pass flagged
> CFGINT as a possible archive on the basis that both declared dependencies
> (`save-time-trust`, `architecture-safety`) are archived. That was a misread
> — those *planning modules* are archived because their work-item lists
> completed; the live home for architecture rule logic is the
> `crates/anvil-architecture` crate, which is the natural consumer here.
>
> CFGINT is the **explicit handoff target** for config/dependency-graph
> extraction across the 2026-04-08 Language & Coverage design:
> - `lang-rust.aps.md` — "Cargo dependency-graph analysis lives in
>   `config-intelligence`"
> - `lang-python.aps.md` — Python config analysis delegated here
> - `surface-env-files.aps.md` — out-of-scope handoff to CFGINT
> - `plans/specs/2026-04-08-language-and-coverage-design.md:73,885` —
>   names CFGINT as the canonical home
>
> Not blocking RTAI launch. Promotes to Ready when Language & Coverage
> Phase 1 packs (PACKPUL, PACKLLM) need `package.json` parsing or RSTLAN
> needs the Cargo.toml graph.
>
> **Rescope work pending** (tracked separately, see followup list):
> 1. Retarget Interfaces: depends on `crates/anvil-architecture` (live);
>    consumed by RSTLAN, PYLAN, SURFENV, PACKPUL, PACKLLM.
> 2. Decide implementation home — new `crates/anvil-config` crate, or
>    fold into `crates/anvil-architecture/src/config/`.
> 3. Define the graph artefact shape (likely a typed export under
>    `crates/anvil-kernel-types` per SCHEMA module).
> 4. Confirm task list still matches the Phase 1 needs once RSTLAN /
>    PACKPUL move toward Ready.

## Purpose

Extract architecture-relevant signals from configuration files — dependency
graphs, project boundaries, and module structure — without requiring language-
specific analysers. Config files encode more architecture intent than most
teams realise: `package.json` declares module boundaries, `tsconfig.json`
defines project references, `go.mod` exposes the dependency graph.

## In Scope

**Dependency graph extraction:**

- `package.json` — `dependencies`, `devDependencies`, `peerDependencies`
- `Cargo.toml` — `[dependencies]`, `[dev-dependencies]`, workspace members
- `go.mod` — `require` block, `replace` directives
- `pyproject.toml` — `[project.dependencies]`
- `build.gradle.kts` — `dependencies { }` block
- `pubspec.yaml` — `dependencies` block
- `Package.swift` — dependencies array

**Project structure extraction:**

- `tsconfig.json` — `references`, `paths`, `include`/`exclude` (monorepo
  boundaries)
- `pnpm-workspace.yaml` — workspace packages
- `nx.json` — project graph, implicit dependencies
- `Cargo.toml` workspace — `[workspace.members]`
- `go.work` — workspace modules

**Architecture rule files:**

- `.anvil/architecture.yaml` — layer definitions, boundary rules
- `.operc` / `policy/` — OPA policy configuration
- `.anvil.yml` — Anvil configuration

## Out of Scope

- Lock file analysis (use existing dependency check)
- License compliance checking
- Vulnerability scanning from deps
- Transitive dependency resolution

## Interfaces

**Depends on:**

- `crates/anvil-architecture` — live consumer for layer/edge logic; would
  consume the extracted dependency graph
- `crates/anvil-cli` — runner integration surface

**Consumed by (live planning modules):**

- `lang-rust` (RSTLAN) — Cargo dependency-graph analysis
- `lang-python` (PYLAN) — Python config analysis
- `surface-env-files` (SURFENV) — `.env` file handoff
- `pack-pulumi` (PACKPUL), `pack-llm-provider` (PACKLLM) — `package.json`
  parsing for substrate detection

**Exposes:**

- Dependency graph (nodes = packages, edges = imports)
- Project structure (workspace members, project references)
- Architecture rule definitions

## Estimated Scope

- **Parsers:** 7-10 config format parsers
- **Effort:** 2-3 weeks

## Work Items

- CFGINT-001: Config parser framework (format detection, streaming parse)
- CFGINT-002: JavaScript/TypeScript ecosystem (package.json, tsconfig, pnpm-workspace)
- CFGINT-003: Rust ecosystem (Cargo.toml workspace)
- CFGINT-004: Go ecosystem (go.mod, go.work)
- CFGINT-005: Dependency graph data model
- CFGINT-006: Architecture rule file parser (.anvil/architecture.yaml)
- CFGINT-007: Tests and documentation
