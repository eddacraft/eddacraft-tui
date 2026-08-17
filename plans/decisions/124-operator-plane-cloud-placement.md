# ADR-124: Operator-plane cloud placement — two-plane scope, portability tiering, and the credit-neutrality rule

## Status

Proposed

## Date

2026-08-17

## Context

eddacraft now holds scaling credits from **Cloudflare** and **AWS** alongside
its existing Azure footprint. [ADR-059](059-production-tracing-sink.md) chose
Azure Monitor + Application Insights as the production tracing sink, and its
decisive vendor argument was stack consolidation:

> **Why Azure over Honeycomb / Grafana Cloud:** eddacraft is already on Azure —
> Front Door edge (ADR-032), Azure DNS, Entra identity, Azure billing/credits.

Three of those four premises have weakened or were never as load-bearing as the
sentence implies:

1. **Billing/credits is no longer Azure-exclusive.** Cloudflare and AWS credits
   make "no new vendor relationship" a weaker argument than "no new *paid*
   vendor relationship" was in May.
2. **The Azure footprint in IaC is thinner than the ADR implies.** `infra/`
   provisions `azure.dns.RecordSet`, `azure.keyvault`, `azure.codesigning`
   (`CodeSigningAccount` + `CertificateProfile`), and `azure.resources.ResourceGroup`
   — plus `infra/src/vercel.ts` for compute. **Azure Front Door and the Log
   Analytics workspace described in [ADR-032](032-edge-architecture-afd.md) are
   not in Pulumi at all.**
3. **Compute was never on Azure.** ADR-059 already recorded cross-cloud egress
   from Vercel to Azure as an accepted negative; ADR-032 anticipated
   "Azure-hosted within 8 weeks" from 2026-04, which has not happened.

Meanwhile the operator plane has accreted two significant vendors with **no ADR
at all**:

- **Neon** (`@neondatabase/serverless`, `apps/anvil-api/src/db/client.ts`) holds
  every account-plane table — `waitlist`, `beta_users`, `access_tokens`,
  `refresh_tokens`, `otp_codes`, `device_codes`, `github_device_sessions`,
  `admin_keys`, `audit_log`, `telemetry_beacons`, `activity_rollup_daily`.
  This is the entire customer and auth record.
- **Resend** (`apps/anvil-api/src/lib/email.ts`, `resend-credentials.ts`) is the
  sole transactional email path — waitlist, invitations, OTP, admin broadcast.

So documented architecture and deployed reality have diverged in both
directions: ADR'd things that do not exist (AFD, Log Analytics), and existing
things that were never ADR'd (Neon, Resend). A decision is needed now because
credits create real pressure to move workloads, and moving them without a
placement rule converts a funding opportunity into permanent lock-in.

### The distinction the vendor question keeps obscuring

Anvil has **two planes**, and only one of them is affected by any of this.

**The product plane** is the shipped `anvil` binary. It is planless-first
([ADR-001](001-planless-first.md)), git-native for durable governance evidence
([ADR-072](072-git-native-governance-substrate.md)), Kindling-of-record
([ADR-116](116-kindling-product-profiles-and-governance-record.md)), and
air-gap-capable — [ADR-089](089-fp-telemetry-destination.md) holds false-positive
telemetry to **zero network calls**, and ADR-072 states plainly that "Cloud is an
optional amplifier never the source of truth." ADR-059 already applied this at
the tracing layer: the Rust CLI and daemon are formatter-only and **never
auto-export**.

**The operator plane** is eddacraft's own business infrastructure: `apps/anvil-api`
on Vercel, Neon, Resend, Azure DNS / Key Vault / code signing, and the tracing
sink. Its data is eddacraft's operational and commercial record, not customer
governance evidence.

Cloud placement is an **operator-plane decision only**. No cloud choice in this
ADR may create a product-plane dependency; if a proposed placement would, that
alone disqualifies it.

## Decision

Adopt a **portability-tiered operator-plane placement model** with a
credit-neutrality rule, in five parts.

### 1. The two-plane boundary is normative

The product plane takes no runtime dependency on any operator-plane vendor.
Existing invariants that enforce this — ADR-072 (git-native evidence),
ADR-089 (zero-egress FP telemetry), ADR-116 (Kindling-of-record), and ADR-059
§Decision-2 (local-first surfaces never auto-export) — remain in force
unchanged and are **not** reopened by this ADR. Any future placement proposal
that would put a cloud vendor on the product plane's critical path requires its
own superseding ADR against those, not a placement amendment.

### 2. Every operator-plane dependency is assigned a portability tier

| Tier | Meaning | Switching cost | Current members |
| --- | --- | --- | --- |
| **P0 — Portable** | Standard protocol, config-level swap | Days | Tracing sink (OTLP), DNS |
| **P1 — Substitutable** | Standard interface, vendor-specific glue | Weeks | Postgres (Neon), transactional email (Resend), object/edge caching |
| **P2 — Sticky** | Vendor-specific identity, certificates, or framework coupling | Months, or not at all | Code signing (Azure Trusted Signing), CI/CD identity, compute framework coupling (Vercel-specific runtime) |

Tier assignment is recorded here and reviewed when a dependency is added.
**Adding an operator-plane dependency without assigning it a tier is the
condition this ADR exists to prevent.**

### 3. The credit-neutrality rule

> **Credits may determine placement within P0 and P1. Credits may not, on their
> own, justify a P2 placement.**

Credits expire; migrations do not. A P0/P1 workload placed on a credit-bearing
cloud can be moved when the credits run out, so "it is free this year" is a
sufficient reason. A P2 workload placed the same way becomes a permanent
commitment purchased with temporary money — the failure mode this rule forbids.
A P2 move must be justified on merit that survives the credits going to zero,
and must say so explicitly in its own ADR.

### 4. Tracing sink: vendor selection reopened, architecture preserved

ADR-059 §Decision-1 (**vendor-neutral OTLP instrumentation**) and §Decision-2
(**operator-hosted surfaces only ever export**) are **preserved and
re-affirmed**. They are the reason this reopening is cheap: the sink is P0
precisely because ADR-059 refused to let a vendor SDK into application code.

ADR-059 §Decision-3 (Azure Monitor OTel exporter) and §Decision-4 (App Insights
+ Grafana) are **superseded** by this ADR pending selection under §5. Until a
sink is selected, EXPORT-001 stays Draft and the exporter stays unwired — which
is already the status quo, so this reopening blocks nothing.

Self-hosting is **back in scope** as a candidate class. ADR-059 rejected
self-hosted Tempo/Jaeger on ops burden ("contradicts 'no observability team'"),
and that objection is only partly answered by credits: credits cut hosting cost,
not operational cost. Self-hosted candidates therefore qualify only if they are
**near-zero-ops** — single-container or embedded deployments, no cluster to
operate. A ClickHouse + PostgreSQL cluster is not near-zero-ops and does not
qualify on credits alone.

### 5. Neon and Resend are ratified as the incumbent P1 choices

Both are adopted in production without an ADR. Rather than relitigate working
infrastructure, this ADR **ratifies them as-is** and binds them to their tier:

- **Neon (P1):** permitted, on the condition that the account plane stays on
  **standard PostgreSQL semantics**. Neon-specific features (branching in the
  runtime path, the serverless driver's proprietary surface beyond the
  Postgres wire protocol) must not become load-bearing. `apps/anvil-api/src/db/`
  is the enforcement point.
- **Resend (P1):** permitted. The send path stays behind the existing
  `lib/email.ts` seam so the provider is swappable without touching routes.

Ratifying them here is what makes the tier model real rather than aspirational:
the two largest operator-plane dependencies are now tiered, documented, and
constrained.

### Data-residency boundary

[ADR-032](032-edge-architecture-afd.md) pinned Australia East for user-residency
intent. That intent binds **account-plane and customer data** (the Neon tables
above). It does **not** bind the tracing pipe: per
[ADR-035](035-three-pipe-observability-rule.md) spans are ephemeral debugging
context and never source-of-truth, so a non-AU trace sink is acceptable where a
non-AU account database would not be.

Any placement change that moves account-plane data across a jurisdiction, or
adds a processor of it, requires the sub-processor disclosure to be updated
first. **Note:** no sub-processor list or privacy notice currently exists in
this repository, despite ADR-032 committing to add Azure to one. That gap is
recorded as OQ3 below.

## Rationale

The instinct to re-evaluate is correct, but "which cloud" is the wrong first
question. eddacraft is a small team whose infrastructure is already spread over
four vendors; the durable risk is not being on the wrong cloud, it is having no
rule that governs how the next vendor gets added. Two of the four current
vendors arrived with no decision record at all.

Tiering solves the actual problem. It makes the expensive question ("does this
lock us in?") answerable at adoption time rather than at migration time, and it
lets credits be spent aggressively where they are harmless — which is most
places, because ADR-059's vendor-neutral instrumentation and the standard
Postgres wire protocol already did the portability work.

The two-plane boundary is doing the heaviest lifting. Once it is explicit that
the product plane is local-first, git-native, and air-gap-capable, the cloud
question shrinks to eddacraft's own back office. That is a genuinely reversible,
genuinely commercial decision — exactly the kind credits *should* influence.
Anvil's differentiation does not live there.

### Alternatives Considered

| Option | Pros | Cons |
| --- | --- | --- |
| **Two-plane scope + portability tiering + credit-neutrality (chosen)** | Answers the durable question (how vendors get added) not just the momentary one; lets credits be used aggressively where reversible; ratifies Neon/Resend instead of relitigating them; preserves ADR-059's portable architecture | Does not name a winning cloud today — the vendor calls still need credit terms (OQ1) |
| Pick one cloud and consolidate everything onto it | Simplest mental model; one bill, one identity plane | Cannot be decided without credit terms; forces a P2 migration (code signing) on temporary money — exactly what §3 forbids; ignores that compute has never been on the "consolidated" cloud |
| Keep ADR-059 and change nothing | Zero effort; EXPORT stays specified | Leaves a ratified decision resting on a premise (Azure-exclusive credits) that is no longer true, and leaves Neon/Resend undocumented |
| Multi-cloud by default, no preferred vendor | Maximum negotiating leverage | Multiplies operational surface for a team with no platform engineer; makes identity and secrets materially harder |
| Full migration to the largest credit grant | Maximises credit value | Optimises for a one-off subsidy over switching cost; strands Azure Trusted Signing (P2) with no near-equivalent |

## Consequences

- **Positive:** The next vendor decision has a rule to follow instead of a
  precedent to argue about. Credits become usable immediately for P0/P1
  placement without a per-workload ADR. Neon and Resend stop being undocumented
  single points of failure. ADR-059's genuinely good architectural work is kept
  while its stale vendor premise is retired. The tracing sink can now be chosen
  on merit, with self-hosting (including OTLP-native single-container options)
  legitimately in the running.
- **Negative:** This ADR does not name a cloud, so the placement questions
  remain open until credit terms are known (OQ1). Tier assignment is a
  judgement call with no automated enforcement — a P1 dependency can silently
  drift to P2 through ordinary feature work (the Neon-specific-features risk
  §5 names). ADR-032's edge decision is left standing but is now visibly
  inconsistent with IaC (OQ2).
- **Risks:** Tiering becomes documentation nobody consults. Credits drive a P1
  choice that quietly acquires P2 characteristics. The sub-processor gap (OQ3)
  becomes a compliance problem before it becomes a documentation one.
- **Mitigations:** The tier table lives in this ADR and is cited by any new
  operator-plane dependency; §5's enforcement points (`apps/anvil-api/src/db/`,
  `lib/email.ts`) are concrete files a reviewer can check. OQ1–OQ3 are tracked
  as named open questions with owners rather than left implicit.

## Open Questions

- **OQ1 — Credit terms.** Amount, expiry, and covered services for the
  Cloudflare and AWS grants. Blocks the compute, edge, and sink selections. A
  grant covering managed observability changes the sink shortlist; one covering
  only compute does not.
- **OQ2 — ADR-032 reconciliation.** Azure Front Door and the Log Analytics
  workspace are ADR'd but absent from Pulumi. Either the edge decision is
  executed, or ADR-032 is amended to match reality. This ADR does not decide
  which; it records that the divergence exists. Cloudflare credits make this
  live, since Cloudflare's edge is the natural substitute if AFD is abandoned.
- **OQ3 — Sub-processor disclosure.** No privacy notice or sub-processor list
  exists in this repository. Neon, Resend, Vercel, and Azure are all processing
  operator-plane data today. This must be resolved before any placement change
  adds a fifth.

## References

- Related ADRs: [ADR-059](059-production-tracing-sink.md) (§Decision-3/4
  superseded here; §Decision-1/2 preserved),
  [ADR-032](032-edge-architecture-afd.md) (edge + AU residency intent; see OQ2),
  [ADR-035](035-three-pipe-observability-rule.md) (three-pipe rule — why the
  tracing pipe carries a lighter residency burden),
  [ADR-072](072-git-native-governance-substrate.md) and
  [ADR-089](089-fp-telemetry-destination.md) and
  [ADR-116](116-kindling-product-profiles-and-governance-record.md)
  (product-plane invariants this ADR must not disturb),
  [ADR-001](001-planless-first.md) (local-first posture),
  [ADR-066](066-github-device-flow-cli-auth.md) (Key Vault + Vercel serverless
  session storage — an operator-plane dependency inherited here)
- APS modules: [EXPORT-001](../modules/observability-export.aps.md) (stays Draft;
  its sink premise is reopened by §4),
  [TRACE-002 / TRACE-003](../modules/tracing-foundation.aps.md) (unchanged —
  the redaction layer must wrap whichever exporter is selected)
- Code: `infra/src/` (Pulumi — current Azure + Vercel footprint),
  `apps/anvil-api/src/db/client.ts` (Neon), `apps/anvil-api/src/lib/email.ts`
  (Resend)
