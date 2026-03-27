import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { render } from 'ink-testing-library';
import { stripAnsi } from '../../../../tui/__tests__/test-utils.js';

import { DriftTutorial, DRIFT_STEPS, DRIFT_STEP_DEFINITIONS } from '../features/DriftTutorial.js';
import { IntroStep } from '../features/drift-steps/IntroStep.js';
import { CaptureStep } from '../features/drift-steps/CaptureStep.js';
import { InspectStep } from '../features/drift-steps/InspectStep.js';
import { CompareStep } from '../features/drift-steps/CompareStep.js';
import { SummaryStep } from '../features/drift-steps/SummaryStep.js';

describe('DriftTutorial types', () => {
  describe('DRIFT_STEPS', () => {
    it('has 5 steps', () => {
      expect(DRIFT_STEPS).toHaveLength(5);
    });

    it('has correct step order', () => {
      expect(DRIFT_STEPS).toEqual(['intro', 'capture', 'inspect', 'compare', 'summary']);
    });
  });

  describe('DRIFT_STEP_DEFINITIONS', () => {
    it('has a definition for each step', () => {
      for (const step of DRIFT_STEPS) {
        expect(DRIFT_STEP_DEFINITIONS[step]).toBeDefined();
        expect(DRIFT_STEP_DEFINITIONS[step].title).toBeDefined();
        expect(DRIFT_STEP_DEFINITIONS[step].description).toBeDefined();
      }
    });
  });
});

describe('DriftTutorial component', () => {
  it('renders and shows "Drift Detection" title', () => {
    const { lastFrame } = render(<DriftTutorial />);
    expect(lastFrame()).toContain('Drift Detection');
  });

  it('shows the header with "ANVIL DRIFT TUTORIAL"', () => {
    const { lastFrame } = render(<DriftTutorial />);
    expect(lastFrame()).toContain('ANVIL DRIFT TUTORIAL');
  });

  it('shows progress indicator with step count', () => {
    const { lastFrame } = render(<DriftTutorial />);
    expect(lastFrame()).toContain('Step 1 of 5');
  });

  it('shows intro step description', () => {
    const { lastFrame } = render(<DriftTutorial />);
    expect(lastFrame()).toContain('What drift detection is and why it matters');
  });

  it('shows keyboard shortcuts', () => {
    const { lastFrame } = render(<DriftTutorial />);
    expect(stripAnsi(lastFrame()!)).toContain('q quit');
  });

  it('advances to next step on Enter', async () => {
    const { lastFrame, stdin } = render(<DriftTutorial />);

    expect(lastFrame()).toContain('Step 1 of 5');

    stdin.write('\r');

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Step 2 of 5');
    });
  });

  it('shows Capture Baseline on step 2', async () => {
    const { lastFrame, stdin } = render(<DriftTutorial />);

    stdin.write('\r');

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Capture Baseline');
    });
  });

  it('can navigate back with left arrow', async () => {
    const { lastFrame, stdin } = render(<DriftTutorial />);

    stdin.write('\r');

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Step 2 of 5');
    });

    stdin.write('\u001B[D');

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Step 1 of 5');
    });
  });

  it('can advance through multiple steps', async () => {
    const { lastFrame, stdin } = render(<DriftTutorial />);

    expect(lastFrame()).toContain('Step 1 of 5');

    stdin.write('\r');
    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Step 2 of 5');
    });

    stdin.write('\r');
    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Step 3 of 5');
    });
  });
});

describe('IntroStep', () => {
  it('shows the drift detection title', () => {
    const { lastFrame } = render(<IntroStep />);
    expect(lastFrame()).toContain('Drift Detection');
  });

  it('explains drift detection', () => {
    const { lastFrame } = render(<IntroStep />);
    expect(lastFrame()).toContain('tracks how your architecture changes over time');
  });

  it('shows key commands', () => {
    const { lastFrame } = render(<IntroStep />);
    const frame = lastFrame();
    expect(frame).toContain('anvil drift snapshot');
    expect(frame).toContain('anvil drift compare');
    expect(frame).toContain('anvil drift report');
  });

  it('shows what the tutorial covers', () => {
    const { lastFrame } = render(<IntroStep />);
    const frame = lastFrame();
    expect(frame).toContain('Capture a baseline snapshot');
    expect(frame).toContain('Inspect what a snapshot contains');
    expect(frame).toContain('Compare snapshots to detect drift');
    expect(frame).toContain('Understand drift trends');
  });

  it('shows the continue prompt', () => {
    const { lastFrame } = render(<IntroStep />);
    expect(lastFrame()).toContain('Press Enter to continue');
  });
});

describe('CaptureStep', () => {
  it('shows the step title', () => {
    const { lastFrame } = render(<CaptureStep />);
    expect(lastFrame()).toContain('Capture Baseline Snapshot');
  });

  it('shows the snapshot command', () => {
    const { lastFrame } = render(<CaptureStep />);
    expect(lastFrame()).toContain('anvil drift snapshot --name baseline');
  });

  it('shows snapshot example output', () => {
    const { lastFrame } = render(<CaptureStep />);
    const frame = lastFrame();
    expect(frame).toContain('Snapshot captured: baseline');
    expect(frame).toContain('Modules: 42');
    expect(frame).toContain('Import edges: 187');
  });

  it('shows what gets captured', () => {
    const { lastFrame } = render(<CaptureStep />);
    const frame = lastFrame();
    expect(frame).toContain('Module count');
    expect(frame).toContain('Import edges');
    expect(frame).toContain('Dependency graph');
  });
});

describe('InspectStep', () => {
  it('shows the step title', () => {
    const { lastFrame } = render(<InspectStep />);
    expect(lastFrame()).toContain('Inspect Snapshot');
  });

  it('shows module counts by layer', () => {
    const { lastFrame } = render(<InspectStep />);
    const frame = lastFrame();
    expect(frame).toContain('8 modules');
    expect(frame).toContain('12 modules');
    expect(frame).toContain('6 modules');
    expect(frame).toContain('16 modules');
  });

  it('shows cross-boundary edges', () => {
    const { lastFrame } = render(<InspectStep />);
    const frame = lastFrame();
    expect(frame).toContain('Cross-boundary edges: 3');
    expect(frame).toContain('src/api/users.ts');
    expect(frame).toContain('src/repositories/user.repo.ts');
    expect(frame).toContain('src/services/billing.ts');
  });

  it('explains what Anvil watches for', () => {
    const { lastFrame } = render(<InspectStep />);
    expect(lastFrame()).toContain('cross-boundary edges are what Anvil watches for');
  });
});

describe('CompareStep', () => {
  it('shows the step title', () => {
    const { lastFrame } = render(<CompareStep />);
    expect(lastFrame()).toContain('Compare Snapshots');
  });

  it('shows the compare command', () => {
    const { lastFrame } = render(<CompareStep />);
    expect(lastFrame()).toContain('anvil drift compare baseline current');
  });

  it('shows new edges', () => {
    const { lastFrame } = render(<CompareStep />);
    const frame = lastFrame();
    expect(frame).toContain('New edges: 2');
    expect(frame).toContain('src/api/orders.ts');
    expect(frame).toContain('src/repositories/order.repo.ts');
    expect(frame).toContain('src/utils/logger.ts');
  });

  it('shows removed edges', () => {
    const { lastFrame } = render(<CompareStep />);
    const frame = lastFrame();
    expect(frame).toContain('Removed edges: 1');
    expect(frame).toContain('src/api/auth.ts');
    expect(frame).toContain('fixed!');
  });

  it('shows net drift', () => {
    const { lastFrame } = render(<CompareStep />);
    expect(lastFrame()).toContain('Net drift: +1 cross-boundary edge');
  });

  it('explains positive vs negative drift', () => {
    const { lastFrame } = render(<CompareStep />);
    const frame = lastFrame();
    expect(frame).toContain('Positive drift means architecture is degrading');
    expect(frame).toContain('Negative drift means');
    expect(frame).toContain('improving');
  });
});

describe('SummaryStep', () => {
  it('shows the configured message', () => {
    const { lastFrame } = render(<SummaryStep />);
    expect(lastFrame()).toContain('Drift tracking configured!');
  });

  it('shows command reference', () => {
    const { lastFrame } = render(<SummaryStep />);
    const frame = lastFrame();
    expect(frame).toContain('anvil drift snapshot --name <name>');
    expect(frame).toContain('anvil drift compare <from> <to>');
    expect(frame).toContain('anvil drift report');
    expect(frame).toContain('anvil drift list');
  });

  it('shows the sprint tip', () => {
    const { lastFrame } = render(<SummaryStep />);
    expect(lastFrame()).toContain('snapshot at the start of each sprint');
  });

  it('shows tutorial complete message', () => {
    const { lastFrame } = render(<SummaryStep />);
    expect(lastFrame()).toContain('Tutorial complete!');
  });

  it('does not show inline cleanup instructions (handled by bottom bar)', () => {
    const { lastFrame } = render(<SummaryStep />);
    const frame = lastFrame();
    expect(frame).not.toContain('clean up tutorial files');
  });
});
