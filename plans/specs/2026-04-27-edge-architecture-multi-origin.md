# Edge Architecture: Multi-Origin Federation via Azure Front Door

| Field          | Value                                                                                                                                                                                             |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Status         | Approved post-council (revision 2)                                                                                                                                                                |
| Original draft | 2026-04-27                                                                                                                                                                                        |
| Revised        | 2026-04-28 with council outcomes from session `plan-286b981a`                                                                                                                                     |
| Author         | orchestrator (initial draft); revised after 5-persona council interrogation                                                                                                                       |
| Scope          | Network topology, edge layer, DNS, cross-cutting HTTP concerns, Pulumi IaC layout, and migration plan. Anvil-side application code limited to `/health` endpoints and (deferred) FDID middleware. |
| Decision class | Architectural — ratified as **ADR-032**                                                                                                                                                           |
| Module         | `plans/modules/edge.aps.md`                                                                                                                                                                       |

---

## 1. Executive Summary

Anvil's web surfaces (marketing site, anvil-api, docs apps) are deployed on
Vercel. A second non-Vercel origin (Azure-hosted) is committed within 8 weeks —
validated by the planning council as the load-bearing premise for this
architecture.

**Decision:** Use Azure Front Door Standard as the always-in-front edge layer.
Azure DNS points at AFD; AFD routes to Vercel today and the Azure-hosted origin
within 8 weeks. WAF, the www→apex redirect, TLS termination, observability,
**static-asset caching** (to mitigate Vercel egress at scale), and rate-limiting
all live at AFD.

**Deferred from the original draft, per council:** FDID origin-auth header
enforcement (`x-azure-fdid`). Application-side complexity rejected as not worth
the rotation-incident class for a hype-launch beta with no paying customers.
Documented revisit triggers in ADR-032 §Decision-3.

**Material changes from the pre-council draft (16 decisions):**

1. Multi-origin premise validated (concrete <8 weeks)
2. Capacity confirmed (≥5 focused IaC days pre-launch)
3. Cost: Azure credits cover; Vercel egress capped via AFD-cache-static + alerts
4. Phase 4 named commitment (~2 weeks post-launch); may slip v0.4.2 release
   window
5. Rollback strategy = canary drill (EDGE-018), not aspirational 60s
6. AFD as single edge accepted; TTLs kept low post-cutover; manual repoint
   runbook
7. **Alerts move from Phase 4 to Phase 2** — Phase 3 is gated on alerts being
   live
8. **AFD caches static assets** (was: passthrough); Vercel owns dynamic / ISR
9. **FDID origin auth deferred** — drop EDGE-019/020 from MVP
10. WAF detection-mode through Phase 3, prevention in Phase 4
11. Rate limits: detection-mode calibration window first, then prevention
    thresholds at p99 + margin
12. Cert defence in depth: PR-time test + Pulumi `protect` flag + Azure Monitor
    alert + external synthetic monitor
13. CORS hybrid: AFD coarse host-allowlist + origin per-route logic
14. Log Analytics workspace doesn't exist yet → Phase 2 provisions it (folds
    into pending OBS work)
15. Synthetic `/health` endpoint required on each Vercel origin (consumed by
    probes + smoke checks)
16. Compliance minimum-viable: 1y log retention, RBAC scoping, WAF rule changes
    via PR

## 2. Context

### 2.1 Current state (verified)

DNS authority: Azure DNS, zone `eddacraft.ai`, partly under Pulumi management at
`infra/src/dns/eddacraft-ai.ts`.

| Record                      | Type      | Target                                     | Pulumi-managed?                   |
| --------------------------- | --------- | ------------------------------------------ | --------------------------------- |
| `@`                         | TXT       | SPF (Google)                               | Yes                               |
| `_dmarc`                    | TXT       | DMARC `p=none`                             | Yes                               |
| `api`                       | CNAME     | `cname.vercel-dns.com`                     | Yes                               |
| `install`                   | CNAME     | `eddacraft.github.io`                      | Yes                               |
| `resend._domainkey.updates` | TXT       | DKIM                                       | Yes                               |
| `send.updates`              | MX, TXT   | Resend bounce                              | Yes                               |
| **`@`**                     | **A**     | **`216.150.1.1` (Vercel)**                 | **NO — drift, fixed in EDGE-001** |
| **`www`**                   | **CNAME** | **`6c3bec1fd4ec9127.vercel-dns-017.com.`** | **NO — drift, fixed in EDGE-002** |

### 2.2 Premise (council-validated)

The user has a concrete <8-week commit to a second Azure-hosted origin (workload
TBD; captured as a Ready-checklist item in `edge.aps.md`). This is the
load-bearing fact that justifies AFD now versus deferring to a Vercel-native
fallback.

If the multi-origin commit slipped to >6 months out, the §11.1 fallback
(Vercel-native www→apex redirect, ~30 min of work, zero OPEX) would be the
correct call. The council surfaced this explicitly; the user's answer moved the
architecture from "speculative" to "correctly-sequenced".

### 2.3 Goals

- **Origin-independent edge** — routing/redirect/security/cert layer works
  identically regardless of where the origin lives.
- **One source of truth** — Pulumi owns every DNS record, every domain
  attachment, every redirect rule, every WAF rule.
- **Centralised cross-cutting concerns** — TLS, redirects, CORS (coarse),
  rate-limiting, WAF, static-asset caching, observability all live at AFD.
- **Multi-origin from day one** — Phase 2's IaC accommodates the 8-week Azure
  origin without DNS or cert changes.
- **No live-traffic outage during migration** — phased rollout with measured
  rollback per phase (canary drill in EDGE-018 establishes the real timing).
- **Vercel egress controlled** — AFD caches static assets to reduce Vercel
  origin pull at scale.

### 2.4 Non-goals

- Replacing Vercel as origin for marketing site, anvil-api, or docs apps
- Migrating off Azure DNS
- Application-layer features (auth, sessions) at the edge
- AFD Premium features (deferred)
- FDID origin auth (deferred per ADR-032 §Decision-3)
- Formal SOC 2 audit walkthrough (deferred until first SOC 2 customer)

## 3. Decision Summary (per ADR-032)

> Full decision rationale lives in
> `plans/decisions/032-edge-architecture-afd.md`. This section summarises for
> design-doc context.

- **AFD Standard, not Premium.** 5 custom rules sufficient for v0.4.x.
- **AFD profile pinned to Australia East.** Microsoft Azure added to privacy
  notice / DPA as sub-processor.
- **Cert defence in depth.** Four layers: PR-time test, Pulumi protect flag,
  Azure Monitor alert, external synthetic monitor.
- **FDID origin auth deferred.** Three failure modes (rotation outage, bypass
  via discoverable domains, preview-deploy leak) outweigh marginal
  defence-in-depth benefit at beta scale. Revisit on documented triggers.
- **No AFD-conditional application code.** Design constraint: CORS / HSTS /
  rate-limits live at infrastructure only. Eliminates Vercel-preview /
  production parity bugs by construction.
- **Phased migration with named gates.** Phase 0+1: zero-cost hygiene. Phase 2:
  AFD up but no traffic shift, alerts active, canary drill done. Phase 3:
  per-hostname cutover, apex last. Phase 4: WAF prevention, rate-limit
  calibration, ADR finalisation, runbooks.

## 4. Target Architecture

### 4.1 Network topology

```
                    ┌──────────────────────────────────┐
                    │            Browser               │
                    └───────────────┬──────────────────┘
                                    │
                                    │ DNS query
                                    ▼
        ┌───────────────────────────────────────────────────┐
        │         Azure DNS — eddacraft.ai zone             │
        │         (Pulumi-managed: dns/eddacraft-ai.ts)     │
        │                                                    │
        │  apex   ALIAS → AFD endpoint                      │
        │  www    ALIAS → AFD endpoint   (308 → apex)       │
        │  api    CNAME → AFD endpoint                      │
        │  docs   CNAME → AFD endpoint                      │
        │  install CNAME → eddacraft.github.io  (unchanged) │
        │  TXT/MX/DKIM records                  (unchanged) │
        │  _dnsauth.* TXT (cert validation, Pulumi-protect) │
        └───────────────────────────┬───────────────────────┘
                                    │ HTTPS
                                    ▼
        ┌───────────────────────────────────────────────────┐
        │   Azure Front Door Standard (Australia East)     │
        │       (Pulumi-managed: edge/front-door.ts)       │
        │                                                    │
        │   ─ TLS termination (AFD-managed certs)          │
        │   ─ Custom domain bindings (apex, www, api, docs)│
        │   ─ WAF policy (MS Default Rule Set)             │
        │       Phase 2-3: detection mode                  │
        │       Phase 4: prevention mode (after FP soak)   │
        │   ─ Rules engine:                                 │
        │       • www  → 308 redirect to apex              │
        │       • apex/www  → website origin group         │
        │       • api.*     → anvil-api origin group       │
        │       • docs.*    → docs-shell origin group      │
        │       • shared response headers (HSTS, etc.)     │
        │   ─ Caching:                                      │
        │       • /_next/static/*, images, css, js: AFD    │
        │       • dynamic / ISR routes: passthrough        │
        │   ─ Diagnostic logs → Log Analytics              │
        │                                                    │
        │   ─ Origin auth: DEFERRED (ADR-032 §Decision-3)  │
        └────────────┬────────────┬────────────────────────┘
                     │            │
        ┌────────────┴────┐   ┌───┴────────────────────────┐
        │ Vercel origins  │   │  Azure-hosted origin       │
        │  ─ website      │   │  (8-week commit, exact     │
        │  ─ anvil-api    │   │   workload TBD; plugs in   │
        │  ─ docs-shell   │   │   as origin group + route  │
        │  ─ docs-private │   │   without DNS or cert work)│
        │                 │   │                            │
        │  /health        │   │  /health                   │
        └─────────────────┘   └────────────────────────────┘
```

### 4.2 Cross-cutting concern map (post-council)

| Concern                 | Today (drift)                  | Target                                              | Phase                         |
| ----------------------- | ------------------------------ | --------------------------------------------------- | ----------------------------- |
| TLS termination         | Vercel per-project             | AFD (managed certs)                                 | 2                             |
| Domain → backend        | Vercel `ProjectDomain`         | AFD route + origin group                            | 2                             |
| www → apex redirect     | None (both serve direct)       | AFD rules engine, 308                               | 2                             |
| HSTS + security headers | Inconsistent                   | AFD response-header rule                            | 4                             |
| WAF                     | None                           | AFD WAF policy (detection → prevention)             | 2 (detection); 4 (prevention) |
| Rate limiting           | None                           | AFD rules engine (detection-calibrate → prevention) | 4                             |
| CORS allow-list         | API env var only               | **Hybrid**: AFD coarse host-list + origin per-route | 4                             |
| Static-asset cache      | Vercel                         | AFD edge                                            | 2                             |
| Dynamic / ISR cache     | Vercel                         | Vercel (AFD passthrough)                            | 2                             |
| DDoS                    | Vercel + (none for non-Vercel) | AFD + per-origin                                    | 2                             |
| Observability           | Vercel logs                    | Log Analytics workspace                             | 2 (workspace + alerts)        |
| Origin auth             | None                           | **DEFERRED**                                        | n/a                           |

## 5. IaC Layout (post-council)

### 5.1 Pulumi components

Per ADR-032 §Decision-7, the AFD layer is a **single** ComponentResource (was
three in pre-council draft). Refactor only if it grows past ~400 LOC.

```
infra/src/
├── components/
│   ├── dns-zone.ts              (existing — extend for ALIAS records)
│   ├── vercel-app.ts            (existing — minor changes for /health envs)
│   └── front-door.ts            (NEW — single FrontDoor component)
├── edge/
│   ├── front-door.ts            (NEW — composition: profile, WAF, custom domains)
│   ├── origins.ts               (NEW — origin groups + probes)
│   ├── routes.ts                (NEW — host/path → origin group + cache rules)
│   ├── alerts.ts                (NEW — Phase 2 alerts)
│   └── rate-limits.ts           (NEW — Phase 4 rate-limit rules)
├── observability/
│   └── log-analytics.ts         (NEW — workspace, retention 365d)
├── dns/
│   └── eddacraft-ai.ts          (REFACTOR — apex/www/api/docs ALIAS to AFD; _dnsauth.* records added; protect: true)
└── vercel.ts                    (MODIFY — Vercel projects keep domains for verification; /health envs)
```

### 5.2 FrontDoor component sketch

```ts
interface FrontDoorArgs {
  resourceGroupName: pulumi.Input<string>;
  region: 'australiaeast' | 'eastus' | ...;
  customDomains: { hostname: string; certificate: 'managed' }[];
  originGroups: {
    name: string;
    origins: { hostname: pulumi.Input<string>; priority: number; weight: number }[];
    healthProbe: { path: '/health'; intervalSeconds: 30; successThreshold: 2 };
  }[];
  routes: {
    name: string;
    customDomain: string;
    originGroup: string;
    patternsToMatch: string[];
    cacheConfig: 'static' | 'disabled';
    rulesEngine?: 'www-to-apex-308' | ...;
  }[];
  wafMode: 'detection' | 'prevention';
  diagnosticWorkspaceId: pulumi.Input<string>;
}

class FrontDoor extends pulumi.ComponentResource {
  endpoint: AfdEndpoint;
  profileId: pulumi.Output<string>;
  fdid: pulumi.Output<string>;     // emitted but NOT enforced (deferred)
  domains: AfdCustomDomain[];
  wafPolicy: WafPolicy;
}
```

### 5.3 Existing component changes

**`vercel-app.ts`:**

- Domains stay attached to Vercel projects so AFD-routed requests can be served
  (Vercel pattern-matches Host header).
- `/health` endpoint requirement documented in component args.
- **No** FDID env var (deferred per ADR-032 §Decision-3).

**`dns-zone.ts`:**

- Add `aliasRecord` support for ALIAS-to-AFD entries.
- Existing CNAME / TXT / MX support unchanged.

### 5.4 Stack outputs (new + changed)

```
front-door:
  endpoint: <profile>-<hash>.z01.azurefd.net      (NEW)
  fdid: <uuid>                                      (NEW — emitted, not enforced yet)
  custom-domains: [eddacraft.ai, www.eddacraft.ai, api.eddacraft.ai, docs.eddacraft.ai]

dns:
  apex-target: alias to front-door.endpoint        (CHANGED — was Vercel A)
  www-target:  alias to front-door.endpoint        (CHANGED — was Vercel CNAME)

observability:
  log-analytics-workspace-id: <id>                  (NEW)

vercel:
  health-endpoints: [api/health, /health]           (NEW)
```

## 6. Migration Plan (24 work items, 5 phases)

Detailed work-item-level plan is in `plans/modules/edge.aps.md`. Summary here
for cross-reference.

### Phase 0 — DNS reconciliation (3 items, ~half-day)

EDGE-001..003. Pulumi-import drift records (apex A, www CNAME) + dns.test.ts
assertions. Zero behavioural change.

### Phase 1 — Vercel domain IaC sync (3 items, ~half-day)

EDGE-004..006. Sync `infra/src/vercel.ts` for dual website domains; add www to
anvil-api CORS allow-list on the dev line; assert in tests.

### Phase 2 — AFD provisioning (12 items, ~3-4 days)

EDGE-007..018. The substantive infrastructure step:

- Workspace (EDGE-007) — provisioned because it doesn't exist yet
- FrontDoor component + instantiation (EDGE-008/009) — single component
- Origin groups + routes + caching (EDGE-010) — static-cache config
- `_dnsauth` TXT records under Pulumi protect (EDGE-011) + tests (EDGE-012)
- `/health` endpoints on anvil-api + website (EDGE-013/014) — single observable
  signal for probes AND smoke checks
- Probe configuration (EDGE-015)
- **Phase 2 alerts** (EDGE-016) — moved here from Phase 4 per council; Phase 3
  cuts gated on alerts being live
- Cert expiry alerts + external synthetic monitor (EDGE-017)
- **Canary rollback drill** (EDGE-018) — gates Phase 3; publishes measured
  rollback timing in `edge-cutover.md`

### Phase 3 — Per-hostname DNS cutover (4 items, ~3 days with soak)

EDGE-019..022. Cutover order reflects user's "must stay up" surfaces:

1. EDGE-019: api (validates route + WAF logging)
2. EDGE-020: docs (validates proxy path)
3. EDGE-021: www (verifies 308 redirect rule)
4. EDGE-022: apex (highest visibility, most-validated by then)

24h+ green soak between each. Alerts active throughout.

### Phase 4 — Centralise concerns (2 items, ~1-2 weeks post-launch)

EDGE-023..024. Post-launch hardening:

- WAF prevention mode (after rate-limit calibration window of 2 weeks)
- Rate-limit thresholds at p99 + margin (data-driven, not guesses)
- HSTS + security response headers
- ADR-032 cross-link in DECISION-LOG.md
- Three new runbooks: `edge-cutover.md`, `edge-observability.md`,
  `edge-incident-response.md`
- Post-deploy smoke check updated for edge concerns

May slip v0.4.2 window — acceptable per council.

## 7. Cost Analysis

### 7.1 AFD Standard pricing components

- Base fee: $35/month
- Outbound data transfer (Australia, zone 1): $0.225/GB first 10TB
- Routing rule executions: $0.60 per million
- WAF requests: $0.60 per million
- WAF MS Default Rule Set: ~$25/month flat
- Log Analytics ingestion: ~$2.76/GB

### 7.2 Realistic monthly bill projections

**At beta scale (10K req/day, 50GB/mo egress):**

- AFD base + egress + WAF DRS: ~$71/mo
- Log Analytics ingestion (~5GB/mo): ~$14/mo
- **Total: ~$85/mo**

**At hype-launch scale (200K req/day, 1TB/mo egress):**

- AFD: ~$292/mo
- Log Analytics (~50GB/mo): ~$140/mo
- **Total: ~$432/mo**
- **Note:** Vercel egress is paid SEPARATELY. Vercel Pro includes 1TB; over that
  is $0.40/GB. AFD-cache-static reduces Vercel origin pull at the edge —
  material at this scale.

**At 3x hype-launch worst case (3TB/mo egress total):**

- AFD: ~$835/mo
- Vercel overage (2TB at $0.40/GB): ~$800/mo
- Log Analytics: ~$420/mo
- **Total worst case: ~$2,055/mo**

### 7.3 Cost guardrails (council Decision-Cost-guardrails)

- Vercel usage alert at 80% of included quota (1TB Pro)
- Azure cost alert at 150% of monthly projection
- AFD-cache-static reduces Vercel origin pull (already in EDGE-010)
- Aggressive caching of moderately-static API responses is a documented
  trigger-gated mitigation (revisit at $200/mo Vercel overage for 2 months)

User context: Azure side covered by credits until funding closes; the Vercel
overage path is the binding constraint at scale.

## 8. Operational Concerns

### 8.1 Observability

- AFD diagnostic logs → Log Analytics workspace (EDGE-007 provisions)
- Standard log categories: `FrontDoorAccessLog`,
  `FrontDoorWebApplicationFirewallLog`, `FrontDoorHealthProbeLog`
- KQL queries land in `docs/runbooks/edge-observability.md` (EDGE-024)

### 8.2 Alerts (Phase 2 — moved from Phase 4 per council)

Active before any Phase 3 cut:

- 5xx rate > 1% over 5 min → page on-call
- Origin health probe failed for 3 consecutive samples → page

Phase 4 adds:

- WAF block rate > 100/min sustained → notify
- AFD endpoint reachability < 99.9% rolling 1h → page
- Cert expiry < 30/14/7 days → warn / page / escalate

### 8.3 Runbooks (Phase 4 deliverables)

- `docs/runbooks/edge-cutover.md` — measured rollback timing from canary drill
  (EDGE-018), per-hostname cutover procedure
- `docs/runbooks/edge-observability.md` — KQL queries + Log Analytics pointers
- `docs/runbooks/edge-incident-response.md` — origin failure, WAF false
  positive, cert expiry, AFD endpoint outage scenarios; manual DNS repoint to
  Vercel-direct as canonical recovery for AFD-side outages

### 8.4 Staging strategy

Single AFD profile spans dev + prod stacks. Dev gets a separate set of custom
domains (or the AFD-default endpoint) to avoid colliding with prod DNS. WAF rule
changes go via PR with detection-mode validation in dev before flip in prod.

(Original draft proposed dev/prod twin profiles. Council didn't flag this;
revisit if isolation becomes operationally important.)

### 8.5 Vercel preview deployments

Preview URLs (`anvil-api-git-<branch>.vercel.app`) bypass AFD by design. Per
ADR-032 §Decision-4, application code does NOT gate behaviour on AFD-injected
headers, so preview QA reflects production behaviour for all application logic.
Edge-only concerns (HSTS, WAF, rate limits, AFD caching) are tested via
post-deploy smoke checks against the AFD-fronted production endpoints, not
preview deploys.

## 9. Security Posture

### 9.1 What AFD adds

- WAF (managed rule set blocks OWASP top 10)
- DDoS (Azure Standard, free for AFD tenants)
- Rate limiting (Phase 4, calibrated thresholds)
- TLS posture (1.2 minimum; 1.3-only configurable later)
- Security response headers (HSTS, frame-options, content-type-options,
  referrer-policy) — Phase 4
- One cert pipeline simplifies posture review

### 9.2 Cert renewal — defence in depth (council Decision-Cert)

Four layers:

1. PR-time test (`dns.test.ts` asserts `_dnsauth.*` TXT records exist and match
   AFD-expected values) — EDGE-012
2. Pulumi `protect: true` flag on `_dnsauth` resources — EDGE-011
3. Azure Monitor alert on cert expiry < 30/14/7 days — EDGE-017
4. External synthetic monitor (third-party, outside Azure tenancy) — EDGE-017

Eliminates the silent-cert-death class.

### 9.3 What AFD does NOT solve (and why each is acceptable)

- **Application-level auth** — out of scope; lives in `anvil-api` middleware
- **Secrets handling** — Key Vault remains the source
- **DKIM / SPF / DMARC** — DNS-level, separate
- **Bot detection beyond rate limits** — would require AFD Premium with Bot
  Manager; deferred (trigger: bot traffic against marketing material)
- **Origin bypass via discoverable Vercel default domains** — defence rejected
  at the FDID-header layer per ADR-032 §Decision-3 (rotation outage class >
  marginal protection at beta scale); WAF + rate-limit at AFD remain the primary
  defences for AFD-routed traffic

### 9.4 Compliance baseline (council Decision-Compliance)

Minimum-viable for SOC 2 / ISO 27001-readiness without committing to formal
audit:

- Log Analytics retention 365 days (CC7.2 / A.12.4 baseline)
- RBAC scoping on `rg-iac-state` (named group; PIM-eligible if used)
- AFD WAF rule changes require PR (no portal edits)
- Microsoft Azure listed as sub-processor in privacy notice / DPA
- AFD profile pinned to Australia East (residency-aligned)

Formal audit walkthrough deferred until first SOC 2 customer commits.

## 10. Risks (post-council)

| #   | Risk                                                          | Likelihood | Impact | Mitigation                                                                                                            |
| --- | ------------------------------------------------------------- | ---------- | ------ | --------------------------------------------------------------------------------------------------------------------- |
| R1  | DNS cutover causes brief outage if AFD origin not yet healthy | Low        | High   | Phase 2 alerts active; canary drill (EDGE-018) measures real timing; per-hostname incremental cut; apex last          |
| R2  | Cert provisioning fails for one or more custom domains        | Low        | Medium | Phase 2 validates cert issuance before Phase 3 cuts                                                                   |
| R3  | WAF false-positive blocks legitimate traffic post-flip        | Medium     | High   | Detection mode through Phase 3; prevention only after observed FP rate; rate-limit calibration window in Phase 4      |
| R4  | Cost overrun (Vercel egress)                                  | Low        | Low    | AFD-cache-static + Vercel usage alert at 80% + Azure cost alert at 150%                                               |
| R5  | Cert renewal silently fails                                   | Low        | High   | 4-layer defence in depth (R1 in §9.2)                                                                                 |
| R6  | AFD itself has a regional outage                              | Low        | High   | 99.99% SLA accepted; TTLs kept low post-cutover; manual DNS repoint runbook                                           |
| R7  | Vercel preview/prod parity bugs                               | Low        | Medium | Design constraint (ADR-032 §Decision-4): no AFD-conditional app code                                                  |
| R8  | Hype-launch traffic spike trips unconfigured rate limit       | Low        | High   | No rate limits in Phase 3 (added Phase 4 only after calibration window); deferred prevention until p99 + margin known |
| R9  | Phase 4 stalls post-launch                                    | Medium     | Low    | Named ~2-week commitment; module gate criteria split (Phase 0-3 = shippable, Phase 4 = complete); v0.4.2 may slip     |

## 11. Alternatives Considered

(Same alternatives as pre-council draft; rationale unchanged.)

### 11.1 Vercel-native redirect, no AFD — REJECTED

Solves www→apex in 4 lines. Doesn't solve multi-origin, WAF, or cert
centralisation. The 8-week Azure origin commit makes this a sunk cost — every
origin migration would re-litigate redirect/CORS/WAF logic.

### 11.2 Cloudflare in front — REJECTED

Same architectural shape, materially cheaper. Introduces a third vendor tenancy.
Re-evaluation trigger: Azure costs become material AND Cloudflare adoption is on
the table — architectural shape ports cleanly.

### 11.3 AFD Premium — DEFERRED

Standard's 5 custom rules sufficient for v0.4.x. Configuration-only upgrade when
material.

### 11.4 NGINX + Azure App Service as the edge — REJECTED

All the work of AFD without any of the managed bits. Strictly worse.

### 11.5 No edge layer, multi-origin via DNS only — REJECTED

Doesn't solve cross-cutting concerns. Each origin re-implements redirect, CORS,
rate-limit, cert. The exact problem we want to escape.

## 12. APS Module: EDGE

24 work items in `plans/modules/edge.aps.md`. Status: Ready (council approved).

Phase 0 + 1 (6 items) ship as one PR. Phase 2 (12 items) is a coordinated PR or
short PR-chain. Phase 3 (4 items) is per-hostname. Phase 4 (2 items) is
post-launch.

Ready-checklist outstanding:

- Log Analytics workspace existence verified (Phase 2 provisions if absent)
- Named workload for the 8-week origin commit captured for AFD origin-group
  config
- Owner assigned

## 13. Implementation Notes

### 13.1 Rollout cadence (council-confirmed)

- **Phases 0+1:** This week. Ships before hype launch with zero behaviour
  change.
- **Phase 2:** Next 1-2 weeks. AFD up but no traffic. Canary drill is the gate.
- **Phase 3:** Per-hostname cuts with 24h+ soak. Pre-launch for non-apex; apex
  last.
- **Phase 4:** Post-launch (~2 weeks). May slip v0.4.2 window — acceptable.

### 13.2 Test strategy

- **Phase 0:** `dns.test.ts` extended for imported records.
- **Phase 1:** `vercel.test.ts` extended for dual domains.
- **Phase 2:** new `front-door.test.ts` with Pulumi runtime mocks; smoke test
  AFD endpoint via cURL with `--resolve` override; chaos test origin failure.
- **Phase 3:** post-cutover smoke checks (existing
  `post-deploy-smoke-check.md` + new edge sections).
- **Phase 4:** WAF synthetic SQLi/XSS probes are blocked; rate-limit triggers at
  threshold + 1; no FP over 1-week soak.

### 13.3 Code-touched estimate (post-council)

- IaC: ~700 LOC of new Pulumi components (single FrontDoor) + ~250 LOC of
  composition + ~250 LOC of tests.
- Application: ~50 LOC for `/health` endpoints (anvil-api + website).
- Runbooks: ~400 LOC of markdown across 3 new + 1 updated.
- Total: ~1,400 LOC, mostly IaC and docs.

## 14. ADR-032 cross-reference

The five architectural decisions ratified by council and recorded in
`plans/decisions/032-edge-architecture-afd.md`:

1. AFD Standard, not Premium
2. Cert defence in depth (4 layers)
3. FDID origin auth deferred (with revisit triggers)
4. No AFD-conditional application code (design constraint)
5. Phased migration with named gates (alerts → Phase 2; canary drill gates
   Phase 3)

---

## Council artefacts

- Session: `.claude/council/sessions/plan-286b981a.json`
- Personas: architect, pragmatic-lead, adversarial-reviewer, security-analyst,
  operations-reviewer
- Rounds: 5 (Premise & Timing; Cutover Safety & Rollback; Origin Auth &
  Security; Compliance/Costs/Ops; Architecture Forks)
- Decisions: 20 across the 5 rounds
- Document revision: 1 (pre-council) → 2 (this revision)
