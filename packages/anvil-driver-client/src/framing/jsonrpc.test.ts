/**
 * JSON-RPC framer unit tests.
 *
 * Coverage: envelope construction, classification of incoming
 * shapes, error-from-response mapping (including retriable codes).
 */

import { describe, expect, it } from 'vitest';

import { DriverClientError } from '../errors.js';
import {
  buildNotification,
  buildRequest,
  classifyIncoming,
  encodeNdjsonLine,
  errorFromResponse,
} from './jsonrpc.js';

describe('buildRequest / buildNotification', () => {
  it('produces a valid JSON-RPC 2.0 request envelope', () => {
    const env = buildRequest('req-1', 'session.register', {
      session_id: 's1',
      worktree: '/tmp/wt',
    });
    expect(env).toEqual({
      jsonrpc: '2.0',
      id: 'req-1',
      method: 'session.register',
      params: { session_id: 's1', worktree: '/tmp/wt' },
    });
  });

  it('omits params when undefined', () => {
    const env = buildRequest(42, 'session.list');
    expect(env).toEqual({ jsonrpc: '2.0', id: 42, method: 'session.list' });
  });

  it('builds a notification without an id', () => {
    const env = buildNotification('anvil/correlation', { id: 'abc' });
    expect(env).toEqual({
      jsonrpc: '2.0',
      method: 'anvil/correlation',
      params: { id: 'abc' },
    });
    expect(env).not.toHaveProperty('id');
  });

  it('encodeNdjsonLine produces a single line ending in \\n', () => {
    const line = encodeNdjsonLine({ a: 1 });
    expect(line.endsWith('\n')).toBe(true);
    expect(line.split('\n').filter((s) => s.length > 0).length).toBe(1);
  });
});

describe('classifyIncoming', () => {
  it('routes a success response', () => {
    const out = classifyIncoming({ jsonrpc: '2.0', id: 'req-1', result: { ok: true } });
    expect(out.kind).toBe('response');
    if (out.kind === 'response') {
      expect((out.response as { result: { ok: boolean } }).result.ok).toBe(true);
    }
  });

  it('routes an error response', () => {
    const out = classifyIncoming({
      jsonrpc: '2.0',
      id: 'req-1',
      error: { code: -32_601, message: 'Method not found', data: { method: 'x' } },
    });
    expect(out.kind).toBe('response');
  });

  it('routes a notification', () => {
    const out = classifyIncoming({
      jsonrpc: '2.0',
      method: 'anvil/publishDiagnostics',
      params: { uri: 'file://x', diagnostics: [] },
    });
    expect(out.kind).toBe('notification');
    if (out.kind === 'notification') {
      expect(out.method).toBe('anvil/publishDiagnostics');
    }
  });

  it('returns unknown for non-object frames', () => {
    expect(classifyIncoming(null).kind).toBe('unknown');
    expect(classifyIncoming('string').kind).toBe('unknown');
    expect(classifyIncoming(42).kind).toBe('unknown');
  });

  it('returns unknown when jsonrpc field is missing', () => {
    expect(classifyIncoming({ id: 1, result: 'x' }).kind).toBe('unknown');
  });

  it('returns unknown when method is missing on a notification-shaped frame', () => {
    expect(classifyIncoming({ jsonrpc: '2.0', params: {} }).kind).toBe('unknown');
  });
});

describe('errorFromResponse', () => {
  it('marks server-busy as retriable', () => {
    const err = errorFromResponse({
      jsonrpc: '2.0',
      id: 'req-1',
      error: { code: -32_000, message: 'Server busy', data: {} },
    });
    expect(err).toBeInstanceOf(DriverClientError);
    expect(err.retriable).toBe(true);
    expect(err.code).toBe('anvil-daemon-error');
  });

  it('marks scan-timeout as retriable', () => {
    const err = errorFromResponse({
      jsonrpc: '2.0',
      id: 'req-1',
      error: { code: -32_001, message: 'Scan timed out', data: {} },
    });
    expect(err.retriable).toBe(true);
  });

  it('marks parse-error and invalid-request as non-retriable', () => {
    const parse = errorFromResponse({
      jsonrpc: '2.0',
      id: null,
      error: { code: -32_700, message: 'Parse error', data: {} },
    });
    expect(parse.retriable).toBe(false);
    const invalid = errorFromResponse({
      jsonrpc: '2.0',
      id: 'req-1',
      error: { code: -32_600, message: 'Invalid Request', data: {} },
    });
    expect(invalid.retriable).toBe(false);
  });

  it('marks method-not-found and invalid-params as non-retriable', () => {
    const notFound = errorFromResponse({
      jsonrpc: '2.0',
      id: 'req-1',
      error: { code: -32_601, message: 'Method not found', data: { method: 'x' } },
    });
    expect(notFound.retriable).toBe(false);
    const invalidParams = errorFromResponse({
      jsonrpc: '2.0',
      id: 'req-1',
      error: { code: -32_602, message: 'Invalid params', data: { reason: 'x' } },
    });
    expect(invalidParams.retriable).toBe(false);
  });

  it('preserves daemon error code and data', () => {
    const err = errorFromResponse({
      jsonrpc: '2.0',
      id: 'req-1',
      error: { code: -32_603, message: 'Internal error', data: { trace: 'xyz' } },
    });
    const payload = err.toJSON();
    expect(payload.error).toBe('anvil-daemon-error');
    expect(payload.data).toEqual({
      code: -32_603,
      daemon_data: { trace: 'xyz' },
    });
  });
});
