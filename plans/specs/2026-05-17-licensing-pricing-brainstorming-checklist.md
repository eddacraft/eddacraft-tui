# Licensing and Pricing Brainstorming Checklist

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Advisory | Product strategy | Draft | Created 2026-05-17 against `plans/index.aps.md` and existing product licensing plan |

| Upstream | Downstream |
| -------- | ---------- |
| [`plans/index.aps.md`](../index.aps.md), [`plans/specs/2026-03-12-product-licensing.md`](./2026-03-12-product-licensing.md), [`docs/archive/specs/2026-03-12-product-licensing-design.md`](../../docs/archive/specs/2026-03-12-product-licensing-design.md) | Future licensing, packaging, pricing, and go-to-market decisions |

This checklist is prep material for a brainstorming session. It is not a
pricing decision, licence recommendation, or implementation plan.

## Session Goal

- [ ] Decide what question the session must answer: licensing model, pricing
      model, packaging, or validation plan.
- [ ] Identify which decisions must be made now and which can stay as future
      options.
- [ ] Capture explicit anti-goals, especially adoption friction, legal
      complexity, and enterprise procurement burden.
- [ ] Define the output format for the session: decision matrix, shortlist,
      experiment plan, or follow-up APS work.

## Product Context

- [ ] Summarise Anvil's current product thesis and primary beneficiary from
      `plans/index.aps.md`.
- [ ] List current and planned deployment modes: local CLI, editor integration,
      daemon, self-hosted services, hosted services, or hybrid.
- [ ] Map which capabilities are individual-developer value, team value, and
      organisation value.
- [ ] Identify capabilities that may become paid packaging boundaries, such as
      team policy governance, dashboards, compliance evidence, support, or hosted
      coordination.
- [ ] Separate shipped behaviour from future ideas so pricing does not depend on
      unbuilt features.

## Existing Licensing Context

- [ ] Review the existing product licensing implementation plan for assumptions
      about offline entitlement, licence blobs, tiers, scopes, seats, and refresh
      windows.
- [ ] Review the current repository licence and distribution assumptions.
- [ ] Inventory dependency licences that could constrain commercial packaging.
- [ ] Identify whether contributor licence, DCO, trademark, or redistribution
      rules need legal review.
- [ ] Note any terminology differences between `licence` in implementation docs
      and `license` in external market language.

## Market Research

- [ ] Collect pricing pages from comparable developer tools.
- [ ] Collect licensing models from adjacent open-source, open-core,
      source-available, and commercial developer tools.
- [ ] Capture how competitors split free, pro, team, and enterprise tiers.
- [ ] Capture common conversion triggers: seats, repositories, private projects,
      policy packs, hosted sync, audit logs, SSO, support, or usage volume.
- [ ] Note whether competitors monetise self-hosted deployments, hosted service,
      enterprise controls, support, or usage.

## Customer And Buyer Inputs

- [ ] Define likely user personas: solo developer, AI-heavy engineer, team lead,
      platform engineer, security reviewer, engineering executive.
- [ ] Define likely buyer personas: founder, engineering manager, platform lead,
      security leader, procurement owner.
- [ ] Capture the value each persona receives and what budget it maps to.
- [ ] Identify who feels the pain of architecture drift and who pays to reduce
      it.
- [ ] List objections each persona may raise about pricing, privacy, local-only
      use, or self-hosting.

## Cost And Operational Inputs

- [ ] Estimate hosted infrastructure costs if a SaaS component is offered.
- [ ] Estimate support burden by segment: community, pro, team, enterprise.
- [ ] Identify third-party API, compute, storage, or distribution costs.
- [ ] Estimate onboarding and sales effort for larger customers.
- [ ] Identify operational commitments that should only appear in paid tiers,
      such as SLAs, priority support, or compliance evidence.

## Licensing Model Options

- [ ] Evaluate permissive open-source core plus paid hosted service.
- [ ] Evaluate open-core with paid enterprise features.
- [ ] Evaluate source-available with commercial terms.
- [ ] Evaluate dual licensing.
- [ ] Evaluate support-only or services-led monetisation.
- [ ] Evaluate whether AGPL, BSL, Apache-2.0, MIT, or a custom commercial licence
      align with adoption and defensibility goals.

## Pricing Model Options

- [ ] Evaluate per-seat pricing.
- [ ] Evaluate per-team or per-workspace pricing.
- [ ] Evaluate usage-based pricing tied to repositories, scans, policies,
      commits, or agents.
- [ ] Evaluate enterprise annual contracts.
- [ ] Evaluate free community, paid pro, team, and enterprise tiers.
- [ ] Evaluate whether support, SLA, onboarding, or compliance packages should be
      separate add-ons.

## Packaging Boundaries

- [ ] Draft a free tier boundary that maximises adoption.
- [ ] Draft a pro tier boundary that creates individual willingness to pay.
- [ ] Draft a team tier boundary that maps to collaboration and governance.
- [ ] Draft an enterprise tier boundary that maps to procurement, security, and
      scale.
- [ ] Identify any feature boundary that would create user-hostile friction if
      paywalled too early.

## Decision Criteria

- [ ] Rank adoption friction versus monetisation defensibility.
- [ ] Rank community trust versus enterprise control.
- [ ] Rank self-hosted monetisation versus hosted-service monetisation.
- [ ] Rank pricing simplicity versus value capture.
- [ ] Rank legal simplicity versus protection from direct resale or hosted
      wrapping.
- [ ] Define what evidence would change the preferred model.

## Risks And Open Questions

- [ ] Identify legal risks that require counsel before publication.
- [ ] Identify product risks from promising features before they exist.
- [ ] Identify community trust risks from licence changes or source-available
      positioning.
- [ ] Identify operational risks from enterprise commitments.
- [ ] Identify pricing risks from charging the wrong buyer or metric.

## Session Outputs

- [ ] Produce a licensing option matrix with trade-offs.
- [ ] Produce a pricing option matrix with trade-offs.
- [ ] Produce a feature packaging map across free, pro, team, and enterprise.
- [ ] Produce a list of validation questions for customer discovery.
- [ ] Produce follow-up actions, owners, and decision deadlines.
