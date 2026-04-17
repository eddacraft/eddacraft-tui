import { describe, it, expect } from 'vitest';
import { formatError, formatJson, formatSuccess, renderTable, shouldUseColour } from '../format.js';

describe('renderTable', () => {
  it('returns empty string for no rows', () => {
    expect(renderTable([], [{ key: 'email' }])).toBe('');
  });

  it('renders header, divider, and padded cells', () => {
    const out = renderTable(
      [
        { email: 'a@b.c', count: 1 },
        { email: 'longer@example.com', count: 42 },
      ],
      [
        { key: 'email', header: 'EMAIL' },
        { key: 'count', header: 'COUNT' },
      ]
    );
    const lines = out.split('\n');
    expect(lines[0]).toBe('EMAIL               COUNT');
    expect(lines[1]).toBe('-'.repeat(18) + '  ' + '-'.repeat(5));
    expect(lines[2]).toBe('a@b.c               1    ');
    expect(lines[3]).toBe('longer@example.com  42   ');
  });

  it('applies custom column formatter', () => {
    const out = renderTable(
      [{ created_at: '2026-04-17T12:00:00Z' }],
      [{ key: 'created_at', header: 'DATE', format: (v) => String(v).slice(0, 10) }]
    );
    expect(out).toContain('2026-04-17');
    expect(out).not.toContain('T12');
  });

  it('renders null/undefined as empty cells', () => {
    const out = renderTable(
      [{ name: null }, { name: undefined }, { name: 'x' }],
      [{ key: 'name' }]
    );
    const lines = out.split('\n');
    expect(lines[2]!.trim()).toBe('');
    expect(lines[3]!.trim()).toBe('');
    expect(lines[4]!.trim()).toBe('x');
  });

  it('falls back to key when header is absent', () => {
    const out = renderTable([{ email: 'a@b.c' }], [{ key: 'email' }]);
    expect(out.split('\n')[0]).toBe('email');
  });

  it('collapses embedded control chars so rows stay single-line', () => {
    const out = renderTable(
      [
        { email: 'a@b.c', notes: 'line1\nline2\tindented' },
        { email: 'x@y.z', notes: 'ok' },
      ],
      [
        { key: 'email', header: 'EMAIL' },
        { key: 'notes', header: 'NOTES' },
      ]
    );
    const lines = out.split('\n');
    expect(lines).toHaveLength(4);
    expect(lines[2]).not.toMatch(/[\n\t]/);
    expect(lines[2]).toContain('line1 line2 indented');
    expect(lines[3]).toContain('ok');
    const widths = lines.map((l) => l.length);
    expect(widths[0]).toBe(widths[1]);
    expect(widths[0]).toBe(widths[2]);
    expect(widths[0]).toBe(widths[3]);
  });
});

describe('formatJson', () => {
  it('pretty-prints with 2-space indent', () => {
    expect(formatJson({ a: 1 })).toBe('{\n  "a": 1\n}');
  });
});

describe('shouldUseColour', () => {
  it('returns false when --json', () => {
    expect(shouldUseColour({ json: true }, true)).toBe(false);
  });

  it('returns false when --quiet', () => {
    expect(shouldUseColour({ quiet: true }, true)).toBe(false);
  });

  it('returns false when colour explicitly disabled', () => {
    expect(shouldUseColour({ colour: false }, true)).toBe(false);
  });

  it('returns false when NO_COLOR env set', () => {
    const prev = process.env.NO_COLOR;
    process.env.NO_COLOR = '1';
    try {
      expect(shouldUseColour({}, true)).toBe(false);
    } finally {
      if (prev === undefined) delete process.env.NO_COLOR;
      else process.env.NO_COLOR = prev;
    }
  });

  it('returns false when NO_COLOR env is empty string', () => {
    const prev = process.env.NO_COLOR;
    process.env.NO_COLOR = '';
    try {
      expect(shouldUseColour({}, true)).toBe(false);
    } finally {
      if (prev === undefined) delete process.env.NO_COLOR;
      else process.env.NO_COLOR = prev;
    }
  });

  it('returns false when not a TTY', () => {
    expect(shouldUseColour({}, false)).toBe(false);
  });

  it('returns true when TTY and no overrides', () => {
    const prev = process.env.NO_COLOR;
    delete process.env.NO_COLOR;
    try {
      expect(shouldUseColour({}, true)).toBe(true);
    } finally {
      if (prev !== undefined) process.env.NO_COLOR = prev;
    }
  });
});

describe('formatError / formatSuccess', () => {
  it('prefixes error messages', () => {
    expect(formatError('boom', { colour: false })).toBe('error: boom');
  });

  it('returns plain message on formatSuccess without colour', () => {
    expect(formatSuccess('ok', { colour: false })).toBe('ok');
  });
});
