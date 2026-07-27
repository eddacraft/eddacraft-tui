import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, expect, it } from 'vitest';

import type { components } from '@/api/generated/openapi';
import { TrendCharts } from '@/modules/core/overview/trend-charts';

type History = components['schemas']['ProtectionHistory'];
let root: Root | null = null;
let container: HTMLDivElement | null = null;

afterEach(() => {
  act(() => root?.unmount());
  container?.remove();
  root = null;
  container = null;
});

async function render(history: History) {
  container = document.createElement('div');
  document.body.append(container);
  root = createRoot(container);
  await act(async () => root?.render(<TrendCharts history={history} />));
}

const complete: History = {
  schema_version: 'anvil.dashboard.protection-history.v1',
  data_state: 'complete',
  source_message: 'Two points.',
  actual_range: {
    first_recorded_at: '2026-07-01T00:00:00Z',
    last_recorded_at: '2026-07-03T00:00:00Z',
  },
  points: [
    {
      recorded_at: '2026-07-01T00:00:00Z',
      score: 100,
      status: 'pass',
      status_label: 'pass',
      warning_count: 0,
      duration_seconds: null,
      checks_run: null,
    },
    {
      recorded_at: '2026-07-03T00:00:00Z',
      score: 70,
      status: 'warn',
      status_label: 'warn',
      warning_count: 3,
      duration_seconds: null,
      checks_run: null,
    },
  ],
  gaps: [
    { component: 'drift-history', reason: 'not produced' },
    { component: 'suppression-history', reason: 'not produced' },
  ],
};

it('renders accessible gate and warning trends with actual-range copy and no drift chart', async () => {
  await render(complete);
  expect(container?.textContent).toContain('1 Jul 2026 – 3 Jul 2026');
  expect(container?.querySelector('[aria-label="Gate score trend"]')).not.toBeNull();
  expect(container?.querySelector('[aria-label="Warning count trend"]')).not.toBeNull();
  expect(container?.textContent).not.toContain('Drift trend');
  expect(container?.querySelector('[aria-label="Gate pass-rate trend"]')).not.toBeNull();
  expect(container?.querySelectorAll('[data-recharts-chart]')).toHaveLength(3);
  expect(container?.querySelector('polyline')).toBeNull();
  expect(container?.textContent).toContain('0%');
  expect(container?.querySelectorAll('[data-history-bucket]')).toHaveLength(6);
  for (const graphic of container?.querySelectorAll('[role="img"]') ?? []) {
    expect(graphic.querySelector('[data-history-bucket]')).toBeNull();
  }
  expect(container?.querySelectorAll('ul.history-chart-values')).toHaveLength(3);
});

it('renders partial and unavailable history honestly', async () => {
  await render({ ...complete, data_state: 'partial', source_message: '1 invalid line omitted.' });
  expect(container?.textContent).toContain('Partial retained history');
  expect(container?.textContent).toContain('1 invalid line omitted.');

  await act(async () =>
    root?.render(
      <TrendCharts
        history={{
          ...complete,
          data_state: 'unavailable',
          actual_range: null,
          points: [],
          source_message: 'No history yet.',
        }}
      />
    )
  );
  expect(container?.textContent).toContain('No retained gate history');
  expect(container?.textContent).toContain('No history yet.');
});
