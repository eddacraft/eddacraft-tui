# Authentication Module

**Scope:** AUTH **Owner:** @alice **Priority:** high

> Handles user authentication and session management.

## Tasks

### AUTH-001: Implement login endpoint

**Intent:** Create POST /auth/login endpoint with JWT response **Confidence:**
high **Expected Outcome:** Returns JWT token on success, 401 on failure
**Tags:** security, api

### AUTH-002: Add password reset

**Intent:** Implement password reset flow with email verification
**Confidence:** medium **Dependencies:** AUTH-001

## Notes

- Consider OAuth support in future
