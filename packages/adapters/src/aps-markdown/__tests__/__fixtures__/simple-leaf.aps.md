# Authentication Feature

**Scope:** AUTH **Owner:** @alice **Priority:** high

## Tasks

### AUTH-001: Implement login endpoint

**Intent:** Create POST /auth/login endpoint with JWT response **Expected
Outcome:** Returns JWT token on success, 401 on failure **Validation:**
`pnpm test -- --grep "login"` **Confidence:** high **Tags:** security, api
**Files:** src/auth/login.ts, src/auth/jwt.ts

### AUTH-002: Add password reset

**Intent:** Implement password reset flow with email verification
**Confidence:** medium **Dependencies:** AUTH-001
