# ADR-108: Deterministic policy authoring lint and routed agent guidance

- **Status:** Accepted
- **Date:** 2026-07-16
- **Owners:** Policy, CLI, and Agent Integrations
- **Related:** OPAE, SKPKG, OPAG, ADR-040, ADR-098, ADR-106

## Context

Anvil can discover, install, validate, evaluate, and enforce customer-authored
Rego packs, but the authoring experience still requires users or their agents
to understand pack structure, the `PolicyInput` contract, regorus compatibility,
and the different data available at explicit-eval, gate, and pre-write targets.
The current pack validator proves manifest structure and executes tests; it is
not a comprehensive, customer-facing authoring linter.

The product rule that Anvil itself does not generate policy intent with AI does
not remove the need to support customers who use external cognitive tools.
Treating agent-assisted authoring as exceptional would make the supported path
needlessly difficult and would encourage unvalidated Rego. The trust boundary
must instead sit at deterministic lint, compilation, tests, validation, and
enforcement.

Comprehensive agent guidance must also avoid becoming an ambient context cost.
Putting reference material into startup prompts, root agent instructions, every
MCP tool description, or one large always-loaded skill would charge every Anvil
session for guidance that most tasks do not need. Publishing the same material
as public web documentation would create another distribution and synchrony
surface without improving enforcement.

## Decision

Anvil will provide a deterministic policy-authoring contract with four parts.

1. `pack.yaml` will gain a versioned target, input, and executable-case
   declaration. New packs declare intended evaluation targets, accepted input
   availability, and positive/negative conformance cases. Targets are admission
   compatibility metadata, not activation or runtime-routing instructions.
   New binaries continue to read legacy packs; old binaries are not expected to
   read v2 because their strict parser rejects unknown fields.
2. `anvil policy lint` will check only contracts that can be proven from the
   manifest and regorus parser/compiler evidence and emit stable,
   machine-readable diagnostics. One engine-owned admission session backs both
   lint and validate: sources compile once per command, and validation continues
   by executing each declared case once. Heuristic semantic rules remain
   advisory. Regal and Go OPA remain reference and compatibility tools only.
3. Anvil will embed a generated, version-matched agent-guidance bundle. A small
   installed skill routes to individual topics. CLI and MCP are thin adapters
   over one resolver. No guidance is injected by normal `anvil` commands. MCP
   advertises one compact index resource and one bounded resource template.
4. Guidance may be returned directly or materialised as a leased user-state
   file only after the dedicated filesystem safety gate passes. Materialisation
   uses the shared install-root resolver, no-follow atomic creation, locked
   reference-counted leases, restrictive permissions, and guidance-command-only
   cleanup. Ordinary Anvil execution performs no guidance cleanup or loading.

Policy authoring is the first guidance-system test case. The infrastructure is
domain-neutral, but migration of other Anvil documentation requires separate
evidence and planning.

The canonical `authoring-anvil-policy` skill remains in the private
`eddacraft-skills` catalogue. OPAE owns its policy content; SKPKG owns extending
the managed bundle from one skill to a typed multi-skill content registry.
Client detection, destinations, and capability decisions remain exclusively in
ADR-106's agent-client registry. Installed content is readable by the customer;
it is not secret, but it is not part of the public docs-site build or
navigation.

## Amendment (2026-08-23, ADR-130)

This contract is the Rego power-user toolchain. It is not the supported
answer to "how do I write a policy?". The on-ramp is YAML pack source
([ADR-130](130-policy-authoring-on-ramp.md)). Implementation of this
contract still begins at OPAE-012.

## Consequences

- Customers and agents get a supported **Rego power-user** authoring route
  (lint, generated guidance, small skill) without adding probabilistic
  behaviour to Anvil's enforcement engine. The supported answer to "how do
  I write a policy?" is YAML pack source (ADR-130).
- Target declarations make unavailable inputs detectable before a policy is
  deployed to a surface where it can never fire, but do not activate the pack.
- The binary grows by a bounded guidance bundle and a second managed skill.
- The CLI, MCP resources, generated Markdown, generated JSON, and lint catalogue
  must be built from shared registries and checked for drift in CI.
- Legacy manifests require a documented migration window and cannot receive
  the same target-availability guarantee until upgraded.
- Anvil does not promise that static lint can prove policy intent. Business
  correctness remains covered by authored tests and real gate fixtures.
- Public documentation may explain that the skill and commands exist, but does
  not mirror or link the comprehensive agent reference bundle.

## Alternatives considered

- **Fold authoring into `using-anvil`.** Rejected because product operation and
  policy design have different triggers, reference depth, and validation
  workflows.
- **Use Regal as the shipped linter.** Rejected as the only product path because
  it adds an external runtime and cannot own Anvil-specific target/input
  availability. Regal remains useful for reference compatibility in CI.
- **Put every reference in the installed skill.** Rejected because it produces
  a large context load and duplicates product registries.
- **Publish agent references on the public docs site.** Rejected because it
  creates a second distribution surface and does not provide version matching.
- **Expose many MCP guidance tools/resources.** Rejected because client schema
  advertisement can create an ambient token cost. One compact index and routed
  reads are sufficient.
- **Generate temporary files in the workspace.** Rejected because it dirties
  customer repositories and complicates provenance and cleanup.
