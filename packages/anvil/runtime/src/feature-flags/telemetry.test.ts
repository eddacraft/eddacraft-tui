import { describe, expect, it } from 'vitest';

import { createSessionTelemetry, createEvaluationEvent, createOverrideEvent } from './telemetry.js';

// ---------------------------------------------------------------------------
// Session Telemetry
// ---------------------------------------------------------------------------

describe('createSessionTelemetry', () => {
  it('includes snapshot version and environment', () => {
    const tel = createSessionTelemetry({
      snapshotVersion: 42,
      environment: 'production',
      runtime: 'typescript',
    });
    expect(tel.snapshotVersion).toBe(42);
    expect(tel.environment).toBe('production');
    expect(tel.runtime).toBe('typescript');
    expect(tel.timestamp).toBeDefined();
  });

  it('does not include PII', () => {
    const tel = createSessionTelemetry({
      snapshotVersion: 1,
      environment: 'development',
      runtime: 'typescript',
    });
    const json = JSON.stringify(tel);
    // Should not contain user-identifying fields
    expect(json).not.toContain('email');
    expect(json).not.toContain('userId');
    expect(json).not.toContain('password');
  });
});

// ---------------------------------------------------------------------------
// Evaluation Event
// ---------------------------------------------------------------------------

describe('createEvaluationEvent', () => {
  it('captures flag key and resolved variant', () => {
    const event = createEvaluationEvent({
      flagKey: 'cli.licence-gate',
      variant: 'enabled',
      reason: 'targeting_match',
    });
    expect(event.flagKey).toBe('cli.licence-gate');
    expect(event.variant).toBe('enabled');
    expect(event.reason).toBe('targeting_match');
    expect(event.timestamp).toBeDefined();
  });

  it('does not include evaluation context details', () => {
    const event = createEvaluationEvent({
      flagKey: 'test.flag',
      variant: 'disabled',
      reason: 'default',
    });
    const json = JSON.stringify(event);
    expect(json).not.toContain('targetingKey');
    expect(json).not.toContain('audience');
  });
});

// ---------------------------------------------------------------------------
// Override Event
// ---------------------------------------------------------------------------

describe('createOverrideEvent', () => {
  it('captures override source and variant', () => {
    const event = createOverrideEvent({
      flagKey: 'cli.licence-gate',
      variant: 'disabled',
      source: 'emergency',
    });
    expect(event.flagKey).toBe('cli.licence-gate');
    expect(event.variant).toBe('disabled');
    expect(event.source).toBe('emergency');
    expect(event.timestamp).toBeDefined();
  });
});
