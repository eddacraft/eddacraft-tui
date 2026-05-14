import { describe, expect, it } from 'vitest';

import {
  ANVIL_AGENT_TAG_ENV,
  ANVIL_TASK_ID_ENV,
  type AgentTag,
  makeAgentTag,
  parseAgentTag,
} from './types.js';

/**
 * MLP2-029: cross-language parity tests for the AgentTag wire shape.
 *
 * The reference JSON below is the byte-exact output of the Rust
 * `serde_json::to_string(&AgentTag::new("anvil-run",
 * "claude-code-1", 1_700_000_042))` from the
 * `agent_tag_round_trips_through_json` test in
 * `crates/anvil-intercept-proto/src/session.rs`. Keeping this
 * fixture string verbatim means the parity test catches any drift
 * if a future Rust serde rename (`#[serde(rename = "...")]` or
 * `#[serde(rename_all)]`) lands without the TS mirror being
 * updated.
 */
const RUST_EMITTED_AGENT_TAG_JSON =
  '{"driver_id":"anvil-run","claimed_agent_id":"claude-code-1","pid_starttime":1700000042}';

const RUST_EQUIVALENT_AGENT_TAG: AgentTag = {
  driver_id: 'anvil-run',
  claimed_agent_id: 'claude-code-1',
  pid_starttime: 1_700_000_042,
};

describe('session env-var constants', () => {
  it('matches the Rust ANVIL_AGENT_TAG_ENV constant', () => {
    expect(ANVIL_AGENT_TAG_ENV).toBe('ANVIL_AGENT_TAG');
  });

  it('matches the Rust ANVIL_TASK_ID_ENV constant', () => {
    expect(ANVIL_TASK_ID_ENV).toBe('ANVIL_TASK_ID');
  });
});

describe('parseAgentTag (Rust → TS parity)', () => {
  it('round-trips the Rust serde JSON shape losslessly', () => {
    const parsed = parseAgentTag(JSON.parse(RUST_EMITTED_AGENT_TAG_JSON));
    expect(parsed).toEqual(RUST_EQUIVALENT_AGENT_TAG);
  });

  it('drops unknown future fields silently (forward-compat)', () => {
    // Mirrors the Rust struct's lack of `#[serde(deny_unknown_fields)]`
    // so a future Rust-side addition can land before TS catches up.
    const extended = {
      ...JSON.parse(RUST_EMITTED_AGENT_TAG_JSON),
      future_field: 'should be ignored',
      nested_future: { inner: 1 },
    };
    const parsed = parseAgentTag(extended);
    expect(parsed).toEqual(RUST_EQUIVALENT_AGENT_TAG);
  });

  it('rejects null / non-object input', () => {
    expect(() => parseAgentTag(null)).toThrow(TypeError);
    expect(() => parseAgentTag(42)).toThrow(TypeError);
    expect(() => parseAgentTag('not-an-object')).toThrow(TypeError);
    // Arrays are `typeof 'object'` so they pass the object-type
    // guard and fall through to the missing-required-field checks;
    // we pin that they fail on the first required-field lookup with
    // a typed error mentioning the field name (here `driver_id`).
    expect(() => parseAgentTag([])).toThrow(/driver_id/);
  });

  it('rejects each missing required field with a typed error', () => {
    expect(() =>
      parseAgentTag({
        claimed_agent_id: 'x',
        pid_starttime: 1,
      } as unknown)
    ).toThrow(/driver_id/);
    expect(() =>
      parseAgentTag({
        driver_id: 'x',
        pid_starttime: 1,
      } as unknown)
    ).toThrow(/claimed_agent_id/);
    expect(() =>
      parseAgentTag({
        driver_id: 'x',
        claimed_agent_id: 'y',
      } as unknown)
    ).toThrow(/pid_starttime/);
  });

  it('rejects non-integer pid_starttime', () => {
    expect(() =>
      parseAgentTag({
        ...RUST_EQUIVALENT_AGENT_TAG,
        pid_starttime: 1.5,
      } as unknown)
    ).toThrow(/pid_starttime/);
    expect(() =>
      parseAgentTag({
        ...RUST_EQUIVALENT_AGENT_TAG,
        pid_starttime: -1,
      } as unknown)
    ).toThrow(/pid_starttime/);
    expect(() =>
      parseAgentTag({
        ...RUST_EQUIVALENT_AGENT_TAG,
        pid_starttime: Number.POSITIVE_INFINITY,
      } as unknown)
    ).toThrow(/pid_starttime/);
    expect(() =>
      parseAgentTag({
        ...RUST_EQUIVALENT_AGENT_TAG,
        pid_starttime: Number.NaN,
      } as unknown)
    ).toThrow(/pid_starttime/);
  });
});

describe('makeAgentTag (TS → Rust parity)', () => {
  it('produces the same JSON shape the Rust side emits', () => {
    const tag = makeAgentTag('anvil-run', 'claude-code-1', 1_700_000_042);
    // JSON.stringify field order is insertion order for plain objects
    // in modern JS engines; `makeAgentTag` constructs the object in
    // the same order Rust's serde emits, so the serialised string
    // matches byte-for-byte.
    expect(JSON.stringify(tag)).toBe(RUST_EMITTED_AGENT_TAG_JSON);
  });

  it('round-trips through parse(make(...)) without loss', () => {
    const tag = makeAgentTag('anvil-run', 'claude-code-1', 1_700_000_042);
    const wire = JSON.stringify(tag);
    const reparsed = parseAgentTag(JSON.parse(wire));
    expect(reparsed).toEqual(tag);
  });

  it('treats different pid_starttime as a distinct tag (PID-reuse defence)', () => {
    // Mirrors the Rust `distinct_pid_starttimes_produce_distinct_tags`
    // test pinning the Eq invariant.
    const a = makeAgentTag('anvil-run', 'claude-1', 1_700_000_000);
    const b = makeAgentTag('anvil-run', 'claude-1', 1_700_000_001);
    expect(a).not.toEqual(b);
  });
});
