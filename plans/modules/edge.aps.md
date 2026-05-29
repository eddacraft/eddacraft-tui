<!--
APS Module: Edge Architecture (Azure Front Door)
=================================================
Multi-origin edge layer for eddacraft.ai. Azure Front Door Standard sits in
front of Vercel today and Azure-hosted origin (8-week commit) tomorrow,
owning TLS, redirect, WAF, observability, static-asset caching, and
rate-limiting. Migration is phased; alerts and canary rollback drill gate
the production cutover.

Authoritative design: plans/specs/2026-04-27-edge-architecture-multi-origin.md
ADR: plans/decisions/032-edge-architecture-afd.md
Council: plan-286b981a (5 personas, 5 rounds, 20 decisions, 2026-04-28)

This is a cross-cutting module — work items touch infra/, apps/anvil-api/,
apps/website/, docs/runbooks/, and plans/decisions/. Per the LAUNCH/RTAI
convention, references to these surfaces are explicit on each work item.

See: plans/aps-rules.md
-->

# Edge Architecture (Azure Front Door)

| ID   | Owner | Status |
| ---- | ----- | ------ |
| EDGE | —     | Ready  |

24 work items across 5 phases. Phase 0 + 1 are zero-cost hygiene that ship
immediately. Phase 2 stands AFD up with cert validation and alerts but does not
move traffic. Phase 3 cuts hostnames over to AFD one at a time with a canary
rollback drill as the gate. Phase 4 centralises cross-cutting concerns (WAF
prevention, rate-limit calibration, runbooks, ADR-032 finalisation).

The user's "must stay up" surfaces are the apex marketing site and the waitlist
API. Phase 3 cutover order reflects this: api/docs/www first, apex last, with
24h+ green soak between each.

## Purpose

Establish a single edge layer that survives origin migration. Cross-cutting HTTP
concerns (TLS, redirect, WAF, observability, caching, rate limiting) move from
per-origin or absent to AFD-managed and Pulumi-declarative. The DNS drift
documented in `infra/src/dns/eddacraft-ai.ts` is reconciled as part of the
migration.

## Out of Scope

- Replacing Vercel as origin for marketing site, anvil-api, or docs apps
- Migrating off Azure DNS
- Application-layer features (auth, sessions) at the edge
- AFD Premium features (Bot Manager, Private Link, 100 custom rules) — deferred
- FDID origin auth (`x-azure-fdid` header validation) — deferred per ADR-032
  §Decision-3
- Formal SOC 2 audit walkthrough — deferred until first SOC 2 customer

## Interfaces

**Depends on:**

- `@pulumi/azure-native` Front Door resource family
- Existing `infra/src/dns/eddacraft-ai.ts` (drift fix lands here)
- Existing `infra/src/components/dns-zone.ts` (extend for ALIAS records)
- Azure Log Analytics (workspace provisioned in Phase 2)

**Exposes:**

- AFD endpoint and FDID as Pulumi stack outputs
- `/health` endpoints on each Vercel origin (consumed by AFD probes +
  post-deploy smoke checks)
- New runbooks: `edge-cutover.md`, `edge-observability.md`,
  `edge-incident-response.md`

## Risks

| Risk                                                          | Impact | Mitigation                                                                                                         |
| ------------------------------------------------------------- | ------ | ------------------------------------------------------------------------------------------------------------------ |
| DNS cutover causes brief outage if AFD origin not yet healthy | High   | Phase 2 alerts; canary rollback drill; per-hostname incremental cut; apex last                                     |
| Cert provisioning fails for one or more custom domains        | Medium | Phase 2 validates cert issuance before any traffic shift                                                           |
| WAF false-positive blocks legitimate traffic post-flip        | High   | WAF stays in detection mode through Phase 3; flip to prevention only after observed FP rate                        |
| Cost overrun if egress projections wrong                      | Low    | Vercel usage alert at 80% quota; Azure cost alert at 150% of §7.2 projection; AFD-cache-static reduces Vercel pull |
| Cert renewal silently fails (`_dnsauth` TXT deleted)          | High   | PR-time TXT test + Pulumi protect flag + Azure Monitor alert + external synthetic monitor (defence in depth)       |
| AFD itself has a regional outage during launch                | Medium | Accept 99.99% SLA; keep TTLs at 60-120s post-cutover; manual DNS repoint runbook                                   |
| Vercel preview/prod parity gap creates silent bugs            | Medium | Design constraint (ADR-032 §Decision-4): no AFD-conditional app code                                               |
| Hype-launch traffic spike trips unconfigured rate limit       | Medium | Detection-mode calibration in Phase 4; thresholds derived from p99 + margin                                        |

## Ready Checklist

- [x] Council interrogation complete (plan-286b981a)
- [x] ADR-032 written with all 5 decisions
- [x] Multi-origin premise validated (concrete <8wk Azure-hosted origin commit)
- [x] Cost approval (Azure credits cover; Vercel egress mitigated by
      AFD-cache-static)
- [x] Capacity confirmed (≥5 focused IaC days pre-launch)
- [ ] Log Analytics workspace existence verified (Phase 2 provisions if absent)
- [ ] Named workload for the 8-week origin commit captured for AFD origin-group
      config
- [ ] Owner assigned

## Estimated Scope

- **Effort:** Phase 0 + 1: ~1 day. Phase 2: ~3-4 days. Phase 3: ~3 days
  (per-host cutover with soak). Phase 4: ~1 week post-launch.
- **Total LOC estimate:** ~1,400 across IaC, app middleware, runbooks, tests.

## Work Items

> Status discipline: each work item lists its phase. Phase 0 + 1 must be Done
> before Phase 2 starts. Phase 3 cuts are gated on Phase 2 alerts being live AND
> the canary rollback drill having been performed.

### EDGE-001: Pulumi-import the apex A record

- **Phase:** 0 (DNS reconciliation)
- **Surface:** `infra/src/dns/eddacraft-ai.ts`
- **Intent:** Bring the manually-added `eddacraft.ai` A record (`216.150.1.1`,
  currently pointing at Vercel) under Pulumi management without changing its
  target.
- **Expected Outcome:** `pulumi preview` shows zero changes for the apex record
  after import.
- **Validation:** `pulumi preview` returns "no changes";
  `dig +short eddacraft.ai` unchanged.
- **Confidence:** high

### EDGE-002: Pulumi-import the www CNAME

- **Phase:** 0
- **Surface:** `infra/src/dns/eddacraft-ai.ts`
- **Intent:** Bring the manually-added `www.eddacraft.ai` CNAME (currently
  pointing at `6c3bec1fd4ec9127.vercel-dns-017.com.`) under Pulumi management.
- **Expected Outcome:** `pulumi preview` shows zero changes for the www record
  after import.
- **Validation:** `dig +short www.eddacraft.ai` unchanged; record-of-record now
  in Pulumi state.
- **Confidence:** high

### EDGE-003: dns.test.ts assertions for imported records

- **Phase:** 0
- **Surface:** `infra/src/__tests__/dns.test.ts`
- **Intent:** Add Pulumi-runtime assertions that the apex A and www CNAME exist
  with their imported values, so future drift fails at PR time. Also assert
  (placeholder) `_dnsauth.*` TXT records will exist post-Phase-2.
- **Expected Outcome:** Test suite catches drift on these records.
- **Validation:**
  `pnpm exec vitest run --config vitest.config.ts src/__tests__/dns.test.ts`
  passes.
- **Confidence:** high

### EDGE-004: Vercel website domain IaC sync

- **Phase:** 1 (Vercel domain IaC sync)
- **Surface:** `infra/src/vercel.ts`
- **Intent:** Update website project `domains` array to declare both
  `eddacraft.ai` and `www.eddacraft.ai`. `pulumi import` the existing manual www
  `ProjectDomain` so Pulumi adopts rather than recreates.
- **Expected Outcome:** Pulumi state reflects both domains attached to the
  website project.
- **Validation:** `pulumi preview` shows only the import (no recreate); Vercel
  dashboard unchanged.
- **Confidence:** high

### EDGE-005: vercel.test.ts dual-domain assertion

- **Phase:** 1
- **Surface:** `infra/src/__tests__/vercel.test.ts`
- **Intent:** Assert that website has exactly two `ProjectDomain` resources
  (apex + www). Mirrors the assertion already on `main` (PR #1124) but applied
  to dev IaC.
- **Expected Outcome:** Test suite catches removal or addition of website
  domains at PR time.
- **Validation:**
  `pnpm exec vitest run --config vitest.config.ts src/__tests__/vercel.test.ts`
  passes.
- **Confidence:** high

### EDGE-006: anvil-api CORS allow-list updated for www

- **Phase:** 1
- **Surface:** `infra/src/vercel.ts` (`ANVIL_CORS_ORIGINS` env var on
  `anvil-api`)
- **Intent:** Add `https://www.eddacraft.ai` to the API CORS allow-list. Mirrors
  what already shipped on `main` via PR #1124. Lands the change on the `dev`
  line. Origin-side CORS remains the source of truth per ADR-032 §Decision-6.
- **Expected Outcome:** Waitlist submissions from www origin are accepted by the
  API on the dev deploy.
- **Validation:** Post-deploy CORS preflight smoke check (existing
  `docs/runbooks/post-deploy-smoke-check.md` §1b) passes for both origins.
- **Confidence:** high

### EDGE-007: Provision Log Analytics workspace

- **Phase:** 2 (AFD provisioning, no traffic)
- **Surface:** `infra/src/observability/log-analytics.ts` (new)
- **Intent:** Provision a Log Analytics workspace in `rg-iac-state` (or a
  sibling RG, decision per implementation), pinned to Australia East. Retention
  365 days for SOC 2 baseline. Attach diagnostic settings here for AFD logs.
- **Expected Outcome:** Workspace exists in Pulumi state; ready to receive AFD
  diagnostic logs.
- **Validation:** `az monitor log-analytics workspace show` returns the
  resource; retention is 365 days.
- **Confidence:** medium — coordinates with the OBS module's pending
  observability work; ownership boundary may need a brief discussion.

### EDGE-008: FrontDoor Pulumi component

- **Phase:** 2
- **Surface:** `infra/src/components/front-door.ts` (new)
- **Intent:** Single `FrontDoor` ComponentResource that owns AFD profile +
  endpoint + WAF policy + origin groups + routes + custom-domain bindings.
  Region-pinned to Australia East. WAF MS Default Rule Set in detection mode.
  Origin/route definitions accepted as args. Per ADR-032 §Decision-7.
- **Expected Outcome:** Single component; refactor only if it grows past ~400
  LOC.
- **Validation:** `pnpm exec tsc -p infra/tsconfig.json --noEmit`; component
  compiles and exports expected types.
- **Confidence:** medium

### EDGE-009: AFD profile, endpoint, and WAF instantiation

- **Phase:** 2
- **Surface:** `infra/src/edge/front-door.ts` (new)
- **Intent:** Instantiate the `FrontDoor` component with the four custom domains
  (apex, www, api, docs), WAF in detection mode, MS Default Rule Set attached.
  AFD endpoint is reachable but production DNS is unchanged.
- **Expected Outcome:** AFD endpoint resolves; custom-domain certs show
  "Approved"; WAF logs flowing to Log Analytics.
- **Validation:**
  `curl -k --resolve eddacraft.ai:443:<afd-endpoint-ip> https://eddacraft.ai/`
  returns the website project's content; WAF detection events visible in Log
  Analytics within 5 min.
- **Confidence:** medium

### EDGE-010: Origin groups and routes

- **Phase:** 2
- **Surface:** `infra/src/edge/origins.ts`, `infra/src/edge/routes.ts` (new)
- **Intent:** Origin groups for `website`, `anvil-api`, `docs-shell`,
  `docs-private` pointing at Vercel default-domain hostnames. Routes per public
  host: apex/www → website (with www→apex 308 rule), api → anvil-api, docs →
  docs-shell. Static-asset routes (`/_next/static/*`, images, css, js)
  configured for AFD caching; dynamic routes passthrough.
- **Expected Outcome:** All four hostnames serve via AFD endpoint when accessed
  with appropriate Host header; 308 redirect on www; static assets cached at AFD
  edge.
- **Validation:** Smoke test via `curl --resolve` covers all four hostnames;
  static-asset response includes `x-cache: HIT` after warm-up.
- **Confidence:** medium

### EDGE-011: `_dnsauth` TXT records under Pulumi management

- **Phase:** 2
- **Surface:** `infra/src/dns/eddacraft-ai.ts`
- **Intent:** Custom-domain TXT validation records (`_dnsauth.<hostname>`) added
  to the Pulumi-managed zone. Each resource carries the Pulumi `protect: true`
  flag so destroy requires explicit unprotect step.
- **Expected Outcome:** AFD custom-domain validation passes; cert issuance for
  all four hostnames; `_dnsauth` resources cannot be accidentally destroyed.
- **Validation:** AFD shows custom-domain status "Approved";
  `pulumi destroy --target ...` on a `_dnsauth` resource is refused without
  explicit unprotect.
- **Confidence:** high

### EDGE-012: dns.test.ts asserts `_dnsauth` records present

- **Phase:** 2
- **Surface:** `infra/src/__tests__/dns.test.ts`
- **Intent:** Extend the test suite to assert each `_dnsauth.<hostname>` TXT
  exists and matches the AFD-expected value. PR-time failure beats weeks-later
  silent cert death.
- **Expected Outcome:** Removal of any `_dnsauth` record causes test failure.
- **Validation:** Test suite passes; mutation test (manually delete a record in
  test fixture) fails as expected.
- **Confidence:** high

### EDGE-013: Synthetic /health endpoint on anvil-api

- **Phase:** 2
- **Surface:** `apps/anvil-api/src/routes/health.ts` (extend or new)
- **Intent:** `/health` endpoint returns 200 with payload including version +
  uptime. Sub-500ms latency target. Used by AFD origin probes AND post-deploy
  smoke checks (single observable signal).
- **Expected Outcome:** Endpoint stable, sub-500ms p99 against Vercel cold
  start.
- **Validation:** `curl https://api.eddacraft.ai/health` returns 200 with
  expected payload; Vercel metrics show p99 < 500ms.
- **Confidence:** high

### EDGE-014: Synthetic /health endpoint on website

- **Phase:** 2
- **Surface:** `apps/website/app/health/route.ts` (new) or `next.config.ts`
  health route
- **Intent:** Equivalent endpoint on the marketing site. Static response is fine
  (200 with version string).
- **Expected Outcome:** Endpoint stable.
- **Validation:** `curl https://eddacraft.ai/health` returns 200.
- **Confidence:** high

### EDGE-015: AFD origin health probe configuration

- **Phase:** 2
- **Surface:** `infra/src/edge/origins.ts`
- **Intent:** Each origin group probes `/health` with: interval 30s, success
  threshold 2/3 samples, HTTP 200 required, timeout 5s. AFD
  all-origins-unhealthy returns a documented 503 page.
- **Expected Outcome:** Probe timing tuned for Vercel cold-start P99; failover
  behaviour deterministic.
- **Validation:** Phase 2 manual chaos test: kill an origin (point to
  `127.0.0.1`); AFD reports it unhealthy within 90s; restore reports healthy
  within 90s.
- **Confidence:** medium

### EDGE-016: Phase 2 alerts (5xx-rate + origin-health-probe)

- **Phase:** 2
- **Surface:** `infra/src/edge/alerts.ts` (new)
- **Intent:** Azure Monitor alerts on AFD diagnostic logs: (a) 5xx rate > 1%
  over 5 min, (b) origin health probe failed for 3 consecutive samples. Page
  on-call on either. Critically: these alerts must exist BEFORE any Phase 3 DNS
  cut.
- **Expected Outcome:** Alerts active; test alert fires successfully through the
  on-call channel.
- **Validation:** Synthetic test alert routes to on-call; verified delivery.
- **Confidence:** medium

### EDGE-017: Cert expiry alert + external synthetic monitor

- **Phase:** 2
- **Surface:** `infra/src/edge/alerts.ts`; external uptime/cert monitor (e.g.
  UptimeRobot, Better Stack)
- **Intent:** Azure Monitor alert: cert expiry < 30d (warn), < 14d (page), < 7d
  (escalate). External monitor (outside Azure tenancy) probes each hostname's
  cert chain daily; independent of AFD or Azure-Monitor failure modes.
- **Expected Outcome:** Cert expiry surfaces at least 30 days before any browser
  break.
- **Validation:** Force a test alert at the 30d threshold; external monitor
  reports cert validity.
- **Confidence:** medium — third-party monitor selection requires a small
  evaluation pass.

### EDGE-018: Canary rollback drill

- **Phase:** 2 (gates Phase 3)
- **Surface:** `docs/runbooks/edge-cutover.md` (new)
- **Intent:** Provision `canary.eddacraft.ai` via Pulumi (custom domain on the
  AFD endpoint, points at the website origin group). Cut DNS to AFD; verify;
  revert; measure full propagation timing including AFD control-plane
  association removal, Azure DNS propagation, and resolver cache behaviour.
  Publish measured numbers in the cutover runbook.
- **Expected Outcome:** Real rollback timing documented (target: p95 < 5 min).
  Phase 3 cutover runbook references measured numbers, not the aspirational 60s.
- **Validation:** Drill executed end-to-end; numbers published; on-call signs
  off on the measured rollback SLA.
- **Confidence:** medium

### EDGE-019: Cut api.eddacraft.ai to AFD

- **Phase:** 3 (per-hostname DNS cutover)
- **Surface:** `infra/src/dns/eddacraft-ai.ts`
- **Intent:** Update DNS to ALIAS `api.eddacraft.ai` at the AFD endpoint.
  Smallest blast radius; least user-visible. Validates the route + origin
  group + WAF detection-mode logging. 24h+ green soak before next cutover.
- **Expected Outcome:** Waitlist + admin endpoints serve via AFD; smoke checks
  pass; WAF logs flowing.
- **Validation:** Post-deploy smoke check (full §1 + §1b + §2 in
  `post-deploy-smoke-check.md`); 5xx rate < 0.1% over 24h.
- **Confidence:** medium

### EDGE-020: Cut docs.eddacraft.ai to AFD

- **Phase:** 3
- **Surface:** `infra/src/dns/eddacraft-ai.ts`
- **Intent:** Cut docs hostname. Validates the proxy-to-docs-shell origin path
  under AFD.
- **Expected Outcome:** Docs reachable via AFD; 24h soak.
- **Validation:** `curl -I https://docs.eddacraft.ai/` returns 200; smoke check
  passes.
- **Confidence:** medium

### EDGE-021: Cut www.eddacraft.ai to AFD

- **Phase:** 3
- **Surface:** `infra/src/dns/eddacraft-ai.ts`
- **Intent:** Cut www. Validates the 308 redirect rule. www should now serve a
  redirect, not the website.
- **Expected Outcome:** `curl -I https://www.eddacraft.ai/` returns 308 with
  `Location: https://eddacraft.ai/`.
- **Validation:** Redirect chain test; SEO tools confirm canonical is apex.
- **Confidence:** medium

### EDGE-022: Cut eddacraft.ai apex to AFD

- **Phase:** 3
- **Surface:** `infra/src/dns/eddacraft-ai.ts`
- **Intent:** Cut apex last (highest visibility, most-validated by then). Apex +
  waitlist API are the user's "must stay up" surfaces; this is the most-cautious
  cut.
- **Expected Outcome:** Marketing site serves via AFD; etag matches
  direct-Vercel; static assets cached at AFD.
- **Validation:** Smoke check passes; static-asset response shows `x-cache: HIT`
  post-warmup; no 5xx alerts trigger over 1h watch window.
- **Confidence:** medium — most-careful cut, longest watch window.

### EDGE-023: WAF prevention mode + rate-limit calibration window

- **Phase:** 4 (centralise concerns)
- **Surface:** `infra/src/edge/front-door.ts`, `infra/src/edge/rate-limits.ts`
  (new)
- **Intent:** After Phase 3 has been green-soaked for 1 week, ship rate limits
  in detection mode and observe legitimate traffic in Log Analytics for 2 weeks.
  Set prevention thresholds at p99 + safety margin. Flip WAF from detection to
  prevention mode. Add per-route rate-limit rules (waitlist, admin) at
  calibrated values, not the doc's earlier guesses.
- **Expected Outcome:** WAF blocks known-bad patterns; rate-limit triggers at
  calibrated thresholds; FP rate < 0.01% in production traffic.
- **Validation:** Synthetic SQLi/XSS probes are blocked (test in pre-prod);
  rate-limit triggers at threshold + 1; no legitimate traffic blocked over
  1-week soak.
- **Confidence:** medium

### EDGE-024: Phase 4 hardening + ADR-032 reference + runbooks

- **Phase:** 4
- **Surface:** `infra/src/edge/front-door.ts` (response headers);
  `docs/runbooks/edge-cutover.md`, `edge-observability.md`,
  `edge-incident-response.md`; `docs/runbooks/post-deploy-smoke-check.md`;
  `plans/decisions/032-edge-architecture-afd.md`
- **Intent:** Add HSTS / `x-frame-options: DENY` /
  `x-content-type-options: nosniff` / `referrer-policy` response-header rules.
  Cross-link ADR-032 from `plans/decisions/DECISION-LOG.md`. Land the three new
  runbooks. Update post-deploy smoke check with edge sections.
- **Expected Outcome:** Hardened response headers across all hosts;
  observability + incident-response runbooks in place; ADR ratified.
- **Validation:** `securityheaders.com` scan returns A+ for all four hostnames;
  runbooks reviewed; smoke check covers AFD health, WAF mode, cert expiry.
- **Confidence:** high

## Stats

| Phase                         | Total | Done | In Progress | Todo |
| ----------------------------- | ----- | ---- | ----------- | ---- |
| 0 — DNS reconciliation        | 3     | 0    | 0           | 3    |
| 1 — Vercel IaC sync           | 3     | 0    | 0           | 3    |
| 2 — AFD provisioning + canary | 12    | 0    | 0           | 12   |
| 3 — DNS cutover               | 4     | 0    | 0           | 4    |
| 4 — Centralise concerns       | 2     | 0    | 0           | 2    |
| **Total**                     | 24    | 0    | 0           | 24   |

### Item Detail

| ID       | Phase | Status | Notes                                           |
| -------- | ----- | ------ | ----------------------------------------------- |
| EDGE-001 | 0     | Todo   | Pulumi-import apex A record                     |
| EDGE-002 | 0     | Todo   | Pulumi-import www CNAME                         |
| EDGE-003 | 0     | Todo   | dns.test.ts assertions                          |
| EDGE-004 | 1     | Todo   | Vercel website domains: apex + www              |
| EDGE-005 | 1     | Todo   | vercel.test.ts dual-domain assertion            |
| EDGE-006 | 1     | Todo   | API CORS allow-list adds www                    |
| EDGE-007 | 2     | Todo   | Log Analytics workspace                         |
| EDGE-008 | 2     | Todo   | FrontDoor Pulumi component                      |
| EDGE-009 | 2     | Todo   | AFD profile + endpoint + WAF (detection)        |
| EDGE-010 | 2     | Todo   | Origin groups + routes (cache static)           |
| EDGE-011 | 2     | Todo   | \_dnsauth TXT records (Pulumi protect)          |
| EDGE-012 | 2     | Todo   | dns.test.ts \_dnsauth assertions                |
| EDGE-013 | 2     | Todo   | /health on anvil-api                            |
| EDGE-014 | 2     | Todo   | /health on website                              |
| EDGE-015 | 2     | Todo   | AFD probe configuration                         |
| EDGE-016 | 2     | Todo   | Phase 2 alerts (5xx + origin health)            |
| EDGE-017 | 2     | Todo   | Cert expiry alerts + external monitor           |
| EDGE-018 | 2     | Todo   | Canary rollback drill                           |
| EDGE-019 | 3     | Todo   | Cut api.eddacraft.ai                            |
| EDGE-020 | 3     | Todo   | Cut docs.eddacraft.ai                           |
| EDGE-021 | 3     | Todo   | Cut www.eddacraft.ai (verifies 308)             |
| EDGE-022 | 3     | Todo   | Cut eddacraft.ai apex                           |
| EDGE-023 | 4     | Todo   | WAF prevention + rate-limit calibration         |
| EDGE-024 | 4     | Todo   | Hardening headers + runbooks + ADR finalisation |

## Cross-cutting convention

Per the LAUNCH and RTAI cross-cutting precedent: this module touches IaC,
application code, runbooks, and ADR. References are explicit on each work item
(`Surface:` field). The module file lives in `plans/modules/edge.aps.md`; the
design spec lives in `plans/specs/2026-04-27-edge-architecture-multi-origin.md`;
the ADR lives in `plans/decisions/032-edge-architecture-afd.md`.

If/when the cross-cutting convention is promoted to a first-class APS primitive
(see `plans/aps-rules.md` for the current shape), this module will adopt the
typed callout shape.

## Deferred / Trigger-gated

Items deliberately out of MVP scope, with explicit revisit triggers:

- **FDID origin auth (former EDGE-019/020 in pre-council draft).** Triggers:
  first non-Vercel origin migrates / WAF moves to prevention / evidence of
  bypass attempts. See ADR-032 §Decision-3.
- **AFD Premium upgrade.** Triggers: WAF custom rule count exceeds 5 /
  Azure-side origin needs Private Link / bot traffic against marketing site
  becomes material.
- **Formal SOC 2 audit walkthrough.** Trigger: first SOC 2 customer commits.
- **Aggressive AFD caching of moderately-static API responses.** Trigger: Vercel
  egress overage exceeds $200/mo for 2 consecutive months.
