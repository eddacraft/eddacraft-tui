<!--
APS Module: API Governance
====================================
Governs the anvil-api surface as a first-class architectural concern.
See: plans/aps-rules.md
-->

# API Governance

| ID     | Owner | Status    |
| ------ | ----- | --------- |
| APGOV  | —     | Ready     |

**Last reviewed:** 2026-04-26

## Purpose

Govern the `anvil-api` (Hono REST API on Vercel) as a first-class architectural
surface — not just a passive target for other modules to add endpoints to.
Establish API versioning, deprecation policy, rate limiting strategy, CORS
evolution, and OpenAPI spec maintenance.

**Problem:** The API is actively deployed with live endpoints (waitlist, auth,
admin) and being extended by beta-auth-streamline, but has no module governing
its architecture. Endpoints are added ad-hoc without versioning strategy,
deprecation policy, or consistent error shapes.

## In Scope

- **API versioning strategy:** URL prefix (`/api/v1/`), header-based, or both
- **Deprecation policy:** Sunset headers, deprecation timeline, migration guides
- **Rate limiting:** Per-endpoint limits, user-based quotas, abuse detection
- **CORS evolution:** Origin policy as the dashboard and external integrations grow
- **OpenAPI spec:** Auto-generated from route definitions, kept in sync
- **Error contract:** Consistent error shapes across all endpoints
- **Health/readiness:** `/health` endpoint for monitoring, dependency checks
- **Endpoint lifecycle:** Draft → Active → Deprecated → Removed

## Out of Scope

- GraphQL support (no plans)
- WebSocket API design (covered by real-time-validation)
- Authentication implementation (covered by beta-auth-streamline)
- Database schema governance (covered by schema-contracts)

## Interfaces

**Depends on:**

- `apps/anvil-api` — the live API code
- `beta-auth-streamline` — new auth endpoints
- `observability-foundation` — health signals

**Exposes:**

- API governance contract (versioning, deprecation, error shapes)
- OpenAPI spec generation pipeline
- Rate limiting configuration

## Estimated Scope

- **Effort:** 1-2 weeks

## Work Items

### APGOV-001: API versioning strategy and URL prefix convention

- **Status:** Ready
- **Intent:** Establish URL versioning convention (e.g. `/api/v1/`)
- **Expected Outcome:** All endpoints follow `/api/vN/` pattern; version documented
- **Validation:** `grep -q "basePath('/api/v1')" apps/anvil-api/src/index.ts`

### APGOV-002: Error contract specification (consistent shapes)

- **Status:** Ready
- **Intent:** Define standard error response shape across all endpoints
- **Expected Outcome:** All error responses match `{ error: { code, message, details? } }`
- **Validation:** Integration test asserts error shape on 400/401/404/500

### APGOV-003: Rate limiting framework

- **Status:** Ready
- **Intent:** Add per-endpoint rate limiting with configurable limits
- **Expected Outcome:** Rate limit headers on all responses; 429 on exceeded
- **Validation:** `curl -I localhost:3000/api/v1/health | grep X-RateLimit`

### APGOV-004: OpenAPI spec generation from Hono routes

- **Status:** Ready
- **Intent:** Auto-generate OpenAPI spec from route definitions
- **Expected Outcome:** `/api/openapi.json` returns valid spec
- **Validation:** `curl localhost:3000/api/openapi.json | jq .openapi` returns "3.0.0"

### APGOV-005: Deprecation policy and sunset header support

- **Status:** Ready
- **Intent:** Define how endpoints are deprecated and sunset
- **Expected Outcome:** Deprecated endpoints return Sunset and Deprecation headers
- **Validation:** `curl -I localhost:3000/api/v1/test-deprecated -H "Accept: application/json" | grep -i sunset`

### APGOV-006: Health endpoint and dependency checks

- **Status:** Draft — **needs design** (response-shape + dependency-set
  decision; see "Blocks on" below). Not fleshable to Ready without an owner
  call.
- **Intent:** Reconcile the API's health/readiness contract — a `/health`
  endpoint **already ships** at `apps/anvil-api/src/index.ts:79`.
- **Reality on `main` (2026-05-28):** `app.get('/health')` (registered under
  `.basePath('/api/v1')`, so the live path is already `/api/v1/health`)
  probes three dependencies in parallel — the Neon DB (`SELECT 1`), the
  licence signing key, and the licence verifying key — and returns
  `{ status: 'ok', db, signingKey, verifyingKey }` on success or
  `{ status: 'degraded', ... }` with HTTP 503 on any failure. So the
  endpoint exists; what is unresolved is its **contract shape** and **which
  dependencies it is responsible for**, not whether it exists.
- **Why this is needs-design, not ready-to-flesh:**
  1. **Response-shape mismatch.** The original draft outcome
     (`{ status: "ok", checks: {...} }`, nested) contradicts the shipped flat
     shape (`{ status, db, signingKey, verifyingKey }`). Picking one is a
     public-contract decision that must land alongside APGOV-002 (error
     contract) and APGOV-004 (OpenAPI spec) so the health response is
     documented consistently — not invented here.
  2. **Dependency-set ownership overlaps OBS.** "Checks DB and external
     deps" collides with `observability-foundation` (OBS), which owns the
     core health-signal set (`plans/modules/observability-foundation.aps.md:51`
     — uptime, error rate, latency, queue depth, email delivery) and Neon
     health visibility (`:90`, OBS-002 at `:125`). Deciding whether `/health`
     reports email-delivery / queue-depth / OPA-binary reachability, or stays
     a thin liveness probe, is an OBS↔APGOV boundary call.
- **Blocks on:** an owner decision recording (a) the canonical `/health`
  response shape (flat vs. nested `checks`, and whether to add a separate
  `/health/ready` vs `/health/live` split), and (b) the dependency set
  `/health` owns vs. what OBS surfaces through observability instead. Capture
  the decision as a short ADR or an APGOV/OBS coordination note before
  promoting.
- **Coordinates with:** APGOV-002 (error contract), APGOV-004 (OpenAPI spec),
  and `observability-foundation` OBS-001/OBS-002 (health signals, Neon
  instrumentation).
- **Expected Outcome (deferred — set once the shape is decided):** the live
  `/api/v1/health` response matches the agreed contract and the agreed
  dependency set, and is documented in the OpenAPI spec.
- **Validation (deferred):** `curl localhost:3000/api/v1/health | jq .status`
  returns `"ok"` against the agreed shape, plus a route test asserting the
  degraded path returns 503.

### APGOV-007: CORS policy documentation and configuration

- **Status:** Ready
- **Intent:** Document and configure CORS origins as integrations grow
- **Expected Outcome:** CORS policy documented in docs/guides/
- **Validation:** `cat docs/guides/cors-policy.md | grep -q "origins"`
