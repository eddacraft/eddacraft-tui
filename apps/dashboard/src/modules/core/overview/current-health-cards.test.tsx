import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, describe, expect, it } from 'vitest';

import type { components } from '@/api/generated/openapi';
import { protectionOverviewFixture } from '@/api/fixtures';
import { CurrentHealthCards } from '@/modules/core/overview/current-health-cards';

type Overview = components['schemas']['ProtectionOverview'];

let root: Root | null = null;
let container: HTMLDivElement | null = null;

afterEach(() => {
  act(() => root?.unmount());
  container?.remove();
  root = null;
  container = null;
});

async function render(overrides: Partial<Overview> = {}) {
  container = document.createElement('div');
  document.body.append(container);
  root = createRoot(container);
  await act(async () => {
    root?.render(<CurrentHealthCards overview={{ ...protectionOverviewFixture, ...overrides }} />);
  });
}

function card(label: string) {
  return container?.querySelector(`[aria-label="${label}"]`);
}

describe('CurrentHealthCards', () => {
  it('renders five current facts from the typed protection resource', async () => {
    await render({ observed_at_unix: Date.UTC(2026, 6, 13, 8, 30) / 1000 });

    expect(card('Save-time protection')?.textContent).toContain('Active');
    expect(card('Latest gate')?.textContent).toContain('72/100');
    expect(card('Active warnings')?.textContent).toContain('1');
    expect(card('Workspace assurance')?.textContent).toContain('100%');
    expect(card('Evidence freshness')?.textContent).toContain('13 Jul 2026');
    expect(container?.querySelectorAll('.metric-card')).toHaveLength(5);
    expect(container?.querySelector('[data-sparkline]')).toBeNull();
  });

  it('labels a partial warning resource as a shown subset', async () => {
    await render({ warnings_state: 'partial' });

    expect(card('Active warnings')?.textContent).toContain('1 shown');
    expect(card('Active warnings')?.textContent).toContain('Partial warning history');
    expect(card('Active warnings')?.getAttribute('data-state')).toBe('partial');
  });

  it('renders an observed inactive save-time driver consistently as a complete fact', async () => {
    await render({ save_time: { active: false, failure_count: 1, state: 'failed' } });

    expect(card('Save-time protection')?.textContent).toContain('Not observed');
    expect(card('Save-time protection')?.textContent).toContain('Failed');
    expect(card('Save-time protection')?.getAttribute('data-state')).toBe('complete');
  });

  it('renders missing save-time data as unavailable', async () => {
    await render({ save_time: null });

    expect(card('Save-time protection')?.textContent).toContain('Not observed');
    expect(card('Save-time protection')?.textContent).toContain('Live state unavailable');
    expect(card('Save-time protection')?.getAttribute('data-state')).toBe('unavailable');
  });

  it('does not render a zero observation timestamp as Unix epoch evidence', async () => {
    await render({ observed_at_unix: 0 });

    expect(card('Evidence freshness')?.textContent).toContain('Not observed');
    expect(card('Evidence freshness')?.getAttribute('data-state')).toBe('unavailable');
  });

  it('does not turn unavailable facts into zeroes', async () => {
    await render({
      assurance: null,
      latest_run: null,
      observed_at_unix: null,
      warnings: [],
      warnings_state: 'unavailable',
    });

    expect(card('Latest gate')?.textContent).toContain('Unavailable');
    expect(card('Active warnings')?.textContent).toContain('Unavailable');
    expect(card('Workspace assurance')?.textContent).toContain('Unavailable');
    expect(card('Evidence freshness')?.textContent).toContain('Not observed');
    expect(container?.textContent).not.toContain('0 warnings');
  });

  it('shows zero warnings only when the warning resource is complete', async () => {
    await render({ warnings: [], warnings_state: 'complete' });

    expect(card('Active warnings')?.textContent).toContain('0');
    expect(card('Active warnings')?.textContent).toContain('Complete warning resource');
    expect(card('Active warnings')?.getAttribute('data-state')).toBe('complete');
  });
});
