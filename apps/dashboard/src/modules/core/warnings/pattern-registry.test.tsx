import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, describe, expect, it } from 'vitest';

import { PatternRegistry } from '@/modules/core/warnings/pattern-registry';

let root: Root | null = null;
let container: HTMLDivElement | null = null;

afterEach(() => {
  act(() => root?.unmount());
  container?.remove();
  root = null;
  container = null;
});

describe('PatternRegistry', () => {
  it('exposes the expanded state and controlled documentation panel', async () => {
    container = document.createElement('div');
    document.body.append(container);
    root = createRoot(container);
    await act(async () => {
      root?.render(
        <PatternRegistry
          catalogue={{
            schema_version: 'anvil.dashboard.patterns.v1',
            data_state: 'complete',
            source_message: 'fixture',
            patterns: [
              {
                id: 'PAT-001',
                title: 'Nested branch',
                family: 'maintainability',
                severity: 'medium',
                enabled: true,
                instance_count: 1,
                description: 'Avoid deeply nested branches.',
              },
            ],
          }}
        />
      );
    });

    const toggle = container.querySelector<HTMLButtonElement>('button[aria-controls]');
    expect(toggle?.getAttribute('aria-expanded')).toBe('false');
    expect(toggle?.getAttribute('aria-controls')).toBe('pattern-docs-PAT-001');

    act(() => {
      toggle?.dispatchEvent(new MouseEvent('click', { bubbles: true, button: 0 }));
    });

    const expandedToggle = container.querySelector<HTMLButtonElement>('button[aria-controls]');
    expect(expandedToggle?.getAttribute('aria-expanded')).toBe('true');
    expect(container.querySelector('#pattern-docs-PAT-001')?.textContent).toContain(
      'Avoid deeply nested branches.'
    );
  });
});
