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

## Tasks

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

- **Status:** Draft
- **Intent:** Add `/api/v1/health` endpoint that checks DB and external deps
- **Expected Outcome:** `/api/v1/health` returns `{ status: "ok", checks: {...} }`
- **Validation:** `curl localhost:3000/api/v1/health | jq .status` returns "ok"

### APGOV-007: CORS policy documentation and configuration

- **Status:** Ready
- **Intent:** Document and configure CORS origins as integrations grow
- **Expected Outcome:** CORS policy documented in docs/guides/
- **Validation:** `cat docs/guides/cors-policy.md | grep -q "origins"`
