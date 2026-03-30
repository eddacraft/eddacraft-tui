# Agent Guidelines

These conventions apply to all agents working in this project.

## Instruction Precedence

- Start with this file for repo-wide rules.
- If the subtree you are editing has its own `AGENTS.md`, apply the nearest file
  as a refinement of this one.
- When instructions conflict, the more specific `AGENTS.md` for that subtree
  wins.

## Planning - Anvil Plan Spec (APS)

All multi-step work MUST use APS format:

- Master plan: `plans/index.aps.md`
- Modules: `plans/modules/<module>.aps.md`
- Work item IDs: `PREFIX-NNN` (3-digit zero-padded)
- Module statuses: Draft > Proposed > Ready > In Progress > Complete
- Wave-based parallel execution for independent work items
- Archive completed modules to `plans/archive/`

Before starting implementation, check `plans/index.aps.md` for active work items
and current status. Update task status as you progress.

Reference spec: <https://github.com/EddaCraft/anvil-plan-spec>

## Workspace Topology

- `apps/` contains deployable applications and services
- `packages/` contains the TypeScript libraries and shared tooling
- `crates/` contains the Rust CLI, kernel, TUI, and related crates
- `tools/` contains generators, codemods, and local automation
- `docs/` contains product, architecture, and engineering documentation
- `plans/` contains APS modules, execution notes, and decisions
- `archive/` is historical reference material; do not modify it unless the task
  explicitly targets archived code or documents

## Toolchains and Commands

- The JavaScript/TypeScript workspace uses `pnpm` and `Nx`
- The Rust workspace uses `cargo`
- Run commands from the repository root unless a more specific `AGENTS.md` says
  otherwise
- Required toolchains:
  - Node.js `>=22.13.0`
  - `pnpm >=10.20.0`
  - Rust `1.94.0` from `rust-toolchain.toml`
- Prefer root scripts and project targets over ad-hoc commands:
  - `pnpm build`
  - `pnpm test`
  - `pnpm lint` / `pnpm lint:check`
  - `pnpm typecheck`
  - `pnpm exec nx <target> <project>`
  - `pnpm -F <package> <script>`
  - `cargo test -p <crate>`
- If you change Nx project wiring, generators, or workspace configuration, run
  `pnpm exec nx sync` before broader validation

## Verification

- Validate the smallest affected surface first, then widen scope if the change
  crosses package or language boundaries
- For TypeScript/JavaScript changes, use the most specific applicable commands:
  - `pnpm -F <package> test`
  - `pnpm exec nx test <project>`
  - `pnpm exec nx build <project>`
  - `pnpm exec nx typecheck <project>`
- For Rust changes, use:
  - `cargo fmt --check`
  - `cargo test -p <crate>` or `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
- For fresh clones or dependency graph changes, run `pnpm build` before tests if
  cross-package imports need built outputs
- If you skip validation, say so explicitly in the handoff

## Repository Operations - gx

Use `gx` for all repository management. Never use raw `git clone`.

| Task              | Command                  |
| ----------------- | ------------------------ |
| Clone a repo      | `gx clone <url-or-name>` |
| Jump to a project | `gx <name>`              |
| Scaffold configs  | `gx init`                |
| List projects     | `gx list`                |

All cloned repos land in `~/Projects/src/` automatically.

## Commit Format

Conventional commits with imperative mood, lowercase, no period:

```
<type>(<scope>): <subject>
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `chore`, `ci`

Use the narrowest meaningful scope, usually the package, app, crate, or module
being changed.

The `Authored-By:` trailer is added automatically - do not add it manually.

## Scope Constraint

All contributions must align with Anvil's role as a real-time control layer for
safe software creation. Features that do not directly improve prevention,
validation, or enforcement of unsafe outcomes must not be introduced.

See: @docs/vision/anvil-scope-guard.md

## Code Quality

- UK English spelling in plan text and documentation
- Follow the existing architecture and package boundaries instead of creating
  new cross-layer shortcuts
- Clean, modular, functional code following the project's conventions
- TypeScript uses ESM; preserve explicit relative `.js` import extensions where
  the package already requires them
- Prefer Zod-first schemas and export inferred types from schema definitions
- Co-locate tests with source files as `*.test.ts` unless the package already
  uses a different test layout
- Validate at system boundaries; trust internal code
- No secrets in code - use env vars or secret managers
