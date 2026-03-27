import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { render } from 'ink-testing-library';
import type { ScanResults } from '../types.js';
import type { TutorialOption } from '../components/TutorialPicker.js';

import { NextStepsStep } from '../steps/NextStepsStep.js';

const MOCK_TUTORIALS: TutorialOption[] = [
  { topic: 'core', description: 'Core tutorial (scan, watch, fix)' },
  { topic: 'policies', description: 'Write custom OPA/Rego rules' },
  { topic: 'architecture', description: 'Define architecture boundaries' },
  { topic: 'drift', description: 'Track architecture drift over time' },
  { topic: 'ci', description: 'Set up CI integration' },
];

const mockResultsWithWarnings: ScanResults = {
  warningCount: 5,
  fileCount: 3,
  executionTimeMs: 142,
  topWarnings: [
    {
      id: 'AP-001',
      title: 'Broad eslint-disable added',
      file: 'src/utils/helpers.ts',
      line: 12,
      message: 'eslint-disable without specific rule',
      suggestion: 'Specify the exact rule to disable instead of disabling all rules',
    },
  ],
};

const mockResultsClean: ScanResults = {
  warningCount: 0,
  fileCount: 0,
  executionTimeMs: 89,
  topWarnings: [],
};

function makeStartedAt(secondsAgo: number): Date {
  return new Date(Date.now() - secondsAgo * 1000);
}

describe('NextStepsStep', () => {
  it('shows "Tutorial Complete!" message', () => {
    const { lastFrame } = render(
      <NextStepsStep
        startedAt={makeStartedAt(90)}
        scanResults={mockResultsWithWarnings}
        tutorials={MOCK_TUTORIALS}
        onCleanup={vi.fn()}
        onFinish={vi.fn()}
      />
    );

    expect(lastFrame()).toContain('Tutorial Complete!');
  });

  it('shows elapsed time', () => {
    const { lastFrame } = render(
      <NextStepsStep
        startedAt={makeStartedAt(90)}
        scanResults={mockResultsWithWarnings}
        tutorials={MOCK_TUTORIALS}
        onCleanup={vi.fn()}
        onFinish={vi.fn()}
      />
    );

    expect(lastFrame()).toContain('1m 30s');
  });

  it('shows scan summary when warnings were found', () => {
    const { lastFrame } = render(
      <NextStepsStep
        startedAt={makeStartedAt(60)}
        scanResults={mockResultsWithWarnings}
        tutorials={MOCK_TUTORIALS}
        onCleanup={vi.fn()}
        onFinish={vi.fn()}
      />
    );

    const frame = lastFrame();
    expect(frame).toContain('Scanned 3 files');
    expect(frame).toContain('found 5 warnings');
  });

  it('does not show scan summary when project was clean', () => {
    const { lastFrame } = render(
      <NextStepsStep
        startedAt={makeStartedAt(60)}
        scanResults={mockResultsClean}
        tutorials={MOCK_TUTORIALS}
        onCleanup={vi.fn()}
        onFinish={vi.fn()}
      />
    );

    expect(lastFrame()).not.toContain('Scanned 0 files');
  });

  it('does not show scan summary when scanResults is undefined', () => {
    const { lastFrame } = render(
      <NextStepsStep
        startedAt={makeStartedAt(60)}
        tutorials={MOCK_TUTORIALS}
        onCleanup={vi.fn()}
        onFinish={vi.fn()}
      />
    );

    // Should not show the scan stats line (but "Scanned" still appears in "What you learned")
    expect(lastFrame()).not.toContain('found 0 warning');
    expect(lastFrame()).not.toContain('Scanned 0 files');
  });

  it('shows "What you learned" items', () => {
    const { lastFrame } = render(
      <NextStepsStep
        startedAt={makeStartedAt(60)}
        scanResults={mockResultsWithWarnings}
        tutorials={MOCK_TUTORIALS}
        onCleanup={vi.fn()}
        onFinish={vi.fn()}
      />
    );

    const frame = lastFrame();
    expect(frame).toContain('What you learned');
    expect(frame).toContain('Scanned your project for anti-patterns and architecture issues');
    expect(frame).toContain('Used watch mode to get real-time feedback');
    expect(frame).toContain('Fixed an issue at save-time (the core loop)');
  });

  it('shows tutorial picker with numbered options', () => {
    const { lastFrame } = render(
      <NextStepsStep
        startedAt={makeStartedAt(60)}
        scanResults={mockResultsWithWarnings}
        tutorials={MOCK_TUTORIALS}
        onCleanup={vi.fn()}
        onFinish={vi.fn()}
      />
    );

    const frame = lastFrame();
    expect(frame).toContain("What's next");
    expect(frame).toContain('policies');
    expect(frame).toContain('Write custom OPA/Rego rules');
    expect(frame).toContain('architecture');
    expect(frame).toContain('Define architecture boundaries');
    expect(frame).toContain('drift');
    expect(frame).toContain('Track architecture drift over time');
    expect(frame).toContain('ci');
    expect(frame).toContain('Set up CI integration');
  });

  it('shows cleanup confirmation when cleanupConfirming is true', () => {
    const { lastFrame } = render(
      <NextStepsStep
        startedAt={makeStartedAt(60)}
        scanResults={mockResultsWithWarnings}
        tutorials={MOCK_TUTORIALS}
        cleanupConfirming={true}
        onCleanup={vi.fn()}
        onFinish={vi.fn()}
      />
    );

    const frame = lastFrame();
    expect(frame).toContain('Remove these tutorial files?');
    expect(frame).toContain('.anvil/tutorial/');
  });

  it('shows elapsed time in seconds when under a minute', () => {
    const { lastFrame } = render(
      <NextStepsStep
        startedAt={makeStartedAt(45)}
        scanResults={mockResultsWithWarnings}
        tutorials={MOCK_TUTORIALS}
        onCleanup={vi.fn()}
        onFinish={vi.fn()}
      />
    );

    expect(lastFrame()).toContain('45s');
  });
});
