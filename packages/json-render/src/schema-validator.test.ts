import { describe, it, expect } from 'vitest';

import { validateSpec, getComponentNames } from './schema-validator.js';

describe('validateSpec', () => {
  it('accepts a minimal valid spec', () => {
    const spec = {
      root: 'main',
      elements: {
        main: {
          type: 'Stack',
          props: {},
          children: ['heading'],
        },
        heading: {
          type: 'Heading',
          props: { level: 2, children: 'Dashboard' },
          children: [],
        },
      },
    };

    const result = validateSpec(spec);

    expect(result.valid).toBe(true);
    expect(result.errors).toEqual([]);
  });

  it('rejects an invalid spec with root-level errors', () => {
    const result = validateSpec('not-an-object');

    expect(result.valid).toBe(false);
    expect(result.errors.length).toBeGreaterThan(0);
    expect(result.errors.some((e) => e.startsWith('(root)'))).toBe(true);
  });

  it('rejects a spec missing required fields', () => {
    const spec = { elements: {} };

    const result = validateSpec(spec);

    expect(result.valid).toBe(false);
    expect(result.errors.length).toBeGreaterThan(0);
  });

  it('accepts a MetricCard spec that omits optional trend/format', () => {
    const spec = {
      root: 'card',
      elements: {
        card: {
          type: 'MetricCard',
          props: { label: 'Violations', value: '42' },
          children: [],
        },
      },
    };

    const result = validateSpec(spec);

    expect(result.valid).toBe(true);
    expect(result.errors).toEqual([]);
  });

  it('accepts a MetricCard spec with trend and format', () => {
    const spec = {
      root: 'card',
      elements: {
        card: {
          type: 'MetricCard',
          props: { label: 'Pass Rate', value: '94%', trend: 'up', format: 'percent' },
          children: [],
        },
      },
    };

    const result = validateSpec(spec);

    expect(result.valid).toBe(true);
    expect(result.errors).toEqual([]);
  });
});

describe('getComponentNames', () => {
  it('includes custom Anvil components', () => {
    const names = getComponentNames();

    expect(names).toContain('MetricCard');
    expect(names).toContain('StatusBadge');
  });

  it('includes shadcn built-in components', () => {
    const names = getComponentNames();

    expect(names).toContain('Card');
    expect(names).toContain('Stack');
    expect(names).toContain('Grid');
  });
});
