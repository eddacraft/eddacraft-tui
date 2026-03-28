---
id: api-integration
name: Third-party API Integration
description:
  Integrate with external API with proper error handling and retry logic
category: integration
tags: [api, integration, http, retry, external]
variables:
  - name: api_name
    description: Name of the external API
    required: true
  - name: base_url
    description: API base URL
    required: true
  - name: auth_type
    description: Authentication type (api-key, oauth2, basic)
    default: api-key
    required: false
---

# API Integration: {{ api_name }}

## Intent

Integrate {{ api_name }} API with proper authentication, error handling, retry
logic, and rate limiting.

## Changes

### 1. Create API Client

- **File**: `src/clients/{{ api_name }}.client.ts`
- **Action**: Create
- **Description**: HTTP client wrapper for {{ api_name }} API

### 2. Create API Types

- **File**: `src/types/{{ api_name }}.types.ts`
- **Action**: Create
- **Description**: TypeScript types for API requests/responses

### 3. Create API Service

- **File**: `src/services/{{ api_name }}.service.ts`
- **Action**: Create
- **Description**: Business logic using {{ api_name }} API

### 4. Add Configuration

- **File**: `src/config/{{ api_name }}.config.ts`
- **Action**: Create
- **Description**: API configuration (base URL, credentials, timeouts)

### 5. Add Tests

- **File**: `src/__tests__/{{ api_name }}.test.ts`
- **Action**: Create
- **Description**: Integration tests with mocked responses

### 6. Update Environment

- **File**: `.env`
- **Action**: Modify
- **Description**: Add {{ api_name | uppercase }}\_API_KEY,
  {{ api_name | uppercase }}\_BASE_URL

## Configuration

```typescript
const {{ api_name }}Config = {
  baseUrl: '{{ base_url }}',
  authType: '{{ auth_type }}',
  timeout: 30000,
  retries: 3,
  retryDelay: 1000,
};
```

## Client Structure

```typescript
class {{ api_name }}Client {
  constructor(config: ApiConfig);

  async get<T>(endpoint: string, params?: object): Promise<T>;
  async post<T>(endpoint: string, data: object): Promise<T>;
  async put<T>(endpoint: string, data: object): Promise<T>;
  async delete(endpoint: string): Promise<void>;
}
```

## Error Handling

| HTTP Status | Action                             |
| ----------- | ---------------------------------- |
| 429         | Retry with exponential backoff     |
| 500-599     | Retry up to 3 times                |
| 401         | Refresh auth token (if applicable) |
| 400         | Return validation error            |

## Retry Strategy

```typescript
const retryConfig = {
  maxRetries: 3,
  baseDelay: 1000,
  maxDelay: 10000,
  retryOn: [429, 500, 502, 503, 504],
};
```

## Rate Limiting

- [ ] Track API quota usage
- [ ] Implement request queuing
- [ ] Handle 429 responses gracefully
- [ ] Log rate limit warnings

## Monitoring

- [ ] Request/response logging
- [ ] Error tracking
- [ ] Latency metrics
- [ ] Success rate tracking

## Acceptance Criteria

- [ ] Authentication working
- [ ] All endpoints integrated
- [ ] Error handling complete
- [ ] Retry logic functional
- [ ] Rate limiting handled
- [ ] Tests passing
