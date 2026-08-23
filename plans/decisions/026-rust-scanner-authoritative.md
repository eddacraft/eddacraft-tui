# ADR-026: Rust Scanner is Authoritative; `registry.json` is the Contract

## Status

Accepted

> **Amended (2026-04-29) by [ADR-033](./033-park-ide-mcp-retire-ts-scanner.md).**
> §7 and §8 of the Decision below are amended: the TS scanner's
> retention "for in-process surfaces" and its long-term retirement
> "after napi-rs" are both superseded. The TS scanner is retired now
> under ADR-033; the in-process surfaces are archived under
> `archive/anvil-vscode-extension/` and `archive/anvil-mcp-server/`.
> The authoritative-Rust decision in this ADR is unaffected.
>
> **Amended (2026-08-23) by [ADR-131](./131-registry-override-explicit-only.md).**
> Decision §1's four-tier lookup (explicit path → `ANVIL_REGISTRY_PATH` →
> cwd upward walk → executable-directory upward walk) is superseded.
> Resolution is explicit path → `ANVIL_REGISTRY_PATH` → compile-time
> embedded catalogue. Implicit walks are closed so a cloned
> `patterns/compiled/registry.json` cannot replace the scanner catalogue.
> The compiled registry remains the authoring contract.

## Date

2026-04-21

## Context

Anvil currently has two anti-pattern scanners:

- **TS scanner** (`packages/anvil/core/src/antipattern/`) — consumed by the
  VSCode extension, MCP server, embedded analysis surface, gate-runner,
  constraint-collector, and e2e tests. As of ANVFMT-006..015 this scanner
  reads patterns from the compiled `.anvil` registry at
  `patterns/compiled/registry.json`, supports `ArtifactKind`
  (source / pr-description / commit-message / agent-output), and emits
  `family` / `definition_ref` / `spectrum_position` provenance on warnings.
- **Rust scanner** (`crates/anvil-checks/src/antipattern/`) — the engine
  the Rust CLI binary (`anvil check`) invokes. Contains a hardcoded
  `PATTERN_DEFS: &[PatternDef]` array of 13 patterns (AP-001..AP-013).
  Predates the `.anvil` format. **Does not read the registry.** Still
  ships the retired HTML/CSS patterns (AP-008..AP-013). Has no
  `ArtifactKind`, no family metadata, no RL/DD/GS rules.

This divergence was implicit until ANVFMT-013 (pr-description CLI path)
surfaced it. The result is two facts that cannot both remain true:

1. The CLI (the binary users actually run) fires the stale catalogue.
2. The TS scanner fires the current catalogue but nobody runs it from a
   terminal in production.

The performance envelope matters here. Anti-pattern scanning is intended
to run across tens of concurrent artifacts (multi-PR checks, watch-mode
fan-out, full-repo scans in CI). That throughput target rules out Node
as the primary engine — JS startup cost alone dominates for many small
artifacts, and regex execution in V8 is slower than Rust + `regex` crate
by a factor of several on realistic workloads.

Two decisions are therefore needed together:

1. **Which implementation is authoritative?** (The one the CLI runs.)
2. **What is the boundary the two halves of the system agree on?**
   (Neither is getting deleted today — VSCode and MCP surfaces still
   call the TS scanner in-process.)

## Decision

**The Rust scanner in `crates/anvil-checks` is the authoritative
anti-pattern engine.** The contract between the family/rule author and
the scanner is the compiled registry at
`patterns/compiled/registry.json`, produced by the TypeScript
`scripts/compile-patterns` tool from the `.anvil` source tree.

Concretely:

1. Rust reads `registry.json` at startup via a new `registry_loader`
   module in `anvil-checks`. **Amended by ADR-131:** resolution is
   explicit path → `ANVIL_REGISTRY_PATH` env → compile-time embedded
   catalogue. There is no cwd or executable-directory walk. Validates
   with `serde_json` and a schema type that mirrors
   `CompiledRegistrySchema` from TS.
2. The hardcoded `PATTERN_DEFS` array in `anvil-checks/patterns.rs` is
   deleted. `PATTERNS` becomes a `LazyLock<Vec<AntiPattern>>` built from
   the loaded registry. AP-008..AP-013 drop out automatically because
   they aren't in the registry.
3. Rust `AntiPattern` gains `family`, `definition_ref`,
   `spectrum_position`, and `targets` fields. Rust `Warning` gains the
   same provenance fields. JSON output schemas stay compatible (new
   fields are optional / additive).
4. Rust gains `ArtifactKind` and `scan_artifact(artifact, options)`,
   matching the TS API surface. `scan_file` stays as a wrapper that
   constructs a source artifact.
5. The scan loop uses `rayon::par_iter` over artifacts, enabling the
   parallel throughput the system needs.
6. New CLI command (or flag) wires pr-description scanning:
   `anvil scan --artifact pr-description <file>`. This closes
   ANVFMT-013.
7. TS scanner stays as-is for the in-process surfaces that use it today
   (VSCode extension, MCP server, embedded analysis). Both scanners
   read the *same* `registry.json` artifact, so they cannot drift on
   rule content — only on engine behaviour (regex edge cases, artifact
   filtering). Parity is maintained by a shared fixture-driven test
   suite described in the Consequences section.
8. **Long-term**: the TS scanner is a liability. Once the Rust scanner
   has a stable napi-rs / WASM binding, the IDE/MCP surfaces migrate
   off the TS implementation and it is deleted. That's a separate
   module, not part of this ADR.

## Rationale

The decision falls out of three forcing functions:

- **Throughput.** Tens of parallel scans across large artifact corpora
  is not a Node workload. Rust + `regex` + `rayon` is.
- **Registry contract already exists.** ANVFMT-006..015 made
  `registry.json` the source of truth. The "contract boundary" question
  answers itself — it is the JSON artifact. Rust and TS are just two
  consumers.
- **Deleting drift is cheap.** The Rust hardcoded catalogue is
  ~428 lines that become ~50 lines of registry-consuming code. The
  retired HTML/CSS rules vanish for free.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Rust authoritative, TS coexists short-term, contract is `registry.json`** (chosen) | Solves throughput. Removes hardcoded drift. Preserves VSCode/MCP surfaces. Natural migration path to napi-rs later. | Two engines during the transition. Shared test-fixture layer is required to prove parity. |
| TS authoritative, Rust shells out to Node | Reuses all ANVFMT work directly. | Node startup cost per invocation kills the parallel-scan target. Adds a Node runtime dep for every CLI user. Fails the performance envelope by construction. |
| Keep both, no contract | Minimal immediate work. | Permanent divergence. Users get different warnings from `anvil check` vs. VSCode. Already happening — this is the status quo we're exiting. |
| Full merge via napi-rs now | One engine, one codebase. | Adds native-build toolchain requirement to the TS publish pipeline before we've even got CLI parity. Too much work up-front for the risk; napi-rs is a post-ANVFMT concern. |

## Consequences

- **Positive:**
  - CLI users get the current 18-rule family-based catalogue plus
    pr-description/commit-message artifact scanning.
  - Performance target (tens of parallel artifact scans) becomes
    achievable via `rayon`.
  - Rule authors have exactly one contract to maintain: the `.anvil`
    source → `registry.json` pipeline. Neither scanner is edited when
    a rule changes.
  - ~400 lines of hardcoded Rust catalogue code deleted.
  - AP-008..AP-013 are retired from the CLI without any scanner code
    changes — just the registry.
- **Negative:**
  - Two scanner engines remain in the tree until the IDE/MCP migration.
    Parity is not free; it requires the shared fixture suite.
  - Rust gains a runtime dependency on `registry.json` being
    present/reachable. CLI must degrade gracefully when it isn't (same
    fallback behaviour the TS loader already implements: empty
    catalogue + warning diagnostic).
- **Risks:**
  - Regex engine differences between `regex` (Rust, RE2-like, no
    backtracking) and V8 (PCRE-ish). Some patterns that work in TS may
    need rewrites to work in Rust, or vice versa.
  - Parity bugs between scanners can silently diverge user experience.
- **Mitigations:**
  - Every `.anvil` rule carries a minimum fixture file (match sample +
    negative sample). A shared test harness (`tests/scanner-parity/`)
    runs both engines against the fixtures and asserts identical warning
    IDs and locations. Any regex incompatibility surfaces as a CI
    failure at rule-authoring time, not at runtime.
  - Graceful degradation: both loaders already return an empty catalogue
    plus a diagnostic warning when `registry.json` is missing.

## References

- Related ADRs: ADR-012 (Rust CLI replacement), ADR-014 (language
  allocation), ADR-025 (package manager distribution)
- APS modules: ANVFMT (anvil-file-format), anvil-rust-scanner (new — this ADR)
- Retires as scope: ANVFMT-013 becomes the "gate CLI command" work item
  inside the new anvil-rust-scanner module.
