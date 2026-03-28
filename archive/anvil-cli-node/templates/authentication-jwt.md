---
id: authentication-jwt
name: JWT Authentication
description:
  Implement JWT-based authentication with login, registration, and token refresh
category: authentication
tags: [jwt, auth, security, login, token]
variables:
  - name: project_name
    description: Name of the project
    default: my-app
    required: true
  - name: token_expiry
    description: JWT token expiry duration
    default: 1h
    required: false
  - name: refresh_token_expiry
    description: Refresh token expiry duration
    default: 7d
    required: false
---

# JWT Authentication Implementation

## Intent

Implement secure JWT-based authentication for {{ project_name }} with login,
registration, and token refresh capabilities.

## Changes

### 1. Create Auth Controller

- **File**: `src/controllers/auth.controller.ts`
- **Action**: Create
- **Description**: Handle authentication endpoints (login, register, refresh,
  logout)

### 2. Create Auth Middleware

- **File**: `src/middleware/auth.middleware.ts`
- **Action**: Create
- **Description**: JWT verification middleware for protected routes

### 3. Create Auth Service

- **File**: `src/services/auth.service.ts`
- **Action**: Create
- **Description**: Business logic for authentication (token generation,
  validation)

### 4. Create User Model

- **File**: `src/models/user.model.ts`
- **Action**: Create
- **Description**: User schema with password hashing

### 5. Update Environment Configuration

- **File**: `.env`
- **Action**: Modify
- **Description**: Add JWT_SECRET, TOKEN_EXPIRY={{ token_expiry }},
  REFRESH_TOKEN_EXPIRY={{ refresh_token_expiry }}

### 6. Add Auth Routes

- **File**: `src/routes/auth.routes.ts`
- **Action**: Create
- **Description**: POST /auth/login, POST /auth/register, POST /auth/refresh,
  POST /auth/logout

## Dependencies

- `jsonwebtoken` - JWT generation and verification
- `bcryptjs` - Password hashing
- `express-validator` - Input validation

## Validation

- All endpoints return appropriate HTTP status codes
- Passwords are hashed before storage (never stored in plain text)
- Tokens include user ID and expiry
- Refresh tokens are stored securely
- Invalid tokens return 401 Unauthorized

## Security Considerations

- Use HTTP-only cookies for refresh tokens
- Implement rate limiting on auth endpoints
- Add CSRF protection
- Log authentication attempts
- Implement account lockout after failed attempts

## Acceptance Criteria

- [ ] User can register with email and password
- [ ] User can login and receive JWT token
- [ ] Protected routes reject invalid tokens
- [ ] Refresh token flow works correctly
- [ ] Passwords are securely hashed
- [ ] All auth tests passing
