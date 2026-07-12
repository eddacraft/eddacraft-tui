import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, describe, expect, it } from 'vitest';

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

async function render(overrides = {}) {
  container = document.createElement('div');
  document.body.append(container);
  root = createRoot(container);
  await act(async () => {
    root?.render(
      <ProtectionOverviewContent overview={{ ...protectionOverviewFixture, ...overrides }} />
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
});
