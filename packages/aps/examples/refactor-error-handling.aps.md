# Refactor: API Error Handling

**Scope:** API **Owner:** @engineering-team **Priority:** high

> Standardise error handling across all API endpoints with consistent response
> format, logging, and monitoring.

## Tasks

### API-001: Audit current error handling patterns

**Intent:** Survey all endpoints to document current error handling approaches
and inconsistencies **Expected Outcome:** Report listing error patterns, missing
cases, and inconsistency examples **Confidence:** low **Scopes:** API, AUTH,
PROD, CART, PAY **Tags:** audit, research, documentation **Inputs:**

- Access to all API endpoint code
- Error logs from production (last 30 days)

### API-002: Design unified error response schema

**Intent:** Define standard error response format with codes, messages, and
metadata **Expected Outcome:** Error schema documented with examples for common
cases (validation, auth, not found, server error) **Confidence:** medium
**Scopes:** API **Tags:** design, schema, standards **Dependencies:** API-001

### API-003: Create base error classes

**Intent:** Implement error class hierarchy (ApiError, ValidationError,
AuthError, NotFoundError, etc.) **Expected Outcome:** Error classes with proper
inheritance, serialisation, and HTTP status mapping **Confidence:** high
**Scopes:** API **Tags:** errors, classes, core **Dependencies:** API-002

### API-004: Implement global error middleware

**Intent:** Create Express error middleware that catches all errors and formats
responses **Expected Outcome:** Middleware handling all error types with
logging, status codes, and response formatting **Confidence:** medium
**Scopes:** API **Tags:** middleware, errors, express **Dependencies:** API-003
**Inputs:**

- Logging strategy (structured JSON logs? Log aggregation tool?)
- Monitoring integration (Sentry, DataDog?)

### API-005: Refactor authentication endpoints

**Intent:** Update all AUTH endpoints to use new error classes and middleware
**Expected Outcome:** AUTH endpoints throwing proper errors, tests updated,
error responses verified **Confidence:** high **Scopes:** API, AUTH **Tags:**
refactor, authentication **Dependencies:** API-003, API-004

### API-006: Refactor product catalog endpoints

**Intent:** Update all PROD endpoints to use new error classes and middleware
**Expected Outcome:** PROD endpoints throwing proper errors, tests updated
**Confidence:** high **Scopes:** API, PROD **Tags:** refactor, products
**Dependencies:** API-003, API-004

### API-007: Refactor shopping cart endpoints

**Intent:** Update all CART endpoints to use new error classes and middleware
**Expected Outcome:** CART endpoints throwing proper errors, tests updated
**Confidence:** high **Scopes:** API, CART **Tags:** refactor, cart
**Dependencies:** API-003, API-004

### API-008: Refactor payment endpoints

**Intent:** Update all PAY endpoints to use new error classes and middleware
**Expected Outcome:** PAY endpoints throwing proper errors, Stripe error mapping
included **Confidence:** medium **Scopes:** API, PAY **Tags:** refactor,
payments **Dependencies:** API-003, API-004 **Inputs:**

- Stripe error code mappings
- PCI compliance considerations for error messages

### API-009: Add error monitoring integration

**Intent:** Integrate error tracking service (Sentry) with automatic error
reporting **Expected Outcome:** All errors automatically sent to monitoring with
context (user, request, stack) **Confidence:** low **Scopes:** API, INFRA
**Tags:** monitoring, observability, sentry **Dependencies:** API-004
**Inputs:**

- Sentry account and DSN
- PII scrubbing rules
- Alert thresholds

### API-010: Update API documentation

**Intent:** Document new error response format and codes in API docs **Expected
Outcome:** API docs with error section, examples, and troubleshooting guide
**Confidence:** high **Scopes:** DOCS **Tags:** documentation, api
**Dependencies:** API-002, API-005, API-006, API-007, API-008

### API-011: Remove deprecated error handling code

**Intent:** Clean up old error handling patterns and remove dead code **Expected
Outcome:** Codebase with only new error handling, no legacy patterns remaining
**Confidence:** medium **Scopes:** API, AUTH, PROD, CART, PAY **Tags:** cleanup,
refactor, technical-debt **Dependencies:** API-005, API-006, API-007, API-008

## Dependencies

- Monitoring service setup (Sentry or alternative)
- Logging infrastructure (structured logging)

## Notes

- Consider rate limiting errors (e.g., don't log every 401)
- May need database migration if error codes are persisted
- Client applications will need updates to handle new error format
- Rollout strategy: shadow mode first (log but don't change responses)?
- Performance impact of error middleware needs measurement

## Open Questions

- Should we version the error response schema?
- How do we handle errors in background jobs vs API requests?
- Do we need different error formats for internal vs external APIs?
