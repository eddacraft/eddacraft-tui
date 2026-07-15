# Agent Guidance Policy Pilot Implementation Plan

**Goal:** Generate and route version-matched policy-authoring guidance without adding an ambient Anvil context or token cost.
**Architecture:** A versioned Rust-export/Node-generator contract produces embedded Markdown/JSON assets behind one CLI-owned resolver. CLI retrieval lands first. One MCP index plus one resource template and secure leased files have separate protocol/security gates; ordinary commands never load or clean guidance.
**Tech Stack:** Rust, Node.js ESM, clap, MCP JSON-RPC resources, serde_json, SHA-256, governed Markdown

---

**APS:** OPAE-015, OPAE-016, OPAE-019, OPAE-020
**Dependencies:** OPAE-012 and OPAE-013 registries; ADR-108 Accepted.

## File map

| File | Responsibility |
| --- | --- |
| `docs/agent-guidance/policy-authoring/guidance.yaml` | Canonical topic catalogue and routing metadata. |
| `docs/agent-guidance/policy-authoring/topics/*.md` | Hand-authored agent decision guidance and cookbook narrative. |
| `crates/anvil-policy-engine/examples/export_authoring_reference.rs` | Serialise exact target/input and lint registries for generation. |
| `scripts/guidance/generate.mjs` | Deterministically assemble/check the embedded bundle. |
| `scripts/guidance/generate.test.mjs` | Drift, link-boundary, ordering, and context-budget fixtures. |
| `package.json` | `guidance:generate` and `guidance:check` commands. |
| `scripts/docs/docs-check.mjs` | Include guidance freshness in documentation validation. |
| `crates/anvil-cli/assets/guidance/policy-authoring/` | Generated embedded bundle; never hand-edit. |
| `crates/anvil-cli/src/guidance/mod.rs` | Bundle registry, lookup, and output-format contract. |
| `crates/anvil-cli/src/guidance/render.rs` | Markdown/JSON topic rendering and budget enforcement. |
| `crates/anvil-cli/src/guidance/materialise.rs` | Runtime leases, owner-only files, release, and expired sweep. |
| `crates/anvil-cli/src/commands/guidance.rs` | `list/show/explain/materialise/release/clean` CLI. |
| `crates/anvil-cli/src/commands/mod.rs` and `crates/anvil-cli/src/main.rs` | Register the guidance command. |
| `crates/anvil-cli/src/mcp/resources/guidance.rs` | One MCP index descriptor, one resource template, and routed topic reads. |
| `crates/anvil-cli/src/mcp/resources/mod.rs` | Dispatch guidance resource reads. |
| `crates/anvil-cli/tests/guidance_cli.rs` | CLI, token budget, and materialisation integration. |
| `crates/anvil-cli/tests/mcp_guidance_resources.rs` | MCP list/read/no-ambient-payload integration. |

## Task 1: Export authoritative policy-authoring registries

**Files:**

- Create: `crates/anvil-policy-engine/examples/export_authoring_reference.rs`
- Add tests beside the authoring/lint registries as required

- [ ] Write failing golden tests for the exported target matrix, lint catalogue,
      schema version, and deterministic ordering.
- [ ] Freeze a versioned JSON Schema/golden contract for the exporter output,
      including exporter schema, Anvil minimum version, registry revisions, and
      stable provenance fields.
- [ ] Implement an example binary that writes one JSON document to stdout and
      accepts no workspace path or network input:

```json
{
  "schema": "anvil.policy-authoring-reference.v1",
  "policyInput": { "version": "v1", "targets": [] },
  "lintRules": []
}
```

- [ ] Ensure the export includes code, rule, default severity, remediation,
      topic ID, target availability, and partial-availability explanation.
- [ ] Make `pnpm guidance:generate` the only supported orchestration entrypoint;
      it invokes the pinned exporter, validates its schema, and records source
      revisions without timestamps or absolute paths.
- [ ] Run the exporter and full generator twice from a clean checkout, compare
      SHA-256 digests, and assert the second pass leaves no diff.
- [ ] Commit: `feat(policy): export authoring reference registries`

## Task 2: Create governed agent topics and deterministic generator

**Files:**

- Create: `docs/agent-guidance/policy-authoring/guidance.yaml`
- Create: `docs/agent-guidance/policy-authoring/topics/*.md`
- Create: `scripts/guidance/generate.mjs`
- Create: `scripts/guidance/generate.test.mjs`
- Modify: `package.json`

- [ ] Write generator tests for duplicate IDs, unknown lint codes, missing
      upstreams/anchors, prohibited public/internal links, absolute paths,
      stable sorting, LF output, no timestamps, and token budgets.
- [ ] Run `node --test scripts/guidance/generate.test.mjs`; verify failure.
- [ ] Add topics for overview/routing, manifest, each target input matrix,
      regorus compatibility, result rules, testing, lint codes, exceptions,
      CLI workflow, cookbook, troubleshooting, migration, and scenarios.
- [ ] Implement the generator with three explicit sources: registry export,
      governed extraction, and narrative. Never scrape arbitrary headings.
- [ ] Estimate tokens conservatively and enforce 1,500 route-index / 2,500
      default-topic limits plus explicit large-topic chunk metadata.
- [ ] Add scripts:

```json
{
  "guidance:generate": "node scripts/guidance/generate.mjs",
  "guidance:check": "node scripts/guidance/generate.mjs --check"
}
```

- [ ] Run generate, check, then generate again; the second run must create no
      diff.
- [ ] Commit: `feat(guidance): generate policy authoring topics`

## Task 3: Embed one guidance registry and resolver

**Files:**

- Create: `crates/anvil-cli/assets/guidance/policy-authoring/**`
- Create: `crates/anvil-cli/src/guidance/mod.rs`
- Create: `crates/anvil-cli/src/guidance/render.rs`
- Modify: `crates/anvil-cli/src/lib.rs` or module root used by the binary
- Create: unit tests in the new guidance modules

- [ ] Write failing tests for exact topic lookup, aliases, unknown domain/topic,
      target narrowing, Markdown/JSON rendering, content digest, and runtime
      budget refusal.
- [ ] Embed generated assets with `include_str!`/`include_bytes!`; never read the
      source repository at runtime.
- [ ] Define:

```rust
pub enum GuidanceFormat { Markdown, Json }

pub struct GuidanceQuery<'a> {
    pub domain: &'a str,
    pub topic: &'a str,
    pub target: Option<PolicyTarget>,
    pub format: GuidanceFormat,
}

pub fn list(domain: &str) -> Result<GuidanceIndex>;
pub fn resolve(query: &GuidanceQuery<'_>) -> Result<GuidanceDocument>;
pub fn explain_lint(code: PolicyLintCode, format: GuidanceFormat)
    -> Result<GuidanceDocument>;
```

- [ ] Ensure no constructor scans or parses all topics before a guidance call;
      use the generated compact manifest for lookup.
- [ ] Run unit tests and binary size diff; record the bounded bundle increase.
- [ ] Commit: `feat(guidance): embed policy authoring registry`

## Task 4: Add CLI retrieval without default injection

**Files:**

- Create: `crates/anvil-cli/src/commands/guidance.rs`
- Modify: `crates/anvil-cli/src/commands/mod.rs`
- Modify: `crates/anvil-cli/src/main.rs`
- Create: `crates/anvil-cli/tests/guidance_cli.rs`

- [ ] Write failing integration tests for all command forms, unknown IDs,
      Markdown/JSON output, lint-code explanations, and the absence of guidance
      text from `start --help`, `status --json`, and `doctor --json`.
- [ ] Implement `list`, `show`, and `explain` as thin resolver adapters.
- [ ] Keep stdout machine-clean under JSON and do not initialise daemon, graph,
      project scan, or policy engine for static topic retrieval.
- [ ] Measure warm retrieval and assert a generous non-regression ceiling in a
      deterministic integration benchmark fixture rather than wall-clock unit
      assertions on shared CI.
- [ ] Commit: `feat(cli): route embedded agent guidance`

## Task 5: Add leased runtime materialisation

**Files:**

- Create: `crates/anvil-cli/src/guidance/materialise.rs`
- Modify: `crates/anvil-cli/src/commands/guidance.rs`
- Modify: `crates/anvil-cli/tests/guidance_cli.rs`

- [ ] Write failing tests with isolated `ANVIL_HOME` for
      outside-workspace placement, safe slug/digest names, owner-only mode,
      no-follow atomic writes, concurrent shared-content leases, one-hour expiry,
      release/sweep races, symlinked root/components, malformed manifests,
      interrupted creation, wrong ownership, and no sweep during unrelated
      commands.
- [ ] Resolve `<InstallRoot.user_root>/guidance/` through the established
      `--anvil-home`/`ANVIL_HOME`/platform-default precedence. Do not use a
      second runtime-root convention.
- [ ] Implement content-addressed files and a lease manifest:

```rust
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GuidanceLease {
    schema_version: u32,
    lease_id: String,
    topic_id: String,
    content_digest: String,
    relative_path: PathBuf,
    created_unix_secs: u64,
    expires_unix_secs: u64,
}
```

- [ ] Use an owner-only cross-process lock, exclusive no-follow temporary files,
      fsync where supported, and atomic rename. Under the lock, remove content
      only when no committed live lease references it; recover incomplete temp
      records and fail closed on malformed committed state.
- [ ] Make `materialise`, `release`, and `clean` the only commands that sweep;
      `list/show/explain` remain read-only and do not touch runtime state.
- [ ] Return path, lease, and expiry in both human and JSON forms.
- [ ] Commit: `feat(guidance): lease materialised topics`

## Task 6: Add one compact MCP resource and template

**Files:**

- Create: `crates/anvil-cli/src/mcp/resources/guidance.rs`
- Modify: `crates/anvil-cli/src/mcp/resources/mod.rs`
- Create: `crates/anvil-cli/tests/mcp_guidance_resources.rs`

- [ ] Write failing JSON-RPC tests proving `resources/list` adds exactly one
      guidance descriptor under 500 UTF-8 bytes,
      `resources/templates/list` adds exactly one routed template under 700
      bytes, their aggregate is at most 1,200 bytes, and `tools/list` is
      byte-for-byte unchanged.
- [ ] Add reads for `anvil://guidance` and routed topic URIs; reject unknown
      query keys, formats, traversal, and over-budget topics as `BadRequest`.
- [ ] Return Markdown with `text/markdown` and JSON with `application/json`;
      preserve the standard MCP `contents[]` envelope.
- [ ] Prove CLI and MCP return byte-equivalent documents for identical queries;
      MCP reads use the embedded resolver, do not materialise files, do
      not read customer source, and work with no daemon.
- [ ] Measure actual discovery/prompt impact in Claude Code, Codex, and OpenCode;
      stop this wave if any client eagerly injects routed content or breaches
      the aggregate budget.
- [ ] Commit: `feat(mcp): expose routed agent guidance`

## Task 7: Wire drift checks and complete the pilot gate

**Files:**

- Modify: `scripts/docs/docs-check.mjs`
- Modify: `.github/workflows/ci.yml` or the existing Rust/docs workflow that
  owns generated-asset freshness
- Modify: guidance source/generated files only when regeneration requires it

- [ ] Add `guidance:check` to local docs validation and one CI lane without
      duplicating expensive Rust builds across jobs.
- [ ] Add a public-site exclusion assertion for `docs/agent-guidance/**`.
- [ ] Run:

```sh
pnpm guidance:check
node --test scripts/guidance/generate.test.mjs
cargo test -p eddacraft-anvil --test guidance_cli
cargo test -p eddacraft-anvil --test mcp_guidance_resources
cargo fmt --all -- --check
cargo clippy -p eddacraft-anvil --all-targets -- -D warnings
pnpm docs:check
pnpm aps:active-lint
pnpm aps:index:check
```

- [ ] Record route-index/topic token counts, MCP descriptor bytes, embedded
      bundle bytes, and retrieval evidence in the PR.
- [ ] Obtain Council and independent verification before marking
      OPAE-015/016/019/020 complete.
- [ ] Commit: `ci(guidance): enforce generated reference freshness`

## Expected handoff

- Policy-authoring topics are generated from exact product registries and
  governed narrative.
- CLI and MCP resolve the same content without normal-command injection.
- Optional files are outside the workspace and lifecycle-managed.
- OPAE-017 can ship a small router skill that depends on stable topic IDs.
