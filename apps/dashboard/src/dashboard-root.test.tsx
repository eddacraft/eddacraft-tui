import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, describe, expect, it } from 'vitest';

import { protectionOverviewFixture } from '@/api/fixtures';
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
  const historyWarningIndex = protectionOverviewFixture.warnings.findIndex(
    (warning) => warning.id === 'warning-history'
  );
  if (historyWarningIndex >= 0) protectionOverviewFixture.warnings.splice(historyWarningIndex, 1);
  window.history.replaceState(null, '', '/');
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

  it('renders deterministic local protection data without scaffold placeholders', async () => {
    container = document.createElement('div');
    document.body.append(container);
    root = createRoot(container);

    await act(async () => {
      root?.render(<DashboardApp />);
    });

    expect(container.textContent).toContain('2026-07-13 08:30:00');
    expect(container.textContent).toContain('typed-secret-rule');
    expect(container.textContent).not.toContain('No protection data connected');
    expect(container.textContent).not.toContain('Pending API');
    expect(container.textContent).not.toContain('Read-only Wave 1');
  });

  it('renders the protection workspace as labelled operational regions', async () => {
    container = document.createElement('div');
    document.body.append(container);
    root = createRoot(container);

    await act(async () => {
      root?.render(<DashboardApp />);
    });

    expect(container.textContent).toContain('Save-time protection active');
    expect(container.textContent).toContain('Next attention');
    expect(container.textContent).toContain('Evidence inspector');
    expect(container.textContent).toContain('Affected files (1)');
    expect(container.querySelectorAll('table')).toHaveLength(3);
    expect(container.querySelectorAll('th').length).toBeGreaterThan(0);
    expect(container.querySelector('[aria-labelledby="protection-summary-title"]')).not.toBeNull();
  });

  it('provides sibling desktop and mobile navigation surfaces', async () => {
    container = document.createElement('div');
    document.body.append(container);
    root = createRoot(container);

    await act(async () => {
      root?.render(<DashboardApp />);
    });

    expect(container.querySelector('[data-desktop-sidebar]')).not.toBeNull();
    expect(container.querySelector('[data-mobile-header]')).not.toBeNull();
    expect(container.querySelector('[data-mobile-bottom-nav]')).not.toBeNull();
    expect(container.querySelector('button[aria-label="Search dashboard"]')).not.toBeNull();
    expect(container.querySelector('button[aria-label="Open navigation"]')).not.toBeNull();
    expect(container.querySelector('[role="tablist"]')?.textContent).toContain('Runs');
    expect(container.querySelector('[role="tablist"]')?.textContent).toContain('Warnings (1)');
  });

  it('opens Cmd+K over registered module resources', async () => {
    container = document.createElement('div');
    document.body.append(container);
    root = createRoot(container);
    await act(async () => {
      root?.render(<DashboardApp />);
    });

    await act(async () => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', metaKey: true }));
    });

    expect(document.querySelector('[role="dialog"]')).not.toBeNull();
    expect(document.body.textContent).toContain('Active warnings');
    expect(document.body.textContent).toContain('Plan Driver');

    const planCommand = [...document.querySelectorAll<HTMLElement>('[cmdk-item]')].find(
      (item) => item.textContent?.trim() === 'Plan Driver'
    );
    await act(async () => {
      planCommand?.click();
      await new Promise((next) => setTimeout(next, 20));
    });
    expect(container.textContent).toContain('Plan Driver');
  });

  it('restores protection filters from the URL and writes filter navigation back', async () => {
    window.history.replaceState(null, '', '/?view=warnings&severity=high');
    container = document.createElement('div');
    document.body.append(container);
    root = createRoot(container);

    await act(async () => {
      root?.render(<DashboardApp />);
    });

    expect(container.querySelector('[role="tab"][data-state="active"]')?.textContent).toContain(
      'Warnings'
    );
    const severityFilter = container.querySelector<HTMLSelectElement>(
      'select[aria-label="Filter warnings by severity"]'
    );
    expect(severityFilter?.value).toBe('high');

    await act(async () => {
      if (!severityFilter) return;
      severityFilter.value = 'medium';
      severityFilter.dispatchEvent(new Event('change', { bubbles: true }));
      await new Promise((next) => setTimeout(next, 0));
    });

    expect(new URLSearchParams(window.location.search).get('severity')).toBe('medium');
  });

  it('restores evidence selection when browser history changes after mount', async () => {
    protectionOverviewFixture.warnings.push({
      ...protectionOverviewFixture.warnings[0],
      id: 'warning-history',
      evidence_id: 'evidence-history',
      rule: 'history-restored-rule',
      file_path: 'src/history.ts',
      line: 24,
    });
    window.history.replaceState(
      null,
      '',
      '/?view=warnings&severity=all&evidence=evidence-typed-secret'
    );
    container = document.createElement('div');
    document.body.append(container);
    root = createRoot(container);

    await act(async () => {
      root?.render(<DashboardApp />);
    });

    const inspector = container.querySelector('#evidence-inspector');
    expect(inspector?.textContent).toContain('typed-secret-rule');

    await act(async () => {
      [...container!.querySelectorAll<HTMLButtonElement>('.table-rule')]
        .find((button) => button.textContent?.trim() === 'history-restored-rule')
        ?.click();
      await new Promise((next) => setTimeout(next, 0));
    });

    expect(new URLSearchParams(window.location.search).get('evidence')).toBe('warning-history');
    expect(inspector?.textContent).toContain('history-restored-rule');

    await act(async () => {
      window.history.back();
      await new Promise((next) => setTimeout(next, 20));
    });

    expect(new URLSearchParams(window.location.search).get('evidence')).toBe(
      'evidence-typed-secret'
    );
    expect(inspector?.textContent).toContain('typed-secret-rule');

    await act(async () => {
      window.history.forward();
      await new Promise((next) => setTimeout(next, 20));
    });

    expect(new URLSearchParams(window.location.search).get('evidence')).toBe('warning-history');
    expect(inspector?.textContent).toContain('history-restored-rule');
  });
});
