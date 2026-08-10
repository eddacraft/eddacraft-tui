import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, describe, expect, it } from 'vitest';

import type { components } from '@/api/generated/openapi';
import { filterWarnings, WarningTable } from '@/modules/core/warnings/warning-table';

type Warning = components['schemas']['WarningSummary'];

let root: Root | null = null;
let container: HTMLDivElement | null = null;

const warnings: Warning[] = [
  {
    id: 'w-high',
    severity: 'high',
    category: 'Secrets',
    message: 'Potential credential in configuration',
    file_path: 'src/config.ts',
    age_label: 'Latest gate',
    evidence_id: 'w-high',
    rule: 'secret-detection',
    line: 7,
    explanation: 'secret',
    matched_pattern: 'SECRET-001',
    evidence_excerpt: [],
  },
  {
    id: 'w-low',
    severity: 'low',
    category: 'Maintainability',
    message: 'Nested branch can be simplified',
    file_path: 'src/logic.ts',
    age_label: 'Latest gate',
    evidence_id: 'w-low',
    rule: 'antipattern-scan',
    line: 18,
    explanation: 'branch',
    matched_pattern: 'STYLE-002',
    evidence_excerpt: [],
  },
];

afterEach(() => {
  act(() => root?.unmount());
  container?.remove();
  root = null;
  container = null;
});

async function render(rows: readonly Warning[] = warnings) {
  container = document.createElement('div');
  document.body.append(container);
  root = createRoot(container);
  await act(async () => root?.render(<WarningTable warnings={rows} />));
}

function change(selector: string, value: string) {
  const control = container?.querySelector<HTMLInputElement | HTMLSelectElement>(selector);
  expect(control).toBeTruthy();
  act(() => {
    if (!control) return;
    const valueSetter = Object.getOwnPropertyDescriptor(
      Object.getPrototypeOf(control),
      'value'
    )?.set;
    valueSetter?.call(control, value);
    const event = control instanceof HTMLInputElement ? 'input' : 'change';
    control.dispatchEvent(new Event(event, { bubbles: true }));
  });
}

describe('WarningTable', () => {
  it('filters the rendered rows by severity and category', async () => {
    await render();

    change('[aria-label="Filter warnings by severity"]', 'high');
    expect(container?.textContent).toContain('secret-detection');
    expect(container?.textContent).not.toContain('antipattern-scan');

    change('[aria-label="Filter warnings by severity"]', 'all');
    change('[aria-label="Filter warnings by category"]', 'Maintainability');
    expect(container?.textContent).toContain('antipattern-scan');
    expect(container?.textContent).not.toContain('secret-detection');
  });

  it('offers critical severity and isolates critical warning rows', async () => {
    const criticalWarning: Warning = {
      ...warnings[0],
      id: 'w-critical',
      severity: 'critical',
      category: 'Security',
      message: 'Confirmed credential exposure',
      file_path: 'src/credentials.ts',
      evidence_id: 'w-critical',
      rule: 'credential-exposure',
    };
    await render([...warnings, criticalWarning]);

    const severityFilter = container?.querySelector<HTMLSelectElement>(
      '[aria-label="Filter warnings by severity"]'
    );
    expect([...(severityFilter?.options ?? [])].map((option) => option.value)).toContain(
      'critical'
    );

    change('[aria-label="Filter warnings by severity"]', 'critical');
    expect(container?.textContent).toContain('credential-exposure');
    expect(container?.textContent).not.toContain('secret-detection');
    expect(container?.textContent).not.toContain('antipattern-scan');
  });

  it('matches free text against both file paths and warning messages', () => {
    expect(
      filterWarnings(warnings, { severity: 'all', category: 'all', query: 'config.ts' }).map(
        (warning) => warning.id
      )
    ).toEqual(['w-high']);
    expect(
      filterWarnings(warnings, { severity: 'all', category: 'all', query: 'credential' }).map(
        (warning) => warning.id
      )
    ).toEqual(['w-high']);
  });

  it('groups the filtered warnings by the selected dimension', async () => {
    await render();
    change('[aria-label="Group warnings"]', 'severity');

    expect(container?.textContent).toContain('high (1)');
    expect(container?.textContent).toContain('low (1)');
    expect(container?.querySelectorAll('table')).toHaveLength(2);
  });
});
