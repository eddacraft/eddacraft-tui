import { act, type ComponentProps } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { protectionOverviewFixture } from '@/api/fixtures';
import { ProtectionOverviewContent } from '@/modules/protection/protection-overview';

let root: Root | null = null;
let container: HTMLDivElement | null = null;

afterEach(() => {
  act(() => root?.unmount());
  container?.remove();
  root = null;
  container = null;
});

type ProtectionOverviewContentProps = ComponentProps<typeof ProtectionOverviewContent>;

async function render(
  overrides: Partial<ProtectionOverviewContentProps['overview']> = {},
  props: Omit<Partial<ProtectionOverviewContentProps>, 'overview'> = {}
) {
  container = document.createElement('div');
  document.body.append(container);
  root = createRoot(container);
  await act(async () => {
    root?.render(
      <ProtectionOverviewContent
        overview={{ ...protectionOverviewFixture, ...overrides }}
        {...props}
      />
    );
  });
}

describe('Protection Overview typed resource', () => {
  it('renders runs, warnings, affected files, freshness and selected evidence from API data', async () => {
    await render();
    expect(container?.textContent).toContain('2026-07-13 08:30:00');
    expect(container?.textContent).toContain('typed-secret-rule');
    expect(container?.textContent).toContain('src/typed.ts:18');
    expect(container?.textContent).toContain('Evidence inspector');
    expect(container?.textContent).toContain('Data state: Partial');
    expect(container?.textContent).toContain('Offline · last-known evidence');
  });

  it('names the full-data state', async () => {
    await render({ data_state: 'complete', gaps: [] });
    expect(container?.textContent).toContain('Full data');
    expect(container?.textContent).toContain('Data state: Full');
  });

  it('distinguishes empty data without implying a protection failure', async () => {
    await render({ data_state: 'unavailable', recent_runs: [], warnings: [], affected_files: [] });
    expect(container?.textContent).toContain('No local protection evidence yet');
    expect(container?.textContent).not.toContain('Save-time protection failed');
  });

  it('explains an empty recent-runs resource instead of rendering a blank table', async () => {
    await render({ recent_runs: [] });
    expect(container?.textContent).toContain('No recent runs');
  });

  it('renders unavailable warning and affected-file resources without claiming zero results', async () => {
    await render({
      data_state: 'partial',
      warnings_state: 'unavailable',
      warnings: [],
      affected_files_state: 'unavailable',
      affected_files: [],
      gaps: [
        { component: 'retained-warning-history', reason: 'Warning history is unavailable.' },
        { component: 'affected-files', reason: 'Affected files are unavailable.' },
      ],
    });

    expect(container?.textContent).toContain('Warnings unavailable');
    expect(container?.textContent).toContain('Warning history is unavailable.');
    expect(container?.textContent).toContain('Affected files unavailable');
    expect(container?.textContent).toContain('Affected files are unavailable.');
    expect(container?.textContent).not.toContain('Warnings (0)');
    expect(container?.textContent).not.toContain('Affected files (0)');
  });

  it('labels partial warning and affected-file resources without claiming complete counts', async () => {
    await render({
      warnings_state: 'partial',
      affected_files_state: 'partial',
      warnings: [],
      affected_files: [],
      gaps: [
        { component: 'retained-warning-history', reason: 'Only the latest gate is available.' },
        { component: 'affected-files', reason: 'The affected-file index is incomplete.' },
      ],
    });

    expect(container?.textContent).toContain('Warnings partial');
    expect(container?.textContent).toContain('Affected files partial');
    expect(container?.textContent).not.toContain('Warnings (0)');
    expect(container?.textContent).not.toContain('Affected files (0)');
  });

  it('labels non-empty partial resources as shown subsets rather than total counts', async () => {
    await render({
      warnings_state: 'partial',
      affected_files_state: 'partial',
      gaps: [
        { component: 'retained-warning-history', reason: 'Warning history is incomplete.' },
        { component: 'affected-files', reason: 'The affected-file index is incomplete.' },
      ],
    });

    expect(container?.textContent).toContain('Warnings partial (1 shown)');
    expect(container?.textContent).toContain('Affected files partial (1 shown)');
  });

  it('claims zero warning and affected-file results only when each resource is complete', async () => {
    await render({
      warnings_state: 'complete',
      warnings: [],
      affected_files_state: 'complete',
      affected_files: [],
      gaps: [],
    });

    expect(container?.textContent).toContain('Warnings (0)');
    expect(container?.textContent).toContain('Affected files (0)');
    expect(container?.textContent).toContain('No active warnings');
    expect(container?.textContent).toContain('No affected files');
  });

  it('restores the controlled activity tab and reports tab navigation', async () => {
    const onViewChange = vi.fn();
    await render({}, { view: 'warnings', onViewChange });

    expect(container?.querySelector('[role="tab"][data-state="active"]')?.textContent).toContain(
      'Warnings'
    );

    await act(async () => {
      container
        ?.querySelector<HTMLButtonElement>('[role="tab"][data-state="inactive"]')
        ?.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, button: 0 }));
    });

    expect(onViewChange).toHaveBeenCalledWith('runs');
  });

  it('filters warnings by the controlled severity and reports filter navigation', async () => {
    const onSeverityChange = vi.fn();
    const mediumWarning = {
      ...protectionOverviewFixture.warnings[0],
      id: 'warning-medium',
      evidence_id: 'evidence-medium',
      severity: 'medium',
      rule: 'medium-rule',
      file_path: 'src/medium.ts',
    };

    await render(
      { warnings: [...protectionOverviewFixture.warnings, mediumWarning] },
      { view: 'warnings', severity: 'medium', onSeverityChange }
    );

    const warningsPanel = container?.querySelector('[role="tabpanel"][data-state="active"]');
    expect(warningsPanel?.textContent).toContain('medium-rule');
    expect(warningsPanel?.textContent).not.toContain('typed-secret-rule');

    const severityFilter = container?.querySelector<HTMLSelectElement>(
      'select[aria-label="Filter warnings by severity"]'
    );
    expect(severityFilter?.value).toBe('medium');

    await act(async () => {
      if (!severityFilter) return;
      severityFilter.value = 'low';
      severityFilter.dispatchEvent(new Event('change', { bubbles: true }));
    });

    expect(onSeverityChange).toHaveBeenCalledWith('low');
  });
});
