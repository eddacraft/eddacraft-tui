import { describe, expect, it } from 'vitest';
import { Hono } from 'hono';

import {
  TRACE_RESPONSE_HEADER,
  getTraceContext,
  traceContext,
} from '../middleware/trace-context.js';

const TRACEPARENT = '00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01';

describe('traceContext middleware', () => {
  it('parses a Rust-compatible traceparent at the request entry path', async () => {
    const app = new Hono();
    app.use('*', traceContext);
    app.get('/probe', (c) => {
      const context = getTraceContext(c);
      return c.json({
        traceId: context?.traceId,
        parentId: context?.parentId,
        sampled: context?.sampled,
      });
    });

    const response = await app.request('/probe', {
      headers: { traceparent: TRACEPARENT },
    });

    expect(response.status).toBe(200);
    expect(response.headers.get(TRACE_RESPONSE_HEADER)).toBe(TRACEPARENT);
    expect(await response.json()).toEqual({
      traceId: '0af7651916cd43dd8448eb211c80319c',
      parentId: 'b7ad6b7169203331',
      sampled: true,
    });
  });

  it('rejects malformed traceparent headers before route handlers run', async () => {
    const app = new Hono();
    app.use('*', traceContext);
    app.get('/probe', (c) => c.json({ reached: true }));

    const response = await app.request('/probe', {
      headers: { traceparent: '00-not-a-real-traceparent' },
    });

    expect(response.status).toBe(400);
    expect(await response.json()).toEqual({
      error: 'Invalid traceparent',
      code: 'length',
    });
  });
});
