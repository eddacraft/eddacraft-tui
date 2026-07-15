# Policy Authoring Lint and Agent Guidance Pilot

| Type | Authority | Owner | Status | Freshness |
| --- | --- | --- | --- | --- |
| Spec | Authoritative | OPAE | Proposed — owner design approved; ADR-108 review pending | Created 2026-07-15 against Anvil v0.9.0-beta and `main` |

| Upstream | Downstream |
| --- | --- |
| ADR-040, ADR-098, ADR-106, `crates/anvil-policy-engine`, `crates/anvil-cli/src/commands/policy/`, SKPKG | OPAE-012..020, SKPKG-009, `authoring-anvil-policy`, agent-guidance generator, policy-authoring journey tests |

## Goal

Make agent-assisted custom policy authoring a supported, deterministic Anvil
journey without adding AI to Anvil itself or imposing an ambient context/token
cost on ordinary product use.

## Success criteria

A customer can give an external coding agent a plain-language organisational
requirement and, with the installed `authoring-anvil-policy` skill:

1. choose the correct Anvil policy surface;
2. declare the intended enforcement target and required input contract;
3. produce an idiomatic, regorus-compatible pack;
4. receive stable lint diagnostics with actionable remediation;
5. run positive, negative, boundary, and malformed-input tests;
6. validate, explicitly evaluate, and exercise the policy through a real gate;
7. understand warning, violation, remediation, and exception behaviour; and
8. do all of that by loading only the guidance topics required for the task.

The same released binary must reproduce the reference content, diagnostics,
and evaluation behaviour used by the skill.

## Product boundary

The boundary is explicit:

```text
customer intent
    -> external agent + authoring-anvil-policy skill (cognitive authoring)
    -> policy files
    -> Anvil lint + compile + tests + validation + evaluation (deterministic)
    -> gate/pre-write routing (deterministic)
```

Anvil does not infer organisational intent, autonomously generate policy, call
an LLM, or accept probabilistic policy decisions. It provides deterministic
authoring tools for files that a human or external tool creates.

## Current product truth

- Pack discovery, manifest admission, metadata validation, and executable
  `test_*` rules live in `crates/anvil-policy-engine/src/pack/`.
- `anvil policy validate <pack-directory>` is the real pack admission/test
  command. `anvil policy test` currently discovers files but does not execute
  them and must not be presented by the skill as the authoritative runner.
- `anvil policy install`, `show`, `eval`, and the policy gate are shipped.
- Gate and pre-write evaluations use `PolicyInput` v1, but callers populate
  different subsets. Shape existence does not imply target availability.
- The bundled `anvil-baseline` pack proves one advisory-first path. It is not a
  comprehensive authoring or industry-scenario suite.
- Regal and Go OPA are development/reference tools. The shipping runtime is the
  Rust regorus facade.
- The managed skill installer currently embeds one hard-coded skill bundle;
  multiple bundled skills require a registry rather than another hard-coded
  branch.

## Scope

### In scope

- Versioned pack target and input declarations.
- A Rust-owned target/input availability registry.
- Deterministic static policy lint with stable diagnostics and JSON output.
- Composition of lint with current validation and executable tests.
- Generated, embedded agent-reference topics in Markdown and JSON.
- One compact CLI/MCP routing surface over the same topic registry.
- Optional leased runtime-file materialisation and guidance-only cleanup.
- The canonical `authoring-anvil-policy` catalogue skill and Anvil-vendored
  snapshot.
- Three executable industry-scenario packs and a released-binary authoring
  journey.
- Drift, context-budget, link-boundary, and example-conformance CI checks.

### Out of scope

- An Anvil-hosted LLM, natural-language policy generator, or autonomous intent
  inference.
- A general skill marketplace or remote guidance service.
- Public web publication of the comprehensive agent reference.
- Compliance certification or claims for SOC 2, ISO, GDPR, healthcare, or
  financial regulation.
- A full Rego theorem prover. Lint reports only mechanically provable issues.
- Whole-documentation migration in the pilot. Policy authoring is the test
  case; later domains require their own work items.
- Changing default policy enforcement from warnings-first.

## Architecture

```text
Rust types and registries          Governed narrative topics
  PolicyInput                       docs/agent-guidance/policy-authoring/
  PackManifest                              |
  target availability                       |
  lint diagnostics                          |
             \                              /
              guidance generator + drift checks
                            |
              embedded versioned bundle
                            |
          shared resolver/render/materialise layer
                    /                   \
       anvil guidance CLI         anvil://guidance MCP
                    \                   /
                 small routing skill
```

The resolver lives below both CLI and MCP adapters. Neither surface owns topic
content, routing vocabulary, or rendering rules.

## Pack authoring contract

### Manifest versioning

Newly authored pack manifests use version 2:

```yaml
manifest_version: 2
id: payment-change-safety
name: Payment change safety
version: 1.0.0
description: Requires tests when payment-processing code changes.
owner: payments-platform
targets:
  - gate
input_contract:
  schema: v1
  required:
    - path: diff.changed_files
      accepts: [available]
policies:
  - path: policies/payment_tests.rego
    metadata:
      id: payment-tests
      title: Payment changes require tests
      severity: high
      owner: payments-platform
      rationale: Payment behaviour must change with executable evidence.
      remediation: Add or update the payment test named by the finding.
      scope: src/payments/**
      tags: [payments, tests]
test_contract:
  cases:
    - id: payment-change-without-test
      kind: positive
      input: tests/cases/payment-change-without-test.json
      expect: finding
    - id: unrelated-change
      kind: negative
      input: tests/cases/unrelated-change.json
      expect: pass
```

`manifest_version` is optional only for legacy manifests and normalises to
version 1. A legacy manifest remains admissible but lint emits a migration
warning. Version 2 requires non-empty `targets`, `input_contract.required`, and
`test_contract.cases`. Unknown versions, input paths, availability values, case
kinds, and expectations fail closed.

Compatibility is deliberately one-way: new binaries read legacy v1 and v2;
released binaries from before manifest v2 reject v2 because their manifest
parser fails closed on unknown fields. Generated guidance and diagnostics must
therefore name the minimum Anvil version and never claim that a v2 pack is
portable to an older binary.

Canonical input paths are root-relative to `PolicyInput` and never include an
`input.` prefix. Each requirement names the availability classifications the
author accepts for every declared target. `partial` or `caller-supplied` is
accepted only when stated explicitly; `unavailable` can never satisfy a
requirement.

Policy identity normalises as follows: policy metadata IDs are lowercase
kebab-case; the Rego filename stem and final production-package segment are
the same ID with hyphens converted to underscores; the test package adds
`_test`. Pack identity is separate and is not required to equal a policy ID.

### Evaluation targets

The stable target vocabulary is:

| Target | Meaning |
| --- | --- |
| `explicit-eval` | Input is supplied explicitly by the caller; all fields are caller-supplied rather than guaranteed by Anvil. |
| `gate` | Repository/change-set policy check run by `anvil gate`. |
| `pre-write` | Bounded off-daemon policy evaluation used by write-validation surfaces. |

In manifest v2, targets are admission-compatibility declarations only. They do
not install, activate, select, or route a pack. Gate and pre-write continue to
use their existing configuration until a separately tested runtime-routing
contract is accepted; validation must say this plainly.

The registry classifies every input path per target as:

- `available` — Anvil guarantees the field is populated with target-relevant
  data;
- `partial` — a meaningful but deliberately incomplete projection is supplied;
- `caller-supplied` — permitted only for explicit evaluation and not guaranteed;
- `unavailable` — the target does not supply the field.

The initial registry must be derived from current producers, not from the shape
of `PolicyInput` alone. At minimum it records:

| Input path | Explicit eval | Gate | Pre-write |
| --- | --- | --- | --- |
| `schema_version` | caller-supplied | available | available |
| `diff.changed_files` | caller-supplied | available | available |
| `repo_state.files` | caller-supplied | partial | partial |
| `repo_state.edges` | caller-supplied | unavailable until wired | unavailable |
| `diff.new_edges` | caller-supplied | unavailable until wired | unavailable |
| `plans` | caller-supplied | unavailable until wired | unavailable |
| `decisions` | caller-supplied | unavailable until wired | unavailable |
| `baseline.findings` | caller-supplied | unavailable until wired | unavailable |

Implementation tests must sit beside the actual producers: gate parity belongs
in `anvil-cli`, while pre-write parity belongs with the engine-owned producer.
If a producer changes, registry parity fails in the same PR.

For an enforcement target, a required field classified `unavailable` is a lint
error. `partial` or `caller-supplied` is an error unless the requirement's
`accepts` list explicitly includes that classification. Explicit-eval-only
packs may require any v1 field only when they accept `caller-supplied`.

### Executable case contract

Every v2 policy pack declares deterministic JSON cases. `kind` is one of
`positive`, `negative`, `boundary`, or `malformed`; `expect` is one of
`finding`, `pass`, or `input-error`. Paths are pack-relative, cannot escape the
pack root, and are unique after normalisation. Positive and negative cases are
mandatory. Boundary cases are mandatory only for declared threshold/tunable
policies; malformed cases are strongly recommended in the first wave and may
become mandatory only after beta evidence.

Static lint verifies declarations, referenced files, and recognised values.
Composed validation executes each case exactly once and proves its expected
outcome; passing `test_*` rules alone do not prove whether a case exercises
finding or passing behaviour.

## Deterministic linter

### Command contract

```text
anvil policy lint <pack-directory|pack.yaml> [--target <target>] [--json]
```

- The manifest target declaration is authoritative for admission. `--target` narrows the
  report and errors if it names an undeclared target.
- Human output groups diagnostics by file and policy and prints remediation.
- JSON output is stable, sorted, and suitable for agent repair loops.
- Errors exit non-zero. Warnings exit zero.
- Lint never mutates the pack and never accesses the network.

`anvil policy validate` runs, in order:

1. manifest load and structural admission;
2. deterministic lint;
3. regorus compilation;
4. executable policy tests; and
5. report composition with deterministic diagnostic ordering.

Compilation/test failures must not be duplicated under multiple codes.

### Diagnostic shape

```json
{
  "schema": "anvil.policy-lint.v1",
  "valid": false,
  "diagnostics": [
    {
      "code": "POL002",
      "severity": "error",
      "rule": "input-unavailable-for-target",
      "message": "diff.new_edges is unavailable at pre-write",
      "remediation": "Use gate or remove this required input",
      "topic": "policy-authoring.input.pre-write",
      "target": "pre-write",
      "policyId": "new-edge-rule",
      "path": "pack.yaml",
      "line": 12,
      "column": 7
    }
  ]
}
```

Fields that cannot be located precisely use `null`; the linter must never
invent source coordinates.

### First-wave rules

| Code | Rule | Default severity | Requirement |
| --- | --- | --- | --- |
| `POL001` | `target-declaration-missing` | warning for legacy, error for v2 | New packs declare targets and input contract. |
| `POL002` | `input-unavailable-for-target` | error | Required fields must be supplied at every declared enforcement target. |
| `POL003` | `package-outside-anvil-namespace` | error when parser-proven | Production package is `anvil.policies.<name>`. |
| `POL004` | `identity-mismatch` | error when parser-proven | Policy ID, filename, production package, and test package follow the canonical mapping. |
| `POL005` | `unsupported-regorus-capability` | error during admission | Policy compiles with the embedded deterministic regorus capability set. |
| `POL006` | `result-rule-missing` | error when parser-proven | A production policy exposes a recognised warning/violation family. |
| `POL007` | `result-shape-invalid` | error during conformance | Executed results are parseable by the shipped finding extractor. |
| `POL008` | `positive-test-missing` | error | The case contract declares and validation proves a finding-producing input. |
| `POL009` | `negative-test-missing` | error | The case contract declares and validation proves a passing input. |
| `POL010` | `boundary-test-missing` | warning | Tunable/threshold policies exercise the boundary. |
| `POL011` | `test-package-mismatch` | error when parser-proven | Test package mirrors production with `_test`. |
| `POL012` | `metadata-guidance-incomplete` | error | Owner, rationale, scope, tags, severity, and actionable remediation data are present. |
| `POL013` | `non-deterministic-builtin` | error when compiler-proven | Time, random, network, runtime, and other fenced built-ins are refused. |
| `POL014` | `suspicious-unconditional-result` | warning | Statically obvious unconditional warning/violation is reviewed. |

Rules based on semantic heuristics remain warnings. If regorus does not expose
the parser/compiler evidence needed for an error-class rule, the implementation
must downgrade or defer that rule instead of adding a second Rego parser or a
lexical approximation. A lint rule may be promoted to error only with
false-positive fixtures and a stable remediation path.

`policy lint` and `policy validate` use one engine-owned admission session.
Sources are loaded and compiled at most once per command. Lint stops before
executing cases; validation continues from the same compiled session and runs
each case once. Validation does not shell out to, or re-run, the lint command.

### Relationship with Regal

Anvil may run Regal in repository CI for style comparison, but customer
correctness cannot depend on a separate binary. Anvil-specific rules, target
availability, diagnostic codes, and exit behaviour live in Rust. The agent
skill links to official Rego references for language detail and uses Anvil
diagnostics for product admission.

## Agent guidance system

### No ambient context cost

The following budgets are acceptance gates for the pilot:

| Surface | Budget |
| --- | ---: |
| Installed skill routing body | 1,200 estimated tokens |
| MCP `resources/list` guidance descriptor | 500 UTF-8 bytes |
| MCP `resources/templates/list` guidance template | 700 UTF-8 bytes |
| Aggregate added MCP discovery payload | 1,200 UTF-8 bytes |
| Guidance route index | 1,500 estimated tokens |
| Default individual topic | 2,500 estimated tokens |
| Default retrieval | one topic; no recursive prefetch |

The estimator is deterministic and conservative. A topic may exceed the
default only with an explicit `large: true`, a recorded reason, and chunk
routes. Normal `anvil start`, `status`, `doctor`, `gate`, and MCP tool listings
do not load or render guidance.

### Topic metadata

Every source topic declares:

```yaml
id: policy-authoring.input.gate
domain: policy-authoring
audience: agent
schema_version: 1
targets: [gate]
triggers:
  - which policy inputs are available at gate
  - write a gate policy
upstream:
  - crates/anvil-policy-engine/src/input.rs
  - crates/anvil-cli/src/commands/gate.rs
formats: [markdown, json]
max_tokens: 1800
related_lint_codes: [POL001, POL002]
```

Topic IDs and lint codes are stable compatibility surfaces. Renames require an
alias for at least one minor release.

### Source modes and generation

The generator supports three explicit source modes:

1. `registry` — exact fields, defaults, commands, target availability, and lint
   diagnostics serialised from Rust registries;
2. `extract` — explicitly marked sections from governed documentation; and
3. `narrative` — hand-authored routing, decision guidance, troubleshooting, and
   examples under `docs/agent-guidance/`.

Generated outputs are written under
`crates/anvil-cli/assets/guidance/policy-authoring/` and embedded in the binary.
They are marked generated and never hand-edited. Generation performs no network
access and records source paths, content hashes, Anvil version, and generator
schema version.

The comprehensive bundle is excluded from public docs applications. Public
pages may document installation and command availability without linking into
the agent reference.

### Retrieval surfaces

CLI:

```text
anvil guidance list policy-authoring [--json]
anvil guidance show policy-authoring --topic <id> [--target <target>] [--format markdown|json]
anvil guidance explain <lint-code> [--format markdown|json]
anvil guidance materialise policy-authoring --topic <id> [--target <target>] [--format markdown|json]
anvil guidance release <lease-id>
anvil guidance clean [--expired]
```

MCP advertises one compact static descriptor at `anvil://guidance`. Reading it
returns the route index. `resources/templates/list` advertises one bounded
RFC-6570-style template for routed reads:

```text
anvil://guidance/policy-authoring/<topic>?target=<target>&format=<format>
```

The MCP adapter does not materialise files. It returns the selected topic
directly and shares the same resolver and token cap as the CLI.

### Runtime materialisation

Materialisation is an opt-in compatibility path implemented only after its
security and concurrency tests are green:

- root: `<InstallRoot.user_root>/guidance/`, using the same resolved
  `--anvil-home`/`ANVIL_HOME`/platform-default precedence as other CLI-owned
  user state;
- directory permissions: owner-only where the platform supports it;
- filename: content digest plus safe topic slug;
- lease manifest: lease ID, topic ID, digest, created time, expiry, and relative
  file path;
- default TTL: one hour;
- creation: refuse symlinked roots or path components, create temporary content
  and manifests with no-follow/exclusive semantics, fsync where supported, and
  atomically rename into place;
- concurrency: take an owner-only guidance lock before changing leases or
  sweeping; content is removed only after locked reference counting proves no
  live lease refers to it;
- release: atomically remove the lease and then unreferenced content;
- sweep: run only on `anvil guidance materialise|release|clean`; TTL means
  eligible for cleanup, not background deletion;
- recovery: ignore and remove incomplete temporary records under the same lock,
  but fail closed on malformed committed manifests or ownership mismatch;
- repository writes: forbidden unless a future explicit export command is
  separately designed.

Ordinary Anvil commands never sweep, index, or open the guidance bundle.

## `authoring-anvil-policy` skill

The skill is a router and workflow, not a reference dump. It must:

1. decide configuration/built-in check/custom Rego;
2. clarify invariant, scope, severity, remediation, and examples;
3. select the target and retrieve its input-availability topic;
4. inspect the shipped starter pack and author/edit v2 files; it must not invent
   a scaffold command that the capability topic does not advertise;
5. lint and iterate on machine-readable diagnostics;
6. validate the exact pack directory;
7. explicitly evaluate representative input;
8. run a policy-only gate and, when supported, a separate pre-write proof;
9. explain exceptions and proof limits; and
10. report missing product capabilities instead of fabricating them.

The canonical source lives at
`eddacraft-skills/skills/eddacraft/authoring-anvil-policy/`. OPAE owns its
policy workflow and route contract. SKPKG-009 owns vendoring a pinned snapshot
and extending the managed bundle while reusing ADR-106's client registry.
Existing `anvil skill install` behaviour remains compatible for scripts that do
not name a skill.

## Industry-scenario conformance

The pilot ships three executable scenario fixtures, all limited to inputs the
gate actually supplies:

| Scenario | Invariant | Target |
| --- | --- | --- |
| Payments | Changes under payment-processing paths require companion test changes. | gate |
| Clinical rules | Changes under clinical-decision paths require verification fixture changes. | gate |
| Platform/SRE | Production deployment changes require both rollback and alerting evidence. | gate |

The behaviours are deliberately non-isomorphic: broad companion category,
same-stem pairing, and two-part evidence. Each fixture contains a v2 manifest,
production Rego, positive/negative/boundary cases, explicit inputs, passing and
failing repository changes, and expected diagnostics. The agent receives a
held-out plain-language invariant rather than the implementation. Names and
prose say “industry scenario”, never “compliance pack”.

The released-binary journey is:

```text
install authoring skill
  -> retrieve route/index topic
  -> copy or author scenario pack
  -> policy lint
  -> policy validate
  -> policy eval
  -> gate --only-checks policy
  -> inspect remediation and exception guidance
```

## Synchronisation and CI

`guidance:check` fails for:

- generated output different from a clean regeneration;
- missing or duplicate topic IDs;
- a skill route referencing an unknown topic;
- a lint code without a guidance topic;
- target availability different from gate/pre-write producer-owned fixtures;
- source paths or governed extraction anchors that no longer exist;
- topic or route-index context budgets exceeded;
- prohibited internal catalogue links or public-doc-site links;
- generated output containing absolute build paths or secrets;
- executable scenario packs that fail lint, validation, eval, or gate fixtures;
- an embedded bundle whose provenance does not name the source revision; or
- public docs navigation including `docs/agent-guidance/`.

Generation is deterministic: sorted topics, LF line endings, stable JSON key
ordering, no timestamps in content-addressed outputs, and no network access.
Release provenance may record the build version outside the content digest.

## Rollout

### Wave 0 — Contract review

- Land ADR-108 and this spec.
- Reconcile OPAE scope and create exact work items.
- Confirm the canonical skill name and catalogue ownership.

### Wave 1 — Lint foundation

- Land manifest v2 and target/input registry.
- Land lint engine and stable diagnostics.
- Compose lint into validation and migrate the bundled starter pack.

Exit: all current packs remain usable; v2 packs with unavailable inputs fail
before evaluation.

### Wave 2 — Guidance generation and CLI pilot

- Land the generator, embedded policy-authoring bundle, route index, and drift
  checks.
- Land CLI retrieval over the shared resolver.

Exit: ordinary commands are unchanged; CLI topic retrieval works offline.

### Wave 3 — MCP and secure materialisation

- Land the one MCP index resource and one resource template only after protocol
  conformance and real-client aggregate-context measurements pass.
- Land leased materialisation only after the filesystem race, symlink,
  ownership, atomicity, and crash-recovery suite passes.

Exit: MCP discovery remains within the aggregate budget; CLI and MCP return
byte-equivalent topics; concurrent leases cannot delete live content.

### Wave 4 — Skill and scenarios

- Land `authoring-anvil-policy` in the catalogue.
- Vendor the reviewed snapshot and generalise the managed skill registry.
- Land three executable industry scenarios and the full authoring journey.

Exit: a downloaded release artefact can complete the journey with an explicit
non-interactive `--client`, without private repo or public docs access.

### Wave 5 — Beta observation and hardening

- Exercise the journey across supported skill clients.
- Persist the client/version/scenario/result matrix under governed release
  evidence. The primary matrix covers Claude Code, Codex, and OpenCode; any
  unsupported client is recorded as a gap, not silently skipped.
- Track lint false positives, unavailable-input catches, first-route topic
  selection, retrieved bytes/tokens, materialisation cleanup, and command
  latency in test evidence. Stop rollout on any false-negative enforcement
  result, destructive install/cleanup behaviour, unresolved cross-client
  contract divergence, or context-budget breach.
- Promote only proven advisory rules to errors.

### Wave 6 — Generalisation decision

- Review policy-pilot evidence.
- Decide which other Anvil domains justify guidance topics.
- Do not bulk-convert existing documentation without a new APS item.

## Acceptance tests

- Legacy manifest: accepted with `POL001` warning.
- V2 manifest missing target/input contract: rejected.
- Pre-write pack requiring `diff.new_edges`: rejected with `POL002`.
- Gate pack requiring `diff.changed_files`: accepted.
- Unsupported or non-deterministic Rego: rejected with stable code.
- Missing positive or negative test: rejected.
- Human and JSON diagnostic ordering: stable across repeated runs.
- `policy validate`: includes lint and executes tests once.
- Normal `anvil start` and MCP `tools/list`: no guidance payload.
- MCP `resources/list`: one bounded guidance descriptor.
- MCP `resources/templates/list`: one bounded routed-read template; aggregate
  added discovery payload stays within budget in real primary clients.
- Topic retrieval: one topic only, within budget, offline.
- Materialisation: outside workspace, owner-only, releasable, expiry-swept only
  by guidance commands.
- Generator: byte-identical on a clean second run.
- Skill route: every topic/code resolves.
- Industry scenarios: pass lint/validate and produce expected gate outcomes.

## Risks and mitigations

| Risk | Mitigation |
| --- | --- |
| Static lint overclaims semantic correctness | Keep heuristic rules advisory; require executable tests and gate fixtures. |
| Manifest v2 breaks older binaries | State the one-way compatibility matrix and minimum Anvil version; preserve legacy v1 reading in new binaries. |
| Guidance becomes another stale documentation set | Generate exact facts from code registries; provenance-hash narrative upstreams; fail CI on drift. |
| MCP routing adds context cost | Advertise one sub-500-byte descriptor; retrieve only on demand. |
| Temporary files accumulate | Leases, TTL, explicit release, and guidance-only sweep. |
| Agent invents future commands | Skill uses capability/version topics and refuses unavailable commands. |
| Examples imply regulatory compliance | Label as industry scenarios and restrict claims to deterministic behaviour. |
| Generic guidance system expands scope | Pilot only policy authoring; require a later evidence-based generalisation decision. |

## Documentation closeout contract

Implementation PRs update authoritative code/schema references first, regenerate
agent guidance, run `guidance:check`, and then update human narrative only where
behaviour changed. Generated assets are never hand-edited. Public documentation
receives only minimal installation/availability changes and must not link the
comprehensive agent bundle.
