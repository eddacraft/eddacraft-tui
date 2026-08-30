# anvil-checks

| Type   | Authority     | Owner | Status | Freshness                                                                                                                                        |
| ------ | ------------- | ----- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| README | Authoritative | SCAN  | Live   | Last reviewed 2026-08-30 against CONF-011 range-level PR declaration evaluation, ADR-134, `src/conformance/**`, its tests, and `ARCHITECTURE.md` |

| Upstream                                                  | Downstream                                              |
| --------------------------------------------------------- | ------------------------------------------------------- |
| `src/**`, compiled pattern registry, ADR-029, and ADR-087 | CLI, intercept rules, activation, MCP, and contributors |

Reusable quality checks for performance-sensitive evaluation across anvil's CLI,
kernel, and interception surfaces. Read the source-linked
[local architecture](ARCHITECTURE.md) before changing family boundaries,
suppression, finding shapes, or guarded-byte evaluation. The former central
[checks as-built](../../docs/architecture/checks-as-built.md) is a dated
compatibility and history record.

## Modules

- **`secret`** — secret/credential detection in source files, with a
  `max_line_bytes` ReDoS guard (default 4096 bytes) that skips oversized lines
  before regex evaluation. A skipped line is fail-closed: it blocks a clean pass
  and zeroes the score rather than merely reporting a count, because the guard
  runs before both the pattern and entropy passes and a line that was never read
  cannot be proven clean. Detection is measured against a committed calibration
  corpus (`pnpm secret:calibrate`), which reports detection rate, false-positive
  rate and per-rule misses and fails on drift in either direction.
- **`antipattern`** — registry-backed anti-pattern detection (unsafe code
  patterns, known bad practices). Every shipped rule flows through the compiled
  `.anvil` registry at `patterns/compiled/registry.json`, and rule provenance is
  attached to every finding.
- **`reasoning`** — AI-001 reasoning-category rule that flags appeal-to-
  authority comments at info severity, scoped to comment regions and honouring
  `// @anvil-ignore AI-001 -- <reason>`.
- **`surface`** — `.env`, `.env.*`, and `.envrc` parsing (SURFENV-001) that
  routes values through the existing secret patterns and reports findings with
  the variable name and source line; suppress with
  `# @anvil-ignore SURFENV-001`, plus SQL, Dockerfile, GitHub Actions, and shell
  source-specific checks.
- **`command_safety`** — shell command safety analysis.
- **`conformance`** — bounded, replacement-disabled Tier-0 Git extraction,
  deterministic weak-grade PR-body declaration extraction, and advisory
  claim-versus-effect evaluation. Its range-level entry point aggregates every
  selected commit for production callers such as `anvil conformance check`. That
  PR footprint is claim-agnostic and does not require Conventional Commit
  syntax; Conventional Commit claim extraction remains a separate entry point.
  An empty, partial, or identity-mismatched range remains not evaluated. Opaque
  repository/worktree identities and structured budget diagnostics keep failures
  observable without leaking local paths. Raw Git records stay distinct from
  canonical per-path coverage and evaluator-owned evidence dispositions.

## Intent-conformance claim table v1

The closed v1 table recognises:

- Conventional Commit type `docs` as `documentation-only`: paths under `docs/`
  or `plans/`, documentation extensions (`.md`, `.mdx`, `.rst`, `.adoc`), and
  the documented root/basename files in `evaluate.rs`;
- type `test` as `test-only`: conventional test directories and the exact
  filename suffixes in `evaluate.rs`;
- an explicit `path:<prefix>` scope, or a versioned
  `intent_conformance.scope_mappings` entry loaded only from the evaluated base
  tree.

A PR description may carry exactly one fenced declaration:

````markdown
```anvil-claims
claim: documentation-only
claim: test-only
claim: no-behaviour-change
claim: refactor-only
scope: path:crates/anvil-checks
```
````

The member vocabulary and spacing are exact. Members use canonical kind/value
ordering and are deduplicated after extraction. Only a top-level fence counts;
examples inside another Markdown fence or HTML comment are ignored. A missing,
empty, unclosed, or second block, an unknown claim, or a malformed/invalid-scope
member yields stable `claim.pr-body.*` reasons and a not-evaluated source while
preserving any known members. Provenance is weak-grade `PullRequest` intent and
retains only the caller-supplied immutable source reference plus a SHA-256
digest over the exact raw body bytes; the body itself is never retained. Git may
evaluate the documentation, test, and explicit-path claims. The two
graph-semantic claims remain not evaluated under the existing claim table until
complete graduated CEG evidence is admitted.

Extraction-limit version 1 admits at most 256 KiB of body, a 4 KiB source
reference, 256 non-empty members, 1 KiB per member, 128 distinct scopes, and 512
bytes per scope. Budget failures are reason-coded and cannot release contract
parts. An over-limit body is neither parsed nor hashed; its source uses the
explicit `sha256:unavailable-body-over-limit` sentinel. An over-limit reference
is not retained.

The v1 mapping shape is:

```yaml
intent_conformance:
  scope_mappings:
    schema_version: 1
    mappings:
      core:
        - crates/core
        - src/core
```

Mapping keys are non-empty and each prefix array is non-empty, byte-sorted,
unique, and free of glob syntax, traversal segments, empty segments, and
absolute paths. All prefixes in a selected mapping form one authorised union.
Unknown `scope_mappings` keys are rejected. The selected base-tree source path
and canonical full-config digest travel with the authority, and every
contributing base configuration path is marked `policy-change` independently of
the claim's scope form.

Git path classification is byte- and case-exact. Renames retain one raw record
while both old and new endpoints receive independent coverage dispositions.
Legal repository-relative UTF-8 filenames are not restricted by the stricter
scope-prefix grammar. The bounded no-renames preflight and rename-enabled final
diff must produce identical endpoint sets. Budget failures preserve the
originating stage and expose configured limits, elapsed time, counts through the
last complete record of that stage, and a raw-input digest where available. Git
is resolved once to a canonical absolute executable; relative or empty `PATH`
entries cannot change the invoked program after the working directory changes.
Every invocation runs in an isolated process tree, which is terminated as a unit
on timeout or output overflow, and pipe capture has its own bounded shutdown.
Repository subdirectories resolve to the same canonical Git top-level identity,
while bare repositories are not evaluated.

Free-form scopes never gain path meaning by similarity. Graph bindings are
checked across repository, canonical worktree, run, path, revision/blob, schema,
and generation before semantic evaluation. Missing, stale, and mismatched
bindings remain visible. Graph-semantic claims have no v1 predicate and remain
reason-coded `not-evaluated` even when the binding is valid; only a future
predicate-specific adapter over an actual, revision-bound GV2 delta can change
that. Findings use canonical policy diagnostics at warning severity, leaving
process-exit policy to callers.

## Parallel Scanning

`gate`, `audit`, `check`, `drift`, policy, architecture validation, and the
watcher all share the gitignore-aware discovery walk plus the rayon scan
pattern. First-run scans honour the `ANVIL_SCAN_THREADS` environment variable
and default to `min(num_cpus, 4)` so the parallel walk does not starve TUI or
editor work. Raise the cap on dedicated CI runners; lower it on shared laptops
if you see contention.

## Benchmarks

```bash
cargo bench -p eddacraft-anvil-checks
```

Benchmarks live in `benches/checks.rs`.

## Part of

[eddacraft anvil](../../README.md) monorepo (`crates/anvil-checks`).
