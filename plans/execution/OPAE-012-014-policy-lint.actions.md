# Policy Authoring Target Contract and Lint Implementation Plan

**Goal:** Ship target-aware policy-pack manifests, deterministic Anvil Rego lint, and one composed lint/validation admission path.
**Architecture:** `anvil-policy-engine` owns manifest, target availability, diagnostics, and a compile-once admission session; `anvil-cli` is a thin human/JSON adapter. Producer-owned parity tests cover the real gate and pre-write inputs. Static lint, compilation evidence, and executable conformance are separate phases of one validation session.
**Tech Stack:** Rust, serde/serde_yaml, regorus, clap, insta, Cargo integration tests

---

**APS:** OPAE-012, OPAE-013, OPAE-014
**Design:** `plans/specs/2026-07-15-policy-authoring-lint-and-agent-guidance.md`
**Gate:** ADR-108 must be Accepted before Task 1 implementation begins.

## File map

| File | Responsibility |
| --- | --- |
| `crates/anvil-policy-engine/src/authoring.rs` | Stable target vocabulary, input availability registry, and manifest input-contract validation. |
| `crates/anvil-policy-engine/src/pack/manifest.rs` | Legacy/v2 manifest parsing and target/input/case declarations. |
| `crates/anvil-policy-engine/src/lint/mod.rs` | Lint orchestration and deterministic report ordering. |
| `crates/anvil-policy-engine/src/lint/diagnostic.rs` | Stable `POL001`..`POL014` diagnostic wire contract. |
| `crates/anvil-policy-engine/src/lint/source.rs` | Bounded Rego/package/test source extraction shared by lint rules. |
| `crates/anvil-policy-engine/src/lint/rules.rs` | First-wave Anvil Rego-family rules. |
| `crates/anvil-policy-engine/src/lib.rs` | Public exports for authoring/lint contracts. |
| `crates/anvil-policy-engine/tests/policy_authoring_contract.rs` | Manifest migration, case contracts, and engine-owned pre-write parity. |
| `crates/anvil-policy-engine/tests/policy_lint.rs` | Rule-level positive/negative diagnostics and stable snapshots. |
| `crates/anvil-policy-engine/tests/fixtures/lint/` | Minimal valid/invalid packs for every lint code. |
| `crates/anvil-cli/src/commands/policy/lint.rs` | `anvil policy lint` path resolution and human/JSON rendering. |
| `crates/anvil-cli/src/commands/policy/validate.rs` | Compose structural validation, lint, compilation, and tests once. |
| `crates/anvil-cli/src/commands/policy/mod.rs` | Register the lint subcommand. |
| `crates/anvil-cli/tests/policy_lint.rs` | Released-binary CLI contract, ordering, and exit semantics. |
| `crates/anvil-cli/tests/policy_input_gate_parity.rs` | Gate producer parity against the target availability registry. |
| `crates/anvil-cli/src/commands/policy/starter_packs/anvil-baseline/pack.yaml` | Migrate the shipped starter pack to manifest v2. |
| `docs/specs/policy-input-v1.md` | Document target population separately from input shape. |
| `docs/guides/opa-policy-testing.md` | Replace stale authoring/test guidance with lint/validate workflow. |

## Task 1: Add manifest v2 and target availability

**Files:**

- Create: `crates/anvil-policy-engine/src/authoring.rs`
- Modify: `crates/anvil-policy-engine/src/pack/manifest.rs`
- Modify: `crates/anvil-policy-engine/src/lib.rs`
- Create: `crates/anvil-policy-engine/tests/policy_authoring_contract.rs`
- Create: `crates/anvil-cli/tests/policy_input_gate_parity.rs`

- [ ] Write failing tests for the full old/new binary by v1/v2 compatibility
      matrix, v2 required fields, unknown paths/values, path traversal,
      deterministic serialisation, exact identity normalisation, remediation
      metadata, and positive/negative case declarations.
- [ ] Put gate parity beside the real CLI constructor and pre-write parity
      beside its engine-owned producer; prove `repo_state.files` is partial.
- [ ] Run
      `cargo test -p eddacraft-anvil-policy-engine --test policy_authoring_contract`
      and `cargo test -p eddacraft-anvil --test policy_input_gate_parity`; verify
      failures name missing types/fields and producer mismatches.
- [ ] Implement the public contract:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PolicyTarget {
    ExplicitEval,
    Gate,
    PreWrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InputAvailability {
    Available,
    Partial,
    CallerSupplied,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyInputContract {
    pub schema: SchemaVersion,
    #[serde(default)]
    pub required: Vec<InputRequirement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputRequirement {
    pub path: PolicyInputPath,
    pub accepts: BTreeSet<InputAvailability>,
}

pub fn input_availability(
    target: PolicyTarget,
    input_path: &str,
) -> Option<InputAvailability>;
```

- [ ] Extend `PackManifest` with a legacy-defaulted `manifest_version`, sorted
      `targets`, `input_contract`, and typed `test_contract`; validate v2
      strictly while v1 continues to load. State in diagnostics that targets
      do not activate or route packs.
- [ ] Keep unknown fields fail-closed and preserve manifest member order.
- [ ] Run the targeted test and the full engine suite; expect all green.
- [ ] Commit: `feat(policy): declare pack targets inputs and cases`

## Task 2: Freeze the lint diagnostic contract

**Files:**

- Create: `crates/anvil-policy-engine/src/lint/diagnostic.rs`
- Create: `crates/anvil-policy-engine/src/lint/mod.rs`
- Modify: `crates/anvil-policy-engine/src/lib.rs`
- Create: `crates/anvil-policy-engine/tests/policy_lint.rs`

- [ ] Write failing serde/snapshot tests for every diagnostic field, code label,
      severity, missing source location, and deterministic sorting.
- [ ] Run `cargo test -p eddacraft-anvil-policy-engine --test policy_lint diagnostic`
      and verify it fails before implementation.
- [ ] Implement:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum PolicyLintCode {
    Pol001, Pol002, Pol003, Pol004, Pol005, Pol006, Pol007,
    Pol008, Pol009, Pol010, Pol011, Pol012, Pol013, Pol014,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyLintDiagnostic {
    pub code: PolicyLintCode,
    pub severity: PolicyLintSeverity,
    pub rule: String,
    pub message: String,
    pub remediation: String,
    pub topic: String,
    pub target: Option<PolicyTarget>,
    pub policy_id: Option<String>,
    pub path: PathBuf,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyLintReport {
    pub schema: String,
    pub valid: bool,
    pub diagnostics: Vec<PolicyLintDiagnostic>,
}
```

- [ ] Sort by normalised path, location, code, target, and policy ID; recompute
      `valid` from error-class diagnostics rather than trusting callers.
- [ ] Map every code to a stable kebab-case rule, default severity, remediation
      template, and `policy-authoring.*` guidance topic.
- [ ] Run targeted and full engine tests; expect stable snapshots.
- [ ] Commit: `feat(policy): freeze lint diagnostics`

## Task 3: Land parser-proven structural lint

**Files:**

- Create: `crates/anvil-policy-engine/src/lint/source.rs`
- Create: `crates/anvil-policy-engine/src/lint/rules.rs`
- Modify: `crates/anvil-policy-engine/src/lint/mod.rs`
- Add: `crates/anvil-policy-engine/tests/fixtures/lint/**`
- Modify: `crates/anvil-policy-engine/tests/policy_lint.rs`

- [ ] First spike the regorus parser/compiler API in a test. Record which facts
      are exposed without maintaining a second parser; downgrade or defer any
      rule whose evidence is unavailable.
- [ ] Add one minimal valid pack plus adversarial fixtures for manifest,
      namespace, identity, metadata, test-package, and capability rules. Cover
      comments, strings, aliases, comprehensions, undefined results, and source
      limits; each fixture asserts code, severity, target, path, and remediation.
- [ ] Add false-positive controls for comments/strings that mention package or
      result-rule tokens without defining them.
- [ ] Run `cargo test -p eddacraft-anvil-policy-engine --test policy_lint rules`
      and verify fixtures fail.
- [ ] Implement bounded source extraction with explicit byte/file/member caps;
      reuse regorus compilation for language/capability errors rather than
      maintaining another Rego parser.
- [ ] Implement only target/input, namespace, identity, test-package, metadata,
      and deterministic-capability checks supported by manifest or
      parser/compiler evidence. Keep semantic heuristics warning-only.
- [ ] Statically require typed positive and negative case declarations and
      referenced JSON inputs; never infer case intent from `test_*` names.
- [ ] Ensure unsupported/unprovable semantic analysis produces no finding
      rather than a confident false error.
- [ ] Run the targeted tests twice and compare JSON snapshots byte-for-byte.
- [ ] Run `cargo test -p eddacraft-anvil-policy-engine`; expect green.
- [ ] Commit: `feat(policy): lint provable Anvil Rego contracts`

## Task 3b: Land executable conformance diagnostics

**Files:**

- Modify: `crates/anvil-policy-engine/src/lint/rules.rs`
- Modify: `crates/anvil-policy-engine/src/validation.rs`
- Add: `crates/anvil-policy-engine/tests/fixtures/conformance/**`
- Modify: `crates/anvil-policy-engine/tests/policy_lint.rs`

- [ ] Add failing fixtures proving declared positive, negative, boundary, and
      malformed cases produce the exact expected finding/pass/input-error
      outcome and a valid finding shape.
- [ ] Introduce the engine-owned admission session that loads and compiles each
      member once, then executes each declared case once.
- [ ] Emit `POL005`, `POL007`, `POL008`, `POL009`, and applicable `POL010`
      evidence from the appropriate compile/conformance phase without reloading
      or re-running cases.
- [ ] Add counters/test hooks proving one load, compile, and case execution per
      validation command.
- [ ] Commit: `feat(policy): validate executable authoring cases`

## Task 3c: Add advisory heuristics behind evidence gates

- [ ] Add `POL014` only with a false-positive corpus and warning severity.
- [ ] Keep any unprovable result-family or builtin heuristic out of error-class
      admission; document deferred codes rather than simulating an AST with
      string matching.
- [ ] Commit: `feat(policy): add evidence-backed lint advisories`

## Task 4: Add `anvil policy lint`

**Files:**

- Create: `crates/anvil-cli/src/commands/policy/lint.rs`
- Modify: `crates/anvil-cli/src/commands/policy/mod.rs`
- Create: `crates/anvil-cli/tests/policy_lint.rs`

- [ ] Write failing CLI tests for directory/manifest resolution, target
      narrowing, undeclared target rejection, JSON schema, human remediation,
      warning exit 0, error exit non-zero, path redaction, and repeat ordering.
- [ ] Run `cargo test -p eddacraft-anvil --test policy_lint`; verify failure.
- [ ] Add the clap surface:

```rust
#[derive(Debug, Args)]
pub struct LintArgs {
    /// Pack manifest file, or directory containing pack.yaml.
    pub path: PathBuf,
    /// Narrow lint to one target declared by the manifest.
    #[arg(long, value_enum)]
    pub target: Option<PolicyTarget>,
}
```

- [ ] Resolve and canonicalise within the selected pack, invoke the engine once,
      render the stable report, and return `AlreadyReported` for error reports.
- [ ] Keep `--json` on stdout and human operational errors/remediation on the
      established output streams.
- [ ] Run CLI integration tests and `cargo clippy -p eddacraft-anvil --all-targets -- -D warnings`.
- [ ] Commit: `feat(cli): add policy lint command`

## Task 5: Compose lint into validation

**Files:**

- Modify: `crates/anvil-cli/src/commands/policy/validate.rs`
- Modify: `crates/anvil-cli/tests/policy_lint.rs`
- Modify/add: policy validation unit tests beside `validate.rs`

- [ ] Add failing tests showing `policy validate` includes lint diagnostics,
      compiles each member once, executes tests once, removes duplicate load or
      compile errors, and preserves warning/error exit semantics.
- [ ] Run `cargo test -p eddacraft-anvil -- policy_validate`; verify failure.
- [ ] Replace the ad-hoc report assembly with one composed admission report:

```text
load manifest and bounded sources once
  -> manifest/static lint
  -> compile members once with regorus
  -> compiler-backed diagnostics
  -> execute each declared case once
  -> conformance diagnostics and existing Rego tests
  -> deduplicate by (code, policy, path, location)
  -> stable sort and render
```

- [ ] Preserve the existing `anvil.policy-validation` JSON compatibility or
      introduce a documented v2 envelope that nests structural and lint
      diagnostics; do not silently change field meaning.
- [ ] Add a regression proving `anvil policy test` is not called or advertised
      as the execution authority.
- [ ] Run targeted CLI and full policy-engine suites.
- [ ] Commit: `feat(policy): compose lint into pack validation`

## Task 6: Migrate the starter pack and authoring references

**Files:**

- Modify: `crates/anvil-cli/src/commands/policy/starter_packs/anvil-baseline/pack.yaml`
- Modify: starter-pack policy tests as required
- Modify: `docs/specs/policy-input-v1.md`
- Modify: `docs/guides/opa-policy-testing.md`

- [ ] Add a failing starter-pack proof that requires manifest v2, declares
      `gate`/`pre-write` honestly, and passes all error-class lint rules.
- [ ] Migrate the manifest without claiming unavailable edge, plan, decision,
      baseline, or configuration inputs.
- [ ] Add the target availability table to the PolicyInput reference while
      keeping `input.rs` authoritative for shape.
- [ ] Update the testing guide to use `policy lint`, the exact pack directory
      for `policy validate`, explicit eval, and a real policy gate.
- [ ] Run:

```sh
cargo test -p eddacraft-anvil-policy-engine
cargo test -p eddacraft-anvil --test policy_lint
cargo test -p eddacraft-anvil -- starter_policy_pack
cargo fmt --all -- --check
cargo clippy -p eddacraft-anvil-policy-engine --all-targets -- -D warnings
cargo clippy -p eddacraft-anvil --all-targets -- -D warnings
pnpm docs:check
pnpm aps:active-lint
pnpm aps:index:check
```

- [ ] Record Council and independent verification evidence before marking
      OPAE-012..014 complete.
- [ ] Commit: `docs(policy): align authoring with lint contract`

## Expected handoff

- The one-way manifest compatibility matrix and v2 enforcement are proven.
- Every stable lint code has fixtures and a future guidance topic ID.
- `policy lint` and `policy validate` share one engine admission session;
  compile and conformance work is not duplicated within a command.
- OPAE-015 can serialise the target and lint registries without scraping prose.
