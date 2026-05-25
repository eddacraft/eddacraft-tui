import { describe, expect, it } from 'vitest';

import {
  TraceparentParseError,
  attachTraceparentToEnvelope,
  formatTraceparent,
  isTraceparent,
  parseTraceparent,
  readTraceparentFromJsonRpcEnvelope,
  readTraceparentFromNotificationEnvelope,
} from './index.js';

const RUST_EMITTED = '00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01';

describe('parseTraceparent', () => {
  it('parses the canonical Rust-emitted traceparent shape', () => {
    const context = parseTraceparent(RUST_EMITTED);

    expect(context).toEqual({
      traceId: '0af7651916cd43dd8448eb211c80319c',
      parentId: 'b7ad6b7169203331',
      flags: 1,
      sampled: true,
      header: RUST_EMITTED,
    });
  });

  it('round-trips formatted headers byte-for-byte', () => {
    expect(
      formatTraceparent({
        traceId: '0af7651916cd43dd8448eb211c80319c',
        parentId: 'b7ad6b7169203331',
        flags: 1,
      })
    ).toBe(RUST_EMITTED);
  });

  it('rejects upper-case hex the same way the Rust parser does', () => {
    expect(() =>
      parseTraceparent('00-0AF7651916CD43DD8448EB211C80319C-b7ad6b7169203331-01')
    ).toThrowError(TraceparentParseError);
  });

  it('rejects all-zero ids and malformed lengths', () => {
    expect(() =>
      parseTraceparent('00-00000000000000000000000000000000-b7ad6b7169203331-00')
    ).toThrowError(expect.objectContaining({ code: 'all-zero-trace-id' }));
    expect(() =>
      parseTraceparent('00-0af7651916cd43dd8448eb211c80319c-0000000000000000-00')
    ).toThrowError(expect.objectContaining({ code: 'all-zero-parent-id' }));
    expect(() => parseTraceparent('00-too-short')).toThrowError(
      expect.objectContaining({ code: 'length' })
    );
  });

  it('reports boolean validation without leaking parser errors', () => {
    expect(isTraceparent(RUST_EMITTED)).toBe(true);
    expect(isTraceparent('00-not-a-real-traceparent')).toBe(false);
  });
});

describe('traceparent envelope helpers', () => {
  it('reads traceparent from JSON-RPC envelopes', () => {
    expect(
      readTraceparentFromJsonRpcEnvelope({
        jsonrpc: '2.0',
        id: 'req-1',
        method: 'anvil/scan_buffer',
        traceparent: RUST_EMITTED,
      })?.parentId
    ).toBe('b7ad6b7169203331');
    expect(readTraceparentFromJsonRpcEnvelope({ schema: 'anvil.notification.v1' })).toBeNull();
  });

  it('reads traceparent from notification correlation metadata when present', () => {
    expect(
      readTraceparentFromNotificationEnvelope({
        schema: 'anvil.notification.v1',
        correlation: { source: 'intercept', traceparent: RUST_EMITTED },
      })?.traceId
    ).toBe('0af7651916cd43dd8448eb211c80319c');
    expect(readTraceparentFromNotificationEnvelope({ jsonrpc: '2.0' })).toBeNull();
  });

  it('attaches a canonical traceparent to outgoing envelopes', () => {
    expect(attachTraceparentToEnvelope({ jsonrpc: '2.0', id: 1 }, RUST_EMITTED)).toEqual({
      jsonrpc: '2.0',
      id: 1,
      traceparent: RUST_EMITTED,
    });
  });
});
