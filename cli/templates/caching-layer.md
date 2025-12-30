---
id: caching-layer
name: Caching Layer
description: Implement caching with Redis or in-memory cache
category: infrastructure
tags: [cache, redis, performance, optimization]
variables:
  - name: cache_type
    description: Cache backend (redis, memory)
    default: redis
    required: false
  - name: default_ttl
    description: Default TTL in seconds
    default: 3600
    required: false
  - name: cache_prefix
    description: Cache key prefix
    default: app
    required: false
---

# Caching Layer Implementation

## Intent

Implement {{ cache_type }} caching layer to improve performance with
configurable TTL and cache invalidation strategies.

## Changes

### 1. Create Cache Service

- **File**: `src/services/cache.service.ts`
- **Action**: Create
- **Description**: Cache abstraction with get, set, delete, clear methods

### 2. Create Cache Adapter

- **File**: `src/services/cache/{{ cache_type }}.adapter.ts`
- **Action**: Create
- **Description**: {{ cache_type }} implementation

### 3. Create Cache Middleware

- **File**: `src/middleware/cache.middleware.ts`
- **Action**: Create
- **Description**: Request/response caching middleware

### 4. Create Cache Decorator

- **File**: `src/decorators/cacheable.decorator.ts`
- **Action**: Create
- **Description**: Method-level caching decorator

### 5. Add Configuration

- **File**: `src/config/cache.config.ts`
- **Action**: Create
- **Description**: Cache configuration (TTL, prefix, connection)

### 6. Add Tests

- **File**: `src/__tests__/cache.test.ts`
- **Action**: Create
- **Description**: Cache service tests

## Cache Interface

```typescript
interface CacheService {
  get<T>(key: string): Promise<T | null>;
  set<T>(key: string, value: T, ttl?: number): Promise<void>;
  delete(key: string): Promise<void>;
  clear(pattern?: string): Promise<void>;
  has(key: string): Promise<boolean>;
}
```

## Configuration

```typescript
const cacheConfig = {
  type: '{{ cache_type }}',
  prefix: '{{ cache_prefix }}',
  defaultTTL: {{ default_ttl }},
  // Redis-specific
  redis: {
    host: process.env.REDIS_HOST,
    port: process.env.REDIS_PORT,
  },
};
```

## Cache Key Strategy

```
{{ cache_prefix }}:<resource>:<identifier>:<version>
```

Example: `{{ cache_prefix }}:user:123:v1`

## Usage Examples

```typescript
// Direct usage
await cache.set('user:123', userData, 3600);
const user = await cache.get<User>('user:123');

// Decorator usage
@Cacheable({ ttl: 3600 })
async getUser(id: string): Promise<User> { ... }

// Middleware usage
app.get('/api/users/:id', cacheMiddleware(3600), getUser);
```

## Cache Invalidation

- [ ] TTL-based expiration
- [ ] Manual invalidation on updates
- [ ] Pattern-based clearing
- [ ] Event-driven invalidation

## Monitoring

- [ ] Cache hit/miss ratio tracking
- [ ] Memory usage monitoring
- [ ] Latency metrics
- [ ] Error tracking

## Acceptance Criteria

- [ ] Cache service functional
- [ ] TTL working correctly
- [ ] Invalidation working
- [ ] Performance improvement measured
- [ ] Tests passing
- [ ] No cache stampede issues
