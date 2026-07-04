# ADR-099: moon (moonrepo) build-system migration — defer to a go/no-go spike

> **Renumbered (2026-07-04):** originally authored as ADR-093; the number was
> claimed concurrently by
> [ADR-093 — lang-tail-wave-2 (WASM text + Zig re-entry)](./093-tail-wave-2-wasm-text-and-zig-reentry.md),
> which merged to `main` first. Renumbered to the next free slot per the
> ADR-process race rule; content unchanged.

## Status

Proposed

## Date

2026-06-28

## Context

Anvil is a polyglot monorepo with two task runners in play side by side:

- **nx 22.7.5 + pnpm 11.9** orchestrate the JS/TS surface — 19 `project.json`
  projects across 31 pnpm workspace packages (`pnpm-workspace.yaml`).
- **cargo** builds the Rust surface — a 34-crate workspace under `crates/`
  (root `Cargo.toml`).

The nx layer is not a thin wrapper; it is a load-bearing, invested-in system:

- Task graph and cross-language `^build` edges via `nx.json` `targetDefaults`
  (see ADR-049 for the JS↔Rust seam).
- Input hashing via `nx.json` `namedInputs` (`default` / `production` /
  `sharedGlobals`, the last of which invalidates on `ci.yml`, `rust.yml`,
  lockfiles, and toolchain files).
- Target **inference** via plugins (`@nx/js/typescript`, `@nx/eslint`,
  `@nx/vite`) — most TS/lint/test targets are never written by hand.
- A **custom in-house Rust plugin**, `@eddacraft/nxrust`, that teaches nx to
  see cargo crates, run `build`/`test`/`check`/`clippy`/`fmt-check`, and emit
  Rust-to-Rust graph edges (ADR-021; extended for `CARGO_TARGET_DIR` awareness
  in ADR-057).
- **Custom code generators**, `@eddacraft/anvil-generators`
  (`nx g …:package/command/anvil-package`).
- An **Azure Blob remote cache** (`@nx/azure-cache`, container `nx-cache` on
  `stiacstateprod`) plus nx Cloud analytics and the nx MCP server.
- `nx release` versioning wiring in `nx.json` and per-project `project.json`
  `release` blocks.
- ~12 `nx` invocations in root `package.json` scripts, plus references in
  `scripts/release/preflight.sh`, `scripts/ci/fast-pr-validation.test.sh`, the
  Husky/lint-staged chain, and four CI workflows (`ci.yml` — 45 KB — `rust.yml`,
  `rust-tests.yml`, `ci-nightly.yml`).

[moon](https://moonrepo.dev/docs) ([moonrepo/moon](https://github.com/moonrepo/moon))
is a Rust-based task runner and build system with **first-class Rust toolchain
support**, integrated tool-version management (via proto), smart hashing, and
affected-task detection. Its native Rust support is the live attraction: it is
the off-the-shelf capability that ADR-021 had to build in-house because the
only Rust-aware nx plugin at the time (`@monodon/rust`) shipped with no LICENSE.
If moon's native support is mature enough, it could retire `@eddacraft/nxrust`
and unify both languages under one runner.

A decision needs recording now because the question has been raised explicitly
and a migration would be a multi-week, multi-PR effort touching CI, the remote
cache, and developer toolchains — exactly the class of change that is hard to
reverse once started, and exactly the class ADR-002 ("warnings over blocks")
and the repo's "deterministic, reversible" posture say to de-risk before
committing. This ADR captures the full nx→moon mapping and chooses a path so
the rationale is durable, rather than letting the migration begin (or be
rejected) implicitly.

## Decision

**Do not migrate to moon now. Defer adoption to a time-boxed go/no-go spike**,
mirroring the pattern ADR-057 used for the nx-cache/sccache question
(deferred to a bounded spike rather than committed or rejected outright).

Concretely:

1. **Spike scope — Rust crates only.** Stand up a `.moon/` workspace
   (`.moon/workspace.yml`, `.moon/toolchain.yml`, `.moon/tasks.yml`) plus
   `moon.yml` files for a representative slice of `crates/` — including the
   cross-language NAPI seam crate `crates/anvil-checks-napi` (ADR-049). Rust is
   where moon adds the most (native toolchain) and risks the least (it does not
   yet touch the inferred TS pipeline). Run moon and nx **in parallel** on that
   slice; do not remove any nx wiring during the spike.

2. **Measure against explicit go/no-go criteria.** A **go** requires all of:
   - **Cache parity or better.** moon's local + remote cache hit-rate on the
     Rust slice is within range of the current Azure cache, *and* a remote-cache
     story exists that we accept (moonbase SaaS **or** a self-hosted
     bazel-remote-compatible gRPC endpoint). sccache for Rust is orthogonal and
     stays either way.
   - **Toolchain coexistence.** moon (via proto) coexists with — or cleanly
     replaces — the existing `.node-version` / `.nvmrc` / `rust-toolchain.toml`
     / `.envrc` (direnv) / `package.json` engines stack and the ADR-057
     `CARGO_TARGET_DIR` relocation, without regressing the concurrent-agent box.
   - **Cross-language seam.** The ADR-049 JS↔Rust `^build`/`test` contract is
     expressible as explicit moon cross-project deps without re-introducing the
     40-minute `target/`-lock cascade PR #1729 fixed.
   - **Generator path.** A credible plan exists for `@eddacraft/anvil-generators`
     — either ported to moon codegen templates or kept on a residual nx install
     — with the cost named.

3. **Write the go/no-go as an amendment to this ADR** (Accepted or Rejected),
   and only then open an APS migration module if **go**.

If the spike is **go**, the migration proceeds **phased and reversible**:
project-by-project, nx and moon coexisting, nx generators retained until moon
templates are ready, with a named remote-cache decision (moonbase vs
self-hosted). changesets (JS) and cargo-dist (`dist-workspace.toml`) are
task-runner-independent and **stay**; only `nx release` versioning and the
`preVersionCommand` need replacement.

This ADR does **not** authorise removing nx, `@eddacraft/nxrust`, or the Azure
cache. It authorises the spike and records the decision frame.

## Rationale

The nx→moon concept mapping, and where each piece lands, is the substance of
the decision:

| Capability | Today (nx) | Under moon | Migration cost |
|------------|-----------|------------|----------------|
| Task graph / `dependsOn` / `^build` | `nx.json` `targetDefaults` | `deps` / `dependsOn` in `moon.yml` + `.moon/tasks.yml` | Re-model; mechanical |
| Input hashing | `namedInputs` (`default`/`production`/`sharedGlobals`) | `fileGroups` + task `inputs` | Re-model; `sharedGlobals` self-referential CI inputs need care |
| Target inference | plugins infer TS/lint/vite/test | **none — moon has no plugin inference** | Hand-author ~50 `moon.yml` files; the bulk of the work |
| Rust integration | in-house `@eddacraft/nxrust` (ADR-021) | **native Rust toolchain** | Net **win** — retires a maintained in-house plugin |
| Code generators | `@eddacraft/anvil-generators` | moon codegen templates (weaker) | Port or keep nx for generators only |
| Remote cache | `@nx/azure-cache` (Azure Blob) | moonbase (paid) or self-hosted gRPC | **Highest-risk** infra item |
| Versioning/publish | `nx release` + per-project blocks | not in moon | Keep changesets + cargo-dist |
| Affected detection | `nx affected` | `moon ci` / `moon run --affected` | Re-express in 4 CI workflows |
| Analytics / MCP | nx Cloud + nx MCP | lost | Accept or replace |

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Defer to a Rust-only go/no-go spike (chosen)** | Reversible; isolates the highest-value, lowest-risk surface; proves the cache + toolchain + seam stories before any irreversible work; mirrors ADR-057's accepted spike pattern; captures the full mapping durably | The question stays formally open until the spike reports; some upfront spike cost |
| Full migration now (big-bang nx → moon) | One cutover; ends the dual-runner state sooner | Hand-authoring ~50 `moon.yml` with no inference, replacing the Azure cache, and porting generators **all at once**; high blast radius across CI; hard to reverse; violates the reversible-change posture |
| Reject moon outright; stay on nx + `@eddacraft/nxrust` | Zero work; preserves ADR-021/049/057 investment intact | Forfeits moon's native Rust support (the thing ADR-021 built by hand) without measuring it; leaves the question to resurface unanswered |
| Adopt moon for **new** projects only, freeze nx | Incremental; no migration of existing wiring | Two runners indefinitely; contributors learn both; cross-runner affected/cache never unifies — strictly worse than today's single-graph-per-language split |

The chosen path trades "answer the question immediately" for "answer it with
evidence on the surface where the answer matters most." moon's headline
benefit (native Rust) is precisely what the Rust-only spike measures, so the
spike is maximally informative per unit of risk. It also keeps faith with the
repo's reversible-change principle and reuses an already-accepted decision
shape (ADR-057's deferred spike), so it introduces no new governance
precedent.

This ADR **does not supersede** ADR-021, ADR-049, or ADR-057 — those remain in
effect. A **go** result would supersede ADR-021 (native Rust support replacing
the in-house plugin) and re-home the ADR-049 cross-language contract into moon
deps; that supersession is deferred to the go/no-go amendment, not asserted
here.

## Consequences

- **Positive:**
  - The full nx→moon mapping is recorded once, before any code moves, so the
    eventual go/no-go is an evidence call rather than a rewrite mid-flight.
  - The spike isolates the one capability moon clearly does better (native
    Rust) and the two that carry real risk (remote cache, toolchain), so the
    decision turns on measured facts.
  - No existing wiring is removed; nx, `@eddacraft/nxrust`, and the Azure cache
    keep working throughout — a no-go costs only the spike.
  - changesets + cargo-dist are confirmed task-runner-independent, shrinking
    the true migration surface.

- **Negative:**
  - The migration question stays formally open until the spike reports.
  - The spike itself is non-trivial: authoring a real `.moon/` slice and
    standing up a candidate remote cache to measure parity.

- **Risks:**
  - **Remote cache.** Losing `@nx/azure-cache` with no accepted replacement is
    the migration's biggest single risk; moonbase is paid SaaS and self-hosting
    a gRPC cache is new infra. Gated explicitly in the go criteria.
  - **No inference.** moon requires hand-written `moon.yml` per project; the TS
    surface that nx infers for free becomes ~50 maintained files. Real, but the
    JS/TS surface is contracting under ADR-030/ADR-033.
  - **Toolchain collision.** proto vs the existing `.nvmrc`/`rust-toolchain.toml`/
    direnv/ADR-057 `CARGO_TARGET_DIR` setup could regress the concurrent-agent
    box. Gated in the go criteria.
  - **Generators.** moon's template engine is weaker than nx generators; the
    `@eddacraft/anvil-generators` path is unresolved until the spike names it.

- **Mitigations:**
  - Time-box the spike and run moon **alongside** nx — never replace wiring
    during it.
  - Make remote-cache parity and toolchain coexistence **hard go-gates**, not
    nice-to-haves.
  - Record the go/no-go as an amendment here with the measured numbers, so the
    decision is auditable and the next person inherits evidence, not opinion.
  - Phase any approved migration project-by-project, keeping nx generators
    until moon templates exist.

## References

- Related ADRs:
  - ADR-021 — in-house `@eddacraft/nxrust` plugin (the in-house Rust support
    moon would replace; superseded only on a **go** result)
  - ADR-049 — cross-language `^build` contract (the JS↔Rust seam to re-express
    as moon deps)
  - ADR-057 — dev-environment hardening (the deferred-spike pattern this ADR
    reuses; `CARGO_TARGET_DIR` relocation moon must respect)
  - ADR-020 — versioning strategy (changesets/cargo-dist stay independent of
    the runner)
  - ADR-002 — warnings over blocks (reversible-change posture)
- APS modules: a migration module is opened only on a **go** result.
- Config sites: `nx.json`, `pnpm-workspace.yaml`, root `Cargo.toml`,
  per-project `project.json`, root `package.json` scripts,
  `.github/workflows/{ci,rust,rust-tests,ci-nightly}.yml`.
- External:
  - https://moonrepo.dev/docs
  - https://github.com/moonrepo/moon
