import { QueryClient } from '@tanstack/react-query';
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, describe, expect, it, vi } from 'vitest';

import type { DashboardApi } from '@/api/client';
import { DashboardQueryProvider } from '@/api/query-client';
import { dashboardQueryKeys } from '@/api/query-keys';
import { QueryBoundary } from '@/components/query-boundary';
import { useProtectionOverview } from '@/hooks/use-protection-overview';

let root: Root | null = null;
let container: HTMLDivElement | null = null;

afterEach(() => {
  act(() => root?.unmount());
  container?.remove();
  root = null;
  container = null;
});

function Probe() {
  const query = useProtectionOverview();
  return (
    <QueryBoundary query={query} loadingLabel="Loading protection">
      {(overview) => <p>{overview.source_message}</p>}
    </QueryBoundary>
  );
}

async function renderWith(api: DashboardApi) {
  container = document.createElement('div');
  document.body.append(container);
  root = createRoot(container);
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  await act(async () => {
    root?.render(
      <DashboardQueryProvider api={api} queryClient={queryClient}>
        <Probe />
      </DashboardQueryProvider>
    );
  });
}

describe('dashboard query layer', () => {
  it('keeps resource query keys stable and scoped', () => {
    expect(dashboardQueryKeys.protection.overview()).toEqual([
      'dashboard',
      'protection',
      'overview',
    ]);
    expect(dashboardQueryKeys.plans.detail('DASH')).toEqual([
      'dashboard',
      'plans',
      'detail',
      'DASH',
    ]);
    expect(dashboardQueryKeys.plans.detail('DASH')).toEqual(
      dashboardQueryKeys.plans.detail('DASH')
    );
  });

  it('renders loading then successful generated-client data', async () => {
    let resolve!: (value: Awaited<ReturnType<DashboardApi['getProtectionOverview']>>) => void;
    const api: DashboardApi = {
      getProtectionOverview: () => new Promise((next) => (resolve = next)),
      getPatternCatalogue: async () => ({
        schema_version: 'anvil.dashboard.patterns.v1' as const,
        data_state: 'unavailable' as const,
        source_message: 'fixture',
        patterns: [],
      }),
      listPlans: async () => [],
      getPlan: async () => {
        throw new Error('unused');
      },
    };
    await renderWith(api);
    expect(container?.textContent).toContain('Loading protection');

    await act(async () => {
      resolve({ source_message: 'Generated contract data' } as never);
      await new Promise((next) => setTimeout(next, 20));
    });
    expect(container?.textContent).toContain('Generated contract data');
  });

  it('renders structured API errors without discarding their code', async () => {
    const api: DashboardApi = {
      getProtectionOverview: async () => {
        throw Object.assign(new Error('Workspace unavailable'), { code: 'workspace-unavailable' });
      },
      getPatternCatalogue: async () => ({
        schema_version: 'anvil.dashboard.patterns.v1' as const,
        data_state: 'unavailable' as const,
        source_message: 'fixture',
        patterns: [],
      }),
      listPlans: async () => [],
      getPlan: async () => {
        throw new Error('unused');
      },
    };
    await renderWith(api);
    await act(async () => {
      await new Promise((next) => setTimeout(next, 20));
    });
    expect(container?.textContent).toContain('workspace-unavailable');
    expect(container?.textContent).toContain('Workspace unavailable');
    expect(container?.textContent).toContain('Start or restart the local dashboard server');
    expect(container?.querySelector('button')?.textContent).toContain('Retry');
  });

  it('lets the user retry a failed query', async () => {
    const getProtectionOverview = vi
      .fn<DashboardApi['getProtectionOverview']>()
      .mockRejectedValueOnce(new TypeError('network offline'))
      .mockResolvedValueOnce({ source_message: 'Recovered API data' } as never);
    const api: DashboardApi = {
      getProtectionOverview,
      getPatternCatalogue: async () => ({
        schema_version: 'anvil.dashboard.patterns.v1' as const,
        data_state: 'unavailable' as const,
        source_message: 'fixture',
        patterns: [],
      }),
      listPlans: async () => [],
      getPlan: async () => {
        throw new Error('unused');
      },
    };
    await renderWith(api);
    await act(async () => {
      await new Promise((next) => setTimeout(next, 20));
    });

    await act(async () => {
      container?.querySelector<HTMLButtonElement>('button')?.click();
      await new Promise((next) => setTimeout(next, 20));
    });

    expect(getProtectionOverview).toHaveBeenCalledTimes(2);
    expect(container?.textContent).toContain('Recovered API data');
  });
});
