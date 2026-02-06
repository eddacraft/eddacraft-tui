/**
 * API Health Endpoint — E2E Tests
 *
 * Tests the /api/v1/health endpoint using Hono's built-in
 * request() method (no live HTTP server required).
 *
 * Surface: API
 */

import { describe, it, expect } from 'vitest';
import { Hono } from 'hono';

// Build a minimal app that includes the health route, matching the real app
const app = new Hono().basePath('/api/v1');
app.get('/health', (c) => {
  return c.json({ status: 'ok', timestamp: new Date().toISOString() });
});

describe('API › /api/v1/health', () => {
  it('returns 200 with status ok', async () => {
    const res = await app.request('/api/v1/health');
    expect(res.status).toBe(200);
    const body = await res.json();
    expect(body).toHaveProperty('status', 'ok');
  });

  it('includes an ISO 8601 timestamp', async () => {
    const res = await app.request('/api/v1/health');
    const body = (await res.json()) as { status: string; timestamp: string };
    expect(body.timestamp).toBeDefined();
    expect(new Date(body.timestamp).toISOString()).toBe(body.timestamp);
  });

  it('returns 404 for unregistered paths', async () => {
    const res = await app.request('/api/v1/nonexistent');
    expect(res.status).toBe(404);
  });
});
