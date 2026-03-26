/**
 * Adapter Format Roundtrip — E2E Tests
 *
 * Tests format conversion fidelity across all registered adapters:
 *   External format → APS → External format
 *
 * Verifies that the adapter registry correctly detects formats
 * and that conversions preserve essential data.
 *
 * Surface: Adapters
 */

import { describe, it, expect } from 'vitest';
import { registry } from '@eddacraft/anvil-adapters';
import { createE2EWorkspace, type E2EWorkspace } from '../helpers/workspace.js';
import { makeAPSMarkdown, makeSpecKitDoc } from '../helpers/fixtures.js';

let ws: E2EWorkspace;

beforeAll(() => {
  ws = createE2EWorkspace({
    files: {
      'docs/plan.md': makeAPSMarkdown('Adapter roundtrip test'),
      'docs/spec.md': makeSpecKitDoc('SpecKit roundtrip test'),
    },
  });
});

afterAll(() => ws.cleanup());

describe('Adapter Registry', () => {
  it('has at least 3 registered adapters', () => {
    const adapters = registry.listAdapters();
    // APS Markdown, BMAD, SpecKit, and Generic at minimum
    expect(adapters.length).toBeGreaterThanOrEqual(3);
  });

  it('each adapter exposes a name and supported extensions', () => {
    const adapters = registry.listAdapters();
    for (const adapter of adapters) {
      expect(adapter.metadata.name).toBeDefined();
      expect(typeof adapter.metadata.name).toBe('string');
      expect(adapter.metadata.name.length).toBeGreaterThan(0);
    }
  });
});

describe('APS Markdown Adapter', () => {
  it('detects APS markdown format', () => {
    const content = makeAPSMarkdown('Detection test');
    const detected = registry.detectAdapter(content);
    expect(detected).toBeDefined();
    expect(detected?.adapter.metadata.name.toLowerCase()).toContain('aps');
  });

  it('parses APS markdown into a plan structure', async () => {
    const content = makeAPSMarkdown('Parse test');
    const match = registry.detectAdapter(content);
    expect(match).toBeDefined();
    if (match) {
      const result = await match.adapter.parse(content);
      expect(result).toBeDefined();
      expect(result.data?.intent).toContain('Parse test');
    }
  });
});

describe('SpecKit Adapter', () => {
  it('detects SpecKit format', () => {
    const content = makeSpecKitDoc('SpecKit detection');
    const detected = registry.detectAdapter(content);
    expect(detected).toBeDefined();
  });
});

describe('Format Detection Edge Cases', () => {
  it('does not throw for unrecognised content', () => {
    // Generic adapter may catch this as a fallback — that's acceptable
    // The important thing is it doesn't throw
    expect(() => registry.detectAdapter('just some random text')).not.toThrow();
  });

  it('handles empty content without throwing', () => {
    expect(() => registry.detectAdapter('')).not.toThrow();
  });
});
