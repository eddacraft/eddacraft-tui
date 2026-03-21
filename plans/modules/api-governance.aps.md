<!--
APS Module: API Governance
====================================
Governs the anvil-api surface as a first-class architectural concern.
See: plans/aps-rules.md
-->

# API Governance

| ID     | Owner | Status    |
| ------ | ----- | --------- |
| APGOV  | —     | Draft |

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

- **Status:** Draft

### APGOV-002: Error contract specification (consistent shapes)

- **Status:** Draft

### APGOV-003: Rate limiting framework

- **Status:** Draft

### APGOV-004: OpenAPI spec generation from Hono routes

- **Status:** Draft

### APGOV-005: Deprecation policy and sunset header support

- **Status:** Draft

### APGOV-006: Health endpoint and dependency checks

- **Status:** Draft

### APGOV-007: CORS policy documentation and configuration

- **Status:** Draft
