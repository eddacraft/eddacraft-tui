import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { render } from 'ink-testing-library';
import type { ScanResults } from '../types.js';

import { NextStepsStep } from '../steps/NextStepsStep.js';

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
        onCleanup={vi.fn()}
        onFinish={vi.fn()}
      />
    );

    expect(lastFrame()).not.toContain('Scanned 0 files');
  });

  it('does not show scan summary when scanResults is undefined', () => {
    const { lastFrame } = render(
      <NextStepsStep startedAt={makeStartedAt(60)} onCleanup={vi.fn()} onFinish={vi.fn()} />
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

  it('shows feature tutorial commands', () => {
    const { lastFrame } = render(
      <NextStepsStep
        startedAt={makeStartedAt(60)}
        scanResults={mockResultsWithWarnings}
        onCleanup={vi.fn()}
        onFinish={vi.fn()}
      />
    );

    const frame = lastFrame();
    expect(frame).toContain('Explore further');
    expect(frame).toContain('anvil tutorial policies');
    expect(frame).toContain('Write custom OPA/Rego rules');
    expect(frame).toContain('anvil tutorial architecture');
    expect(frame).toContain('Define architecture boundaries');
    expect(frame).toContain('anvil tutorial drift');
    expect(frame).toContain('Track architecture drift over time');
    expect(frame).toContain('anvil tutorial ci');
    expect(frame).toContain('Set up CI integration');
  });

  it('shows resources section', () => {
    const { lastFrame } = render(
      <NextStepsStep
        startedAt={makeStartedAt(60)}
        scanResults={mockResultsWithWarnings}
        onCleanup={vi.fn()}
        onFinish={vi.fn()}
      />
    );

    const frame = lastFrame();
    expect(frame).toContain('Resources');
    expect(frame).toContain('https://anvil.eddacraft.com/docs');
    expect(frame).toContain('anvil --help');
  });

  it('shows cleanup instructions (c and q keys)', () => {
    const { lastFrame } = render(
      <NextStepsStep
        startedAt={makeStartedAt(60)}
        scanResults={mockResultsWithWarnings}
        onCleanup={vi.fn()}
        onFinish={vi.fn()}
      />
    );

    const frame = lastFrame();
    expect(frame).toContain('c');
    expect(frame).toContain('clean up tutorial files');
    expect(frame).toContain('q');
    expect(frame).toContain('exit');
  });

  it('shows elapsed time in seconds when under a minute', () => {
    const { lastFrame } = render(
      <NextStepsStep
        startedAt={makeStartedAt(45)}
        scanResults={mockResultsWithWarnings}
        onCleanup={vi.fn()}
        onFinish={vi.fn()}
      />
    );

    expect(lastFrame()).toContain('45s');
  });
});
