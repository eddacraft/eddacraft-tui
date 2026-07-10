import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, describe, expect, it } from 'vitest';

import { DashboardApp } from '@/main';

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

  it('keeps navigation outside the main landmark and exposes a skip link', async () => {
    container = document.createElement('div');
    document.body.append(container);
    root = createRoot(container);

    await act(async () => {
      root?.render(<DashboardApp />);
    });

    const main = container.querySelector('main#main-content');
    const sidebar = container.querySelector('aside[aria-label="Dashboard modules"]');
    const skipLink = container.querySelector('a[href="#main-content"]');

    expect(main).not.toBeNull();
    expect(sidebar).not.toBeNull();
    expect(main?.contains(sidebar)).toBe(false);
    expect(skipLink?.textContent).toBe('Skip to dashboard content');
  });

  it('renders a truthful empty state until protection data is connected', async () => {
    container = document.createElement('div');
    document.body.append(container);
    root = createRoot(container);

    await act(async () => {
      root?.render(<DashboardApp />);
    });

    expect(container.textContent).toContain('No protection data connected');
    expect(container.textContent).not.toContain('Save-time status');
    expect(container.textContent).not.toContain('Pending API');
    expect(container.textContent).not.toContain('Read-only Wave 1');
  });
});
