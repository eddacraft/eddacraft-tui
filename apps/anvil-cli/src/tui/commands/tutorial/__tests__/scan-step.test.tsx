import { describe, it, expect, vi, beforeEach } from 'vitest';
import React from 'react';
import { render } from 'ink-testing-library';
import type { ScanResults } from '../types.js';

// Mock the scan-project module (separate file from the component)
vi.mock('../steps/scan-project.js', () => ({
  scanProject: vi.fn(),
}));

// Mock getWorkspaceRoot
vi.mock('../../../../utils/file-io.js', () => ({
  getWorkspaceRoot: () => '/mock/workspace',
}));

// Import after mocks are set up
import { ScanStep } from '../steps/ScanStep.js';
import { scanProject } from '../steps/scan-project.js';

const mockScanProject = vi.mocked(scanProject);

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
    {
      id: 'AP-003',
      title: 'Console statement in production code',
      file: 'src/api/handler.ts',
      line: 45,
      message: 'console.log left in production code',
      suggestion: 'Use a structured logger instead of console.log',
    },
    {
      id: 'ARCH-001',
      title: 'Cross-boundary import detected',
      file: 'src/core/service.ts',
      line: 3,
      message: 'Direct import from adapter layer',
      suggestion: 'Import through the public API of the target module',
    },
  ],
};

const mockResultsClean: ScanResults = {
  warningCount: 0,
  fileCount: 0,
  executionTimeMs: 89,
  topWarnings: [],
};

describe('ScanStep', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('shows scanning indicator initially', () => {
    mockScanProject.mockReturnValue(new Promise(() => {})); // never resolves

    const onComplete = vi.fn();
    const { lastFrame } = render(<ScanStep onComplete={onComplete} />);

    expect(lastFrame()).toContain('Scanning your project...');
  });

  it('shows results summary after scan completes', async () => {
    mockScanProject.mockResolvedValue(mockResultsWithWarnings);

    const onComplete = vi.fn();
    const { lastFrame } = render(<ScanStep onComplete={onComplete} />);

    await vi.waitFor(() => {
      expect(onComplete).toHaveBeenCalledWith(mockResultsWithWarnings);
    });

    const frame = lastFrame();
    expect(frame).toContain('Found 5 warnings across 3 files');
    expect(frame).toContain('142ms');
  });

  it('shows top warnings with file, line, and title', async () => {
    mockScanProject.mockResolvedValue(mockResultsWithWarnings);

    const onComplete = vi.fn();
    const { lastFrame } = render(<ScanStep onComplete={onComplete} />);

    await vi.waitFor(() => {
      expect(onComplete).toHaveBeenCalled();
    });

    const frame = lastFrame();
    expect(frame).toContain('Broad eslint-disable added');
    expect(frame).toContain('src/utils/helpers.ts:12');
    expect(frame).toContain('Console statement in production code');
    expect(frame).toContain('src/api/handler.ts:45');
    expect(frame).toContain('Cross-boundary import detected');
    expect(frame).toContain('src/core/service.ts:3');
  });

  it('shows suggestions for top warnings', async () => {
    mockScanProject.mockResolvedValue(mockResultsWithWarnings);

    const onComplete = vi.fn();
    const { lastFrame } = render(<ScanStep onComplete={onComplete} />);

    await vi.waitFor(() => {
      expect(onComplete).toHaveBeenCalled();
    });

    const frame = lastFrame();
    expect(frame).toContain('Specify the exact rule to disable');
    expect(frame).toContain('Use a structured logger');
    expect(frame).toContain('Import through the public API');
  });

  it('shows clean message when no warnings found', async () => {
    mockScanProject.mockResolvedValue(mockResultsClean);

    const onComplete = vi.fn();
    const { lastFrame } = render(<ScanStep onComplete={onComplete} />);

    await vi.waitFor(() => {
      expect(onComplete).toHaveBeenCalledWith(mockResultsClean);
    });

    const frame = lastFrame();
    expect(frame).toContain('Your project looks clean! No warnings found.');
  });

  it('shows cached results immediately when scanResults prop is provided', () => {
    const onComplete = vi.fn();
    const { lastFrame } = render(
      <ScanStep onComplete={onComplete} scanResults={mockResultsWithWarnings} />
    );

    // Should not call scanProject when results are cached
    expect(mockScanProject).not.toHaveBeenCalled();

    const frame = lastFrame();
    expect(frame).toContain('Found 5 warnings across 3 files');
    expect(frame).toContain('Broad eslint-disable added');
  });

  it('calls onComplete with results when scan finishes', async () => {
    mockScanProject.mockResolvedValue(mockResultsWithWarnings);

    const onComplete = vi.fn();
    render(<ScanStep onComplete={onComplete} />);

    await vi.waitFor(() => {
      expect(onComplete).toHaveBeenCalledTimes(1);
      expect(onComplete).toHaveBeenCalledWith(mockResultsWithWarnings);
    });
  });

  it('shows footer with watch mode prompt', async () => {
    mockScanProject.mockResolvedValue(mockResultsWithWarnings);

    const onComplete = vi.fn();
    const { lastFrame } = render(<ScanStep onComplete={onComplete} />);

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Press Enter to start watch mode');
    });
  });

  it('shows component title', () => {
    mockScanProject.mockReturnValue(new Promise(() => {}));

    const onComplete = vi.fn();
    const { lastFrame } = render(<ScanStep onComplete={onComplete} />);

    expect(lastFrame()).toContain('Scan Your Project');
  });
});
