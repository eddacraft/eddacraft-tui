import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, describe, expect, it } from 'vitest';

import { DashboardApp } from './main';

let root: Root | null = null;
let container: HTMLDivElement | null = null;

afterEach(() => {
  if (root) {
    act(() => {
      root?.unmount();
    });
  }

  container?.remove();
  root = null;
  container = null;
});

describe('dashboard app host', () => {
  it('renders the dedicated dashboard root without using the website app', async () => {
    container = document.createElement('div');
    document.body.append(container);
    root = createRoot(container);

    await act(async () => {
      root?.render(<DashboardApp />);
    });

    const dashboardRoot = container.querySelector('[data-dashboard-root]');

    expect(dashboardRoot).not.toBeNull();
    expect(dashboardRoot?.textContent).toContain('Anvil Dashboard');
    expect(dashboardRoot?.textContent).toContain('Protection overview');
  });
});
