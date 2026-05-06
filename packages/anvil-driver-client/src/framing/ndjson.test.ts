/**
 * NDJSON framer unit tests.
 *
 * Coverage:
 *   - Happy path: complete frames, multi-line chunks, CR-LF line ends
 *   - Partial frames: chunk-boundary join
 *   - Failure modes: malformed JSON, invalid UTF-8, oversize line,
 *     reset-with-pending-bytes
 *
 * Pinned by DRVR-001 brief: "NDJSON framer on parse error discards
 * the frame, emits a `framing-error` event, preserves the connection."
 */

import { describe, expect, it } from 'vitest';

import { NdjsonFramer, type FramingError } from './ndjson.js';

interface Recorded {
  frames: unknown[];
  errors: FramingError[];
}

function record(maxLineBytes?: number): { framer: NdjsonFramer; events: Recorded } {
  const events: Recorded = { frames: [], errors: [] };
  const framer = new NdjsonFramer(
    {
      onFrame: (v) => events.frames.push(v),
      onError: (err) => events.errors.push(err),
    },
    maxLineBytes !== undefined ? { maxLineBytes } : {}
  );
  return { framer, events };
}

describe('NdjsonFramer happy paths', () => {
  it('emits one frame per complete line', () => {
    const { framer, events } = record();
    framer.push(Buffer.from('{"a":1}\n{"b":2}\n'));
    expect(events.frames).toEqual([{ a: 1 }, { b: 2 }]);
    expect(events.errors).toEqual([]);
  });

  it('joins partial frames split across chunks', () => {
    const { framer, events } = record();
    framer.push(Buffer.from('{"jsonrpc":"2.0","id":'));
    framer.push(Buffer.from('"req-1","method":"x","params":'));
    framer.push(Buffer.from('null}\n'));
    expect(events.frames).toEqual([{ jsonrpc: '2.0', id: 'req-1', method: 'x', params: null }]);
    expect(events.errors).toEqual([]);
  });

  it('strips a trailing CR before parsing (CRLF tolerance)', () => {
    const { framer, events } = record();
    framer.push(Buffer.from('{"a":1}\r\n'));
    expect(events.frames).toEqual([{ a: 1 }]);
    expect(events.errors).toEqual([]);
  });

  it('skips empty lines without emitting events', () => {
    const { framer, events } = record();
    framer.push(Buffer.from('\n\n{"a":1}\n\n'));
    expect(events.frames).toEqual([{ a: 1 }]);
    expect(events.errors).toEqual([]);
  });

  it('handles utf-8 multi-byte characters split across chunks', () => {
    // The string "héllo" has 'é' as 2 UTF-8 bytes (0xc3 0xa9). Split
    // across chunks to make sure the framer doesn't try to decode
    // mid-character.
    const { framer, events } = record();
    framer.push(Buffer.from('{"name":"h\xc3', 'binary'));
    framer.push(Buffer.from('\xa9llo"}\n', 'binary'));
    expect(events.frames).toEqual([{ name: 'héllo' }]);
    expect(events.errors).toEqual([]);
  });
});

describe('NdjsonFramer rejection paths', () => {
  it('emits invalid-json on malformed line and continues to next', () => {
    const { framer, events } = record();
    framer.push(Buffer.from('{not json}\n{"a":1}\n'));
    expect(events.frames).toEqual([{ a: 1 }]);
    expect(events.errors.length).toBe(1);
    expect(events.errors[0]?.reason).toBe('invalid-json');
  });

  it('emits invalid-utf8 on a non-UTF-8 line and continues', () => {
    const { framer, events } = record();
    // 0xff is never valid in UTF-8.
    const bad = Buffer.from([0x7b, 0xff, 0x7d, 0x0a]);
    const good = Buffer.from('{"ok":true}\n');
    framer.push(bad);
    framer.push(good);
    expect(events.frames).toEqual([{ ok: true }]);
    expect(events.errors.length).toBe(1);
    expect(events.errors[0]?.reason).toBe('invalid-utf8');
  });

  it('emits oversize-line and recovers on next line', () => {
    const { framer, events } = record(64);
    // Long line first.
    framer.push(Buffer.from('{"x":"' + 'A'.repeat(200) + '"}\n'));
    framer.push(Buffer.from('{"ok":true}\n'));
    expect(events.frames).toEqual([{ ok: true }]);
    expect(events.errors.length).toBe(1);
    expect(events.errors[0]?.reason).toBe('oversize-line');
    expect(events.errors[0]?.bytes).toBeGreaterThan(64);
  });

  it('handles consecutive parse errors without losing the next valid line', () => {
    const { framer, events } = record();
    framer.push(Buffer.from('{bad}\n{also bad}\n{"ok":1}\n'));
    expect(events.frames).toEqual([{ ok: 1 }]);
    expect(events.errors.length).toBe(2);
    expect(events.errors.every((e) => e.reason === 'invalid-json')).toBe(true);
  });

  it('does not emit a frame for a partial line that never terminates', () => {
    const { framer, events } = record();
    framer.push(Buffer.from('{"a":1}'));
    expect(events.frames).toEqual([]);
    expect(events.errors).toEqual([]);
  });

  it('reset() surfaces pending bytes as partial-frame-on-reset', () => {
    const { framer, events } = record();
    framer.push(Buffer.from('{"a":1}'));
    framer.reset();
    expect(events.frames).toEqual([]);
    expect(events.errors.length).toBe(1);
    expect(events.errors[0]?.reason).toBe('partial-frame-on-reset');
  });

  it('reset() does not surface anything when buffer is empty', () => {
    const { framer, events } = record();
    framer.reset();
    expect(events.errors).toEqual([]);
  });

  it('rejects non-positive maxLineBytes at construction', () => {
    expect(
      () =>
        new NdjsonFramer(
          { onFrame: () => undefined, onError: () => undefined },
          { maxLineBytes: 0 }
        )
    ).toThrow(RangeError);
  });
});

describe('NdjsonFramer fuzz-shaped cases', () => {
  it('survives a stream of intermixed valid and malformed frames', () => {
    const { framer, events } = record();
    const lines = [
      '{"v":1}',
      '{not json}',
      '{"v":2}',
      '"not an object"',
      '{"v":3}',
      '{"v":incomplete',
      '',
      '{"v":4}',
    ];
    framer.push(Buffer.from(lines.join('\n') + '\n'));
    // "not an object" parses as the string `"not an object"` which is
    // a valid JSON value; the framer is JSON-RPC-agnostic at this
    // layer, so it surfaces it as a frame. Higher-level classification
    // catches non-object frames as `unknown`.
    const objectFrames = events.frames.filter(
      (v): v is { v: number } =>
        typeof v === 'object' && v !== null && 'v' in (v as Record<string, unknown>)
    );
    expect(objectFrames.map((f) => f.v)).toEqual([1, 2, 3, 4]);
    // We expect at least one parse error from the broken line; the
    // empty line in the middle is silently skipped.
    expect(events.errors.length).toBeGreaterThan(0);
    expect(events.errors.every((e) => e.reason === 'invalid-json')).toBe(true);
  });

  it('handles a very large valid line up to the cap', () => {
    const cap = 4096;
    const { framer, events } = record(cap);
    const big = '{"v":"' + 'x'.repeat(cap - 10) + '"}\n';
    expect(big.length).toBeLessThanOrEqual(cap + 1);
    framer.push(Buffer.from(big));
    expect(events.errors).toEqual([]);
    expect(events.frames.length).toBe(1);
  });

  it('does not buffer past the cap when newline never arrives', () => {
    const cap = 64;
    const { framer, events } = record(cap);
    framer.push(Buffer.from('A'.repeat(100)));
    // Without a newline the framer should have surfaced an oversize
    // error already (the cap fires when accumulated bytes exceed it),
    // so a subsequent push must NOT re-trigger.
    framer.push(Buffer.from('B'.repeat(50) + '\n{"ok":1}\n'));
    expect(events.frames).toEqual([{ ok: 1 }]);
    expect(events.errors.filter((e) => e.reason === 'oversize-line').length).toBe(1);
  });
});
