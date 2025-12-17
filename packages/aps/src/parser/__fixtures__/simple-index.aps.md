# Simple Plan

## Overview

A simple plan with two modules for testing.

## Modules

### auth

- **Path:** [./modules/auth.aps.md](./modules/auth.aps.md)
- **Scope:** AUTH
- **Owner:** @alice
- **Priority:** high
- **Tags:** security, core
- **Dependencies:** (none)

### api

- **Path:** [./modules/api.aps.md](./modules/api.aps.md)
- **Scope:** API
- **Owner:** @bob
- **Priority:** medium
- **Tags:** backend
- **Dependencies:** auth

## Open Questions

- Should we add rate limiting?
- What authentication method to use?

## Decisions

- Using JWT tokens (decided 2025-01-15)
- PostgreSQL database (decided 2025-01-10)
