/**
 * USAGE-001: contract round-trip for the `command.invoked` observation.
 *
 * Guards the Rust ↔ TS wire contract for the usage-analytics row: the
 * Rust producer (`crates/anvil-cli/src/usage.rs`) emits exactly this
 * shape, and the Zod schema here must accept it. In particular this
 * pins the two contract points the Council review flagged:
 *   - timestamps are `Z`-suffixed RFC 3339 (NOT the `+00:00` offset
 *     form, which `z.string().datetime()` rejects);
 *   - argument values are never present — only redacted shapes.
 */

import { describe, expect, it } from 'vitest';
import { CommandInvokedObservationSchema } from './observation-contract.js';

/** A payload byte-shaped like what the Rust producer emits. */
function sampleRow() {
  return {
    kind: 'command.invoked' as const,
    session_id: '33333333-3333-4333-8333-333333333333',
    timestamp: '2026-06-14T10:00:00.000Z',
    command: 'check',
    principal: 'anonymous',
    args: [
      { name: 'path', shape: 'string' as const, length: 'medium' as const, present: true },
      { name: 'json', shape: 'flag' as const },
      { name: 'token', redacted: '<redacted>' as const },
    ],
    flag_set: [],
  };
}

describe('CommandInvokedObservationSchema', () => {
  it('accepts a Z-suffixed producer row', () => {
    const result = CommandInvokedObservationSchema.safeParse(sampleRow());
    expect(result.success).toBe(true);
  });

  it('rejects the +00:00 offset timestamp form (why the producer uses Z)', () => {
    const row = { ...sampleRow(), timestamp: '2026-06-14T10:00:00+00:00' };
    expect(CommandInvokedObservationSchema.safeParse(row).success).toBe(false);
  });

  it('rejects an unexpected top-level field (strict guardrail)', () => {
    const row = { ...sampleRow(), raw_args: ['--token', 'super-secret'] };
    expect(CommandInvokedObservationSchema.safeParse(row).success).toBe(false);
  });

  it('rejects an unexpected field inside an arg shape (strict guardrail)', () => {
    const row = sampleRow();
    row.args = [
      { name: 'path', shape: 'string', length: 'medium', present: true, value: '/secret' },
    ] as never;
    expect(CommandInvokedObservationSchema.safeParse(row).success).toBe(false);
  });

  it('requires flag_set to be present (never omitted)', () => {
    const { flag_set: _omit, ...withoutFlagSet } = sampleRow();
    expect(CommandInvokedObservationSchema.safeParse(withoutFlagSet).success).toBe(false);
  });

  it('round-trips a redacted argument without any raw value', () => {
    const parsed = CommandInvokedObservationSchema.parse(sampleRow());
    const token = parsed.args.find((a) => a.name === 'token');
    expect(token?.redacted).toBe('<redacted>');
    // A redacted arg carries no shape/length/value.
    expect(token?.shape).toBeUndefined();
    expect(token?.length).toBeUndefined();
    // The serialised row never contains an exact length number for the
    // non-redacted arg — only the coarse bucket.
    expect(JSON.stringify(parsed)).not.toContain('"value_len"');
  });
});
