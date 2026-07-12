import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, describe, expect, it } from 'vitest';

import { CodeBlock } from '@/components/primitives/code-block';
import { EmptyState } from '@/components/primitives/empty-state';
import { LoadingSkeleton } from '@/components/primitives/loading-skeleton';
import { MetricCard } from '@/components/primitives/metric-card';
import { SeverityBadge } from '@/components/primitives/severity-badge';
import { StatusBadge } from '@/components/primitives/status-badge';
import { getDashboardComponentNames, validateDashboardSpec } from './catalog';

let root: Root | null = null;
let container: HTMLDivElement | null = null;

afterEach(() => {
  if (root) {
    act(() => root?.unmount());
  }
  container?.remove();
  root = null;
  container = null;
});

describe('dashboard render catalogue', () => {
  it('accepts known json-render components', () => {
    const result = validateDashboardSpec({
      root: 'metric',
      elements: {
        metric: {
          type: 'MetricCard',
          props: { label: 'Warnings', value: '12' },
          children: [],
        },
      },
    });

    expect(result).toEqual({ valid: true, errors: [] });
    expect(getDashboardComponentNames()).toContain('MetricCard');
    expect(getDashboardComponentNames()).toContain('Table');
  });

  it('rejects unknown json-render component names', () => {
    const result = validateDashboardSpec({
      root: 'unknown',
      elements: {
        unknown: { type: 'KernelDecisionButton', props: {}, children: [] },
      },
    });

    expect(result.valid).toBe(false);
    expect(result.errors.length).toBeGreaterThan(0);
  });
});

describe('dashboard shared primitives', () => {
  it('renders labelled, non-colour-only operational states', async () => {
    container = document.createElement('div');
    document.body.append(container);
    root = createRoot(container);

    await act(async () => {
      root?.render(
        <>
          <MetricCard label="Warnings" value="12" />
          <StatusBadge label="Protected" status="pass" />
          <SeverityBadge severity="high" />
          <CodeBlock code={'const safe = true;'} language="typescript" />
          <EmptyState description="Run Anvil to collect evidence." title="No evidence" />
          <LoadingSkeleton label="Loading protection evidence" rows={2} />
        </>
      );
    });

    expect(container.textContent).toContain('Warnings');
    expect(container.textContent).toContain('Protected');
    expect(container.textContent).toContain('High severity');
    expect(container.textContent).toContain('typescript');
    expect(container.textContent).toContain('No evidence');
    expect(container.querySelector('[aria-label="Loading protection evidence"]')).not.toBeNull();
  });
});
