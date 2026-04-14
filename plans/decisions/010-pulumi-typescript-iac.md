# ADR-007: Pulumi (TypeScript-first Infrastructure as Code)

## Status

Accepted

## Date

2026-02-03

## Context

This isn't "why Pulumi is cool". This is why Pulumi is the right tool given
eddacraft's constraints, goals, and posture.

### The Problem We Are Actually Solving

We are not "provisioning cloud resources".

We are:

- Building repeatable, governed environments
- Enforcing policy and intent, not just syntax
- Enabling humans + AI agents to collaborate safely on infrastructure
- Reusing existing Terraform assets without inheriting Terraform as a lifestyle

This immediately disqualifies tools that optimise for static configuration
authoring over system design.

### Options Considered

#### Terraform / OpenTofu (direct)

- :x: Primary authoring language (HCL) not aligned with decision-makers
- :x: Abstraction via convention, not structure
- :x: Higher cognitive load for AI-assisted changes
- :white_check_mark: Excellent ecosystem (retained via Pulumi)

**Verdict:** retained as an ecosystem, rejected as a primary interface.

#### CDK for Terraform (CDKTF)

- :white_check_mark: TypeScript authoring
- :x: Still bound to Terraform's execution and mental model
- :x: Two layers of abstraction to reason about
- :x: Harder to explain and govern cleanly

**Verdict:** transitional tool, not a foundation.

#### Crossplane

- :white_check_mark: Strong reconciliation and control-plane model
- :x: Kubernetes-first assumption
- :x: Heavier operational footprint
- :x: Overkill for current stage

**Verdict:** revisit later if eddacraft becomes Kubernetes-as-platform-first.

#### Pulumi (TypeScript)

- :white_check_mark: Real language authoring with full IDE support
- :white_check_mark: First-class abstractions and composition
- :white_check_mark: Policy-as-code on semantic resource graphs
- :white_check_mark: Consumes Terraform provider ecosystem
- :white_check_mark: AI/automation-friendly deterministic previews

**Verdict:** selected.

## Decision

**eddacraft will adopt Pulumi (TypeScript) as its primary Infrastructure as
Code framework.**

This decision prioritises authoring ergonomics, composability, governance, and
AI-assisted workflows, while preserving access to the existing Terraform
provider ecosystem. Pulumi enables eddacraft to treat infrastructure as a
governed platform capability rather than a collection of static configuration
files, aligning directly with Anvil's validation and policy-first philosophy.

## Rationale

### 1. Authoring Experience (High Weight)

The primary decision-makers and system designers must be able to read, reason
about, and change IaC directly.

HCL is optimised for declaration, not abstraction or composition.

We want:

- Real control flow
- Refactoring safety
- Tests
- Shared libraries
- IDE support that actually understands intent

Pulumi wins because it uses real languages (TypeScript for us).

### 2. Abstraction & Composition (High Weight)

We are not shipping raw resources. We are shipping platform capabilities.

Pulumi enables:

- First-class abstractions (components as code, not conventions)
- Encapsulation of unsafe primitives
- Opinionated "golden paths" enforced structurally, not socially

Terraform can do this, but only by leaning heavily on module conventions and
discipline. Pulumi makes it the default.

### 3. Governance & Policy Integration (High Weight)

eddacraft / Anvil is fundamentally about watching, validating, and enforcing.

Pulumi supports:

- Policy-as-code that operates on semantic resource graphs, not just text diffs
- Enforcement at preview time (before anything touches cloud APIs)
- Deterministic evaluation that fits cleanly into CI and agent workflows

This maps directly to Anvil-style plan validation.

### 4. Terraform Ecosystem Leverage (Medium Weight)

We already have access to Terraform code.

Pulumi:

- Consumes the same provider ecosystem
- Allows incremental migration
- Avoids a "big bang" rewrite
- Lets Terraform exist as input, not the centre of gravity

This preserves optionality without forcing HCL literacy.

### 5. AI & Automation Friendliness (Medium-High Weight)

We expect:

- AI agents to read, propose, and modify infrastructure code
- Deterministic diffs
- Predictable previews
- Composable primitives

TypeScript + Pulumi gives agents:

- Richer context
- Explicit control flow
- Fewer implicit behaviours than HCL interpolation magic

This matters more over time, not less.

### 6. Commercial & Licensing Posture (Medium Weight)

We want:

- No unnecessary licensing landmines
- Flexibility in how eddacraft evolves commercially
- Clean separation between tooling and product IP

Pulumi's model is predictable and less entangled with provider lock-in or sudden
licence shifts compared to Terraform's recent history.

## What Pulumi Gives eddacraft Specifically

### Structurally Enforceable Guardrails

Not: "Please don't create public S3 buckets"

But: "There is no exported constructor that allows you to do that."

### A Real Platform Layer

- `Environment`
- `Application`
- `DataPlane`
- `IdentityBoundary`

These become code constructs, not wiki pages.

### Clean Integration with Anvil

- Previews = deterministic plans
- Policies = enforceable rules
- Diffs = auditable artefacts
- Stacks = explicit intent

Pulumi doesn't just work with governance — it expects it.

## Consequences

### Positive

- TypeScript authoring gives decision-makers direct access to IaC
- First-class component model enables platform-as-code
- Policy enforcement at preview time aligns with Anvil's philosophy
- Full Terraform provider ecosystem available without HCL
- AI agents get richer context and explicit control flow
- Predictable commercial/licensing posture

### Negative

- Team members who expect Terraform must adapt to a new interface
- "Real language" IaC can drift toward software engineering overhead
- Pulumi is a less common choice; smaller community than Terraform

### Mitigations

| Risk                                                | Mitigation                                                                                                                            |
| --------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| "Pulumi is too powerful; people can shoot themselves in the foot" | No raw resources in application stacks. Everything goes through eddacraft-owned component libraries. Policy enforcement at preview time. |
| "Team members expect Terraform"                     | Terraform knowledge remains valuable at the provider/resource level. Existing modules can be wrapped or migrated incrementally. Clear messaging: Terraform ecosystem, Pulumi interface. |
| "Language IaC becomes software engineering overhead" | Strict library boundaries. Opinionated patterns. Minimal surface area exposed to consumers. Treat infra code like platform code, not app code. |

## References

- [Pulumi](https://www.pulumi.com/)
- [Pulumi TypeScript SDK](https://www.pulumi.com/docs/languages-sdks/javascript/)
- [Pulumi Policy as Code (CrossGuard)](https://www.pulumi.com/docs/using-pulumi/crossguard/)
- [Pulumi Terraform Provider Bridge](https://github.com/pulumi/pulumi-terraform-bridge)
