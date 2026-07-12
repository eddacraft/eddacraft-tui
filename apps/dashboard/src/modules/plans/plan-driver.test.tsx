import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, expect, it, vi } from 'vitest';

import { PlanDetailView } from '@/modules/plans/plan-detail';

let root: Root | null = null;
let container: HTMLDivElement | null = null;

afterEach(() => {
  act(() => root?.unmount());
  container?.remove();
  root = null;
  container = null;
});

it('renders readiness evidence and keeps deferred actions inert', async () => {
  container = document.createElement('div');
  document.body.append(container);
  root = createRoot(container);
  await act(async () => {
    root?.render(
      <PlanDetailView
        detail={{
          schema_version: 'anvil.dashboard.plans.v1',
          summary: {
            id: 'dashboard',
            scope: 'DASH',
            title: 'Dashboard',
            status: 'Ready',
            progress: '1/2',
          },
          purpose: 'Ship the dashboard.',
          actions_enabled: false,
          action_message: 'Approval actions are deferred beyond Wave 1.',
          timeline: [
            {
              id: 'DASH-001',
              title: 'Proof',
              status: 'Ready',
              evidence: '`pnpm test`',
              readiness: true,
            },
          ],
        }}
      />
    );
  });
  const action = container.querySelector<HTMLButtonElement>('button');
  const click = vi.fn();
  action?.addEventListener('click', click);
  action?.click();
  expect(container.textContent).toContain('pnpm test');
  expect(action?.disabled).toBe(true);
  expect(click).not.toHaveBeenCalled();
});
