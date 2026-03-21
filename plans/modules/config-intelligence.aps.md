<!--
APS Module: Config Intelligence
====================================
Extracts dependency graphs and project structure from config files.
See: plans/aps-rules.md
-->

# Config Intelligence

| ID     | Owner | Status    |
| ------ | ----- | --------- |
| CFGINT | —     | Placeholder |

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

- `save-time-trust` — runner integration
- `architecture-safety` — edge detector uses extracted dependency graph

**Exposes:**

- Dependency graph (nodes = packages, edges = imports)
- Project structure (workspace members, project references)
- Architecture rule definitions

## Estimated Scope

- **Parsers:** 7-10 config format parsers
- **Effort:** 2-3 weeks

## Tasks

- CFGINT-001: Config parser framework (format detection, streaming parse)
- CFGINT-002: JavaScript/TypeScript ecosystem (package.json, tsconfig, pnpm-workspace)
- CFGINT-003: Rust ecosystem (Cargo.toml workspace)
- CFGINT-004: Go ecosystem (go.mod, go.work)
- CFGINT-005: Dependency graph data model
- CFGINT-006: Architecture rule file parser (.anvil/architecture.yaml)
- CFGINT-007: Tests and documentation
