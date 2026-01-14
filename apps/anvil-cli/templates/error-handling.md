---
id: error-handling
name: Error Handling
description:
  Implement comprehensive error handling with custom errors and logging
category: infrastructure
tags: [error, exception, logging, monitoring]
variables:
  - name: project_name
    description: Name of the project
    default: my-app
    required: false
  - name: error_tracking
    description: Error tracking service (sentry, bugsnag, none)
    default: sentry
    required: false
---

# Error Handling Implementation

## Intent

Implement comprehensive error handling for {{ project_name }} with custom error
classes, centralised handling, and {{ error_tracking }} integration.

## Changes

### 1. Create Error Classes

- **File**: `src/errors/index.ts`
- **Action**: Create
- **Description**: Custom error classes (AppError, ValidationError,
  NotFoundError, etc.)

### 2. Create Error Handler Middleware

- **File**: `src/middleware/error.middleware.ts`
- **Action**: Create
- **Description**: Centralised error handling middleware

### 3. Create Error Response Formatter

- **File**: `src/utils/error-response.ts`
- **Action**: Create
- **Description**: Format errors for API responses

### 4. Create Logger Service

- **File**: `src/services/logger.service.ts`
- **Action**: Create
- **Description**: Structured logging with error context

### 5. Add Error Tracking Integration

- **File**: `src/services/error-tracking.ts`
- **Action**: Create
- **Description**: {{ error_tracking }} integration

### 6. Add Tests

- **File**: `src/__tests__/errors.test.ts`
- **Action**: Create
- **Description**: Error handling tests

## Error Class Hierarchy

```typescript
class AppError extends Error {
  constructor(
    message: string,
    public statusCode: number = 500,
    public code: string = 'INTERNAL_ERROR',
    public isOperational: boolean = true
  ) {
    super(message);
  }
}

class ValidationError extends AppError {
  constructor(
    message: string,
    public details: ValidationDetail[]
  ) {
    super(message, 400, 'VALIDATION_ERROR');
  }
}

class NotFoundError extends AppError {
  constructor(resource: string) {
    super(`${resource} not found`, 404, 'NOT_FOUND');
  }
}

class UnauthorizedError extends AppError {
  constructor(message = 'Unauthorized') {
    super(message, 401, 'UNAUTHORIZED');
  }
}
```

## Error Response Format

```json
{
  "success": false,
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Invalid input data",
    "details": [...],
    "requestId": "abc123"
  }
}
```

## Error Codes

| Code             | Status | Description             |
| ---------------- | ------ | ----------------------- |
| VALIDATION_ERROR | 400    | Invalid input           |
| UNAUTHORIZED     | 401    | Authentication required |
| FORBIDDEN        | 403    | Access denied           |
| NOT_FOUND        | 404    | Resource not found      |
| CONFLICT         | 409    | Resource conflict       |
| RATE_LIMITED     | 429    | Too many requests       |
| INTERNAL_ERROR   | 500    | Server error            |

## Logging Levels

| Level | Use Case                         |
| ----- | -------------------------------- |
| error | Operational errors, exceptions   |
| warn  | Recoverable issues, deprecations |
| info  | Key business events              |
| debug | Development debugging            |

## {{ error_tracking }} Integration

```typescript
import * as Sentry from '@sentry/node';

Sentry.init({
  dsn: process.env.SENTRY_DSN,
  environment: process.env.NODE_ENV,
});
```

## Acceptance Criteria

- [ ] Custom error classes defined
- [ ] Error middleware handling all errors
- [ ] Proper HTTP status codes returned
- [ ] Errors logged with context
- [ ] {{ error_tracking }} integration working
- [ ] Tests passing
