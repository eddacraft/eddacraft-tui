# ADR-032: Azure Front Door as Canonical Edge Layer for eddacraft.ai

**Status:** Accepted **Date:** 2026-04-28 **Council:** plan-286b981a (5
personas, 5 rounds, 20 decisions) **Supersedes:** Implicit "Vercel as both
origin and edge" assumption that informs current `infra/src/vercel.ts`.

---

## Context

Anvil's web surfaces (marketing site, anvil-api, docs apps) are deployed on
Vercel. Cross-cutting HTTP concerns — TLS, domain attachment, redirects, CORS,
WAF, rate limiting — currently live at the Vercel layer or are absent.

A second non-Vercel origin is committed within 8 weeks (Azure App Service /
Container Apps surface, exact workload TBD). Without an edge layer that survives
origin migration, every new origin re-implements
redirects/CORS/WAF/cert-handling and every DNS record needs touching — meaning
origin migrations under hype-launch traffic become risky.

The DNS authority is Azure DNS, partially under Pulumi management at
`infra/src/dns/eddacraft-ai.ts`. The user already operates Azure DNS, Key Vault,
and signing infrastructure — Azure tenancy alignment is established.

## Decision

Use **Azure Front Door Standard** as the always-in-front edge layer for
`eddacraft.ai`. Azure DNS points at AFD; AFD routes to origins (Vercel today,
Azure-hosted within 8 weeks, others later) via host/path rules. WAF, the
www→apex redirect, TLS termination, observability, static-asset caching, and
rate-limiting all live at AFD.

The AFD profile is pinned to **Australia East** to match user-residency intent;
Microsoft Azure is added to the privacy notice / DPA as a sub-processor.

### Cross-cutting concern routing (post-council)

| Concern                   | Lives at                  | Notes                                                                                    |
| ------------------------- | ------------------------- | ---------------------------------------------------------------------------------------- |
| TLS termination           | AFD (managed certs)       | `_dnsauth` TXT records Pulumi-protected; defence-in-depth monitoring per ADR §Decision-2 |
| Domain → backend binding  | AFD route + origin group  | Vercel keeps domains for AFD verification; not DNS-facing                                |
| www → apex redirect       | AFD rules engine, 308     | Eliminates need for client-side or origin-side redirect logic                            |
| Static-asset caching      | AFD                       | Reduces Vercel origin pull (egress concern)                                              |
| Dynamic / ISR caching     | Vercel                    | AFD passthrough for non-static routes                                                    |
| WAF                       | AFD WAF policy            | MS Default Rule Set; detection-only through Phase 3, prevention in Phase 4               |
| Rate limiting             | AFD rules engine          | Detection-mode calibration first; thresholds derived from p99 + margin                   |
| HSTS, security headers    | AFD response-header rules | Phase 4 (post-cutover stability)                                                         |
| CORS allow-list           | **Hybrid**                | AFD coarse host-allowlist; anvil-api keeps `ANVIL_CORS_ORIGINS` for per-route logic      |
| DDoS                      | AFD                       | Native Azure DDoS Standard, free for AFD tenants                                         |
| Observability             | Log Analytics workspace   | New workspace provisioned in Phase 2; 1y retention for SOC 2 baseline                    |
| Origin auth (FDID header) | **DEFERRED**              | See §Decision-3                                                                          |

### Decision-1: AFD Standard, not Premium

Standard is sufficient at beta scale. Custom rule limit (5) is adequate for
v0.4.x. Re-evaluate Premium when: custom rules exceed 5, Azure-side origin needs
Private Link, or bot traffic against marketing site becomes material. One-line
Pulumi change to upgrade.

### Decision-2: Cert defence in depth

`_dnsauth` TXT records are the silent-failure class. Mitigation is layered:

1. **PR-time test** — `dns.test.ts` asserts every `_dnsauth.<hostname>` TXT
   exists and matches AFD-expected value
2. **Pulumi protect flag** — destroy on `_dnsauth` resources requires explicit
   unprotect step
3. **Azure Monitor alert** — cert expiry < 30/14/7 days
4. **External synthetic monitor** — third-party probe outside Azure tenancy
   daily

Defence in depth eliminates the silent-cert-death class.

### Decision-3: FDID origin auth deferred

The `x-azure-fdid` header model has three real failure modes:

- FDID rotation mismatch → 100% API 403 outage until Vercel env redeploys
- Vercel default `*.vercel.app` domains discoverable via cert-transparency /
  preview URLs → weak protection
- Preview-deploy bypass requires careful flag-gating to avoid prod env leak

Anvil's primary auth lives in `apps/anvil-api/src/middleware/admin-auth.ts` and
JWT licence checks. FDID would be defence-in-depth, not primary auth. For a
hype-launch beta with no paying customers and waitlist as the highest-attention
surface, the failure-mode cost exceeds the marginal security benefit.

**Defer EDGE-019/020. Revisit when ANY trigger fires:**

- First non-Vercel origin migrates (bypass becomes more attractive)
- WAF moves to prevention mode (FDID gap is now load-bearing for "all traffic
  goes through WAF")
- Evidence of bypass attempts in Vercel logs

### Decision-4: No AFD-conditional application code

Design constraint: application code does not gate behaviour on AFD-injected
headers. CORS, HSTS, rate-limiting are infrastructure-only concerns enforced at
AFD. This eliminates dev/prod parity bugs (Vercel preview deploys bypass AFD) by
construction.

### Decision-5: Phased migration with named phases

| Phase | Theme                                                                         | Pre-launch?                                    |
| ----- | ----------------------------------------------------------------------------- | ---------------------------------------------- |
| 0     | DNS reconciliation (drift fix, no behaviour change)                           | Yes                                            |
| 1     | Vercel domain IaC sync                                                        | Yes                                            |
| 2     | AFD provisioning + alerts + canary + workspace                                | Yes (capacity confirmed)                       |
| 3     | Per-hostname DNS cutover                                                      | Pre-launch for non-apex; apex last             |
| 4     | Centralise concerns (WAF prevention, rate limits, runbooks, ADR finalisation) | Post-launch (~2 weeks); may slip v0.4.2 window |

Phase 0 + 1 are zero-cost hygiene. Phase 2 is the substantive infrastructure
step. Phase 3 is gated on alerts existing (Phase 2) AND a canary rollback drill
having been performed.

## Consequences

### Positive

- **Multi-origin federation native** — Azure-hosted origin (8wk commit) and any
  future origins plug in as origin groups without DNS changes
- **One source of truth** — redirect, WAF, certs, observability all in one
  Pulumi-managed surface
- **Tenancy alignment** — operates inside the existing Azure footprint (DNS, KV,
  signing); no new vendor
- **Vercel becomes a swappable origin** — origin migration is an AFD
  origin-group config change, not a DNS rebuild
- **Static-asset caching at AFD** reduces Vercel origin pull, addressing the
  user's Vercel-egress concern at scale

### Negative

- **+$40–80/mo OPEX at beta scale, ~$300/mo at hype-launch scale** — covered by
  Azure credits until funding closes; Vercel egress capped via
  AFD-cache-static + usage alerts
- **One additional hop** — AFD edges global; for AU/Asia traffic typically
  neutral or improved; worst case +5–15ms
- **One more Pulumi component to operate** — single `FrontDoor`
  ComponentResource owns the tree
- **Dev/prod parity gap with Vercel preview deploys** — mitigated by Decision-4
  (no AFD-conditional app code)
- **Audit boundary expands** — Azure tenant RBAC + Pulumi state access + AFD log
  retention now in scope; minimum-viable compliance addressed in Phase 2 (1y
  retention, RBAC scoping, WAF rule changes via PR)

### Trigger conditions to reopen this ADR

- Vercel egress overage exceeds $200/mo for 2 consecutive months → revisit AFD
  caching scope
- AFD Standard custom rule limit becomes constraining → upgrade to Premium
- A SOC 2 audit is committed → Phase 4+ formal compliance walkthrough becomes a
  gate
- Cloudflare (or another global edge) becomes operationally preferred →
  re-decision; the architectural shape ports cleanly

## Alternatives Considered

### Vercel-native redirect, no AFD

Adds `redirect: 'eddacraft.ai'` to the www `vercel.ProjectDomain`. Solves the
immediate www→apex question in 4 lines.

**Rejected:** Doesn't solve multi-origin federation. Doesn't centralise
WAF/cert/observability. The 8-week Azure-origin commit makes this a sunk cost —
every origin migration would re-litigate redirect/CORS/WAF logic.

### Cloudflare in front

Same architectural shape, materially cheaper, free tier covers small-scale.

**Rejected:** Introduces a third vendor tenancy alongside Azure (DNS, KV,
signing) and Vercel. AFD aligns with the existing Azure footprint. Re-evaluation
trigger: if Azure costs become material AND Cloudflare adoption is on the table,
the architectural shape ports — same decision, different vendor.

### AFD Premium

Adds Bot Manager + Private Link + 100 custom rules at +$295/mo.

**Deferred, not rejected:** Standard's 5 custom rules sufficient for v0.4.x.
Configuration-only upgrade when material.

### NGINX + Azure App Service as the edge

Roll our own edge layer.

**Rejected:** All the work of AFD without any of the managed bits (certs, WAF,
DDoS, global edge POPs). Strictly worse at our scale.

### No edge layer, multi-origin via DNS only

Use DNS-only routing — apex/www to one origin, api to another.

**Rejected:** Doesn't solve cross-cutting concerns. Each origin re-implements
redirect/CORS/WAF/cert. Exact problem we want to escape.

## References

- Spec: `plans/specs/2026-04-27-edge-architecture-multi-origin.md`
- Module: `plans/modules/edge.aps.md`
- Council session: `.claude/council/sessions/plan-286b981a.json`
- Related ADRs: ADR-007 (Pulumi IaC), ADR-031 (validation latency rubric —
  independent)
