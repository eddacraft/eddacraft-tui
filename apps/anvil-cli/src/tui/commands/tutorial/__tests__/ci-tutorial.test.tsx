// @vitest-environment node
import { describe, it, expect, vi, beforeEach } from 'vitest';
import React from 'react';
import { render } from 'ink-testing-library';

// Use vi.hoisted to create mock functions before vi.mock hoisting
const { mockExistsSyncFn } = vi.hoisted(() => ({
  mockExistsSyncFn: vi.fn(() => false),
}));

// Mock filesystem operations
vi.mock('node:fs', async (importOriginal) => {
  const actual = await importOriginal<typeof import('node:fs')>();
  return {
    ...actual,
    default: {
      ...actual,
      existsSync: mockExistsSyncFn,
    },
    existsSync: mockExistsSyncFn,
    readdirSync: vi.fn(() => []),
    readFileSync: vi.fn(() => ''),
    statSync: vi.fn(() => ({ size: 100, isFile: () => true, isDirectory: () => false })),
  };
});

// Mock getWorkspaceRoot
vi.mock('../../../../utils/file-io.js', () => ({
  getWorkspaceRoot: () => '/mock/workspace',
}));

// Import after mocks
import { CITutorial, CI_STEPS, CI_STEP_DEFINITIONS } from '../features/CITutorial.js';
import { IntroStep } from '../features/ci-steps/IntroStep.js';
import { DetectStep } from '../features/ci-steps/DetectStep.js';
import { WorkflowStep } from '../features/ci-steps/WorkflowStep.js';
import { ExitCodesStep } from '../features/ci-steps/ExitCodesStep.js';
import { HooksStep } from '../features/ci-steps/HooksStep.js';
import { SummaryStep } from '../features/ci-steps/SummaryStep.js';

describe('CITutorial types', () => {
  describe('CI_STEPS', () => {
    it('has 6 steps', () => {
      expect(CI_STEPS).toHaveLength(6);
    });

    it('has correct step order', () => {
      expect(CI_STEPS).toEqual(['intro', 'detect', 'workflow', 'exit-codes', 'hooks', 'summary']);
    });
  });

  describe('CI_STEP_DEFINITIONS', () => {
    it('has a definition for each step', () => {
      for (const step of CI_STEPS) {
        expect(CI_STEP_DEFINITIONS[step]).toBeDefined();
        expect(CI_STEP_DEFINITIONS[step].title).toBeDefined();
        expect(CI_STEP_DEFINITIONS[step].description).toBeDefined();
      }
    });
  });
});

describe('CITutorial component', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useRealTimers();
    mockExistsSyncFn.mockReturnValue(false);
  });

  it('renders and shows "CI Integration" title', () => {
    const { lastFrame } = render(<CITutorial />);
    expect(lastFrame()).toContain('CI Integration');
  });

  it('shows the header with "ANVIL CI TUTORIAL"', () => {
    const { lastFrame } = render(<CITutorial />);
    expect(lastFrame()).toContain('ANVIL CI TUTORIAL');
  });

  it('shows progress indicator with step count', () => {
    const { lastFrame } = render(<CITutorial />);
    expect(lastFrame()).toContain('Step 1 of 6');
  });

  it('shows intro step description', () => {
    const { lastFrame } = render(<CITutorial />);
    expect(lastFrame()).toContain('Why CI integration matters for architecture enforcement');
  });

  it('shows keyboard shortcuts', () => {
    const { lastFrame } = render(<CITutorial />);
    expect(lastFrame()).toContain('q quit');
  });

  it('advances to next step on Enter', async () => {
    const { lastFrame, stdin } = render(<CITutorial />);

    expect(lastFrame()).toContain('Step 1 of 6');

    stdin.write('\r');

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Step 2 of 6');
    });
  });

  it('shows Detect CI System on step 2', async () => {
    const { lastFrame, stdin } = render(<CITutorial />);

    stdin.write('\r');

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Detect CI System');
    });
  });

  it('can navigate back with left arrow', async () => {
    const { lastFrame, stdin } = render(<CITutorial />);

    stdin.write('\r');

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Step 2 of 6');
    });

    stdin.write('\u001B[D');

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Step 1 of 6');
    });
  });

  it('can advance through multiple steps', async () => {
    const { lastFrame, stdin } = render(<CITutorial />);

    expect(lastFrame()).toContain('Step 1 of 6');

    stdin.write('\r');
    await vi.waitFor(
      () => {
        expect(lastFrame()).toContain('Step 2 of 6');
      },
      { timeout: 2000 }
    );

    stdin.write('\r');
    await vi.waitFor(
      () => {
        expect(lastFrame()).toContain('Step 3 of 6');
      },
      { timeout: 2000 }
    );
  });
});

describe('IntroStep', () => {
  it('shows the CI integration title', () => {
    const { lastFrame } = render(<IntroStep />);
    expect(lastFrame()).toContain('CI Integration');
  });

  it('explains three layers of protection', () => {
    const { lastFrame } = render(<IntroStep />);
    const frame = lastFrame();
    expect(frame).toContain('Watch mode');
    expect(frame).toContain('Pre-commit hooks');
    expect(frame).toContain('CI pipeline');
  });

  it('explains what Anvil does in CI', () => {
    const { lastFrame } = render(<IntroStep />);
    expect(lastFrame()).toContain('gate pull requests');
  });

  it('shows what the tutorial covers', () => {
    const { lastFrame } = render(<IntroStep />);
    const frame = lastFrame();
    expect(frame).toContain('Detect your CI system');
    expect(frame).toContain('exit codes');
    expect(frame).toContain('pre-commit hooks');
  });

  it('shows the continue prompt', () => {
    const { lastFrame } = render(<IntroStep />);
    expect(lastFrame()).toContain('Press Enter to continue');
  });
});

describe('DetectStep', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockExistsSyncFn.mockReturnValue(false);
  });

  it('shows the step title', () => {
    const { lastFrame } = render(<DetectStep />);
    expect(lastFrame()).toContain('Detect CI System');
  });

  it('shows "No CI configuration detected" when no CI files found', async () => {
    mockExistsSyncFn.mockReturnValue(false);
    const { lastFrame } = render(<DetectStep />);

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('No CI configuration detected');
    });
  });

  it('shows detected CI system when found', async () => {
    mockExistsSyncFn.mockImplementation((path: string) => {
      if (typeof path === 'string' && path.endsWith('.github/workflows')) return true;
      return false;
    });

    const { lastFrame } = render(<DetectStep />);

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('GitHub Actions');
    });
  });

  it('shows multiple detected CI systems', async () => {
    mockExistsSyncFn.mockImplementation((path: string) => {
      if (typeof path === 'string' && path.endsWith('.github/workflows')) return true;
      if (typeof path === 'string' && path.endsWith('.gitlab-ci.yml')) return true;
      return false;
    });

    const { lastFrame } = render(<DetectStep />);

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('GitHub Actions');
      expect(lastFrame()).toContain('GitLab CI');
    });
  });
});

describe('WorkflowStep', () => {
  it('shows the step title', () => {
    const { lastFrame } = render(<WorkflowStep />);
    expect(lastFrame()).toContain('Generate Workflow');
  });

  it('shows GitHub Actions YAML', () => {
    const { lastFrame } = render(<WorkflowStep />);
    const frame = lastFrame();
    expect(frame).toContain('Anvil Checks');
    expect(frame).toContain('actions/checkout@v4');
    expect(frame).toContain('npx anvil check --all --ci');
  });

  it('shows GitLab CI YAML', () => {
    const { lastFrame } = render(<WorkflowStep />);
    const frame = lastFrame();
    expect(frame).toContain('stage: test');
    expect(frame).toContain('npm ci');
  });

  it('explains the --ci flag', () => {
    const { lastFrame } = render(<WorkflowStep />);
    expect(lastFrame()).toContain('--ci');
    expect(lastFrame()).toContain('machine-readable');
  });

  it('mentions anvil hooks install', () => {
    const { lastFrame } = render(<WorkflowStep />);
    expect(lastFrame()).toContain('anvil hooks install');
  });
});

describe('ExitCodesStep', () => {
  it('shows the step title', () => {
    const { lastFrame } = render(<ExitCodesStep />);
    expect(lastFrame()).toContain('Exit Codes');
  });

  it('explains exit code 0', () => {
    const { lastFrame } = render(<ExitCodesStep />);
    expect(lastFrame()).toContain('Exit 0');
    expect(lastFrame()).toContain('All checks passed');
  });

  it('explains exit code 1', () => {
    const { lastFrame } = render(<ExitCodesStep />);
    expect(lastFrame()).toContain('Exit 1');
    expect(lastFrame()).toContain('Blocking warnings found');
  });

  it('shows CI usage example', () => {
    const { lastFrame } = render(<ExitCodesStep />);
    const frame = lastFrame();
    expect(frame).toContain('npx anvil check --all --ci');
    expect(frame).toContain('PR check passes');
    expect(frame).toContain('PR check fails');
  });

  it('mentions --json flag', () => {
    const { lastFrame } = render(<ExitCodesStep />);
    expect(lastFrame()).toContain('--json');
  });
});

describe('HooksStep', () => {
  it('shows the step title', () => {
    const { lastFrame } = render(<HooksStep />);
    expect(lastFrame()).toContain('Git Hooks');
  });

  it('shows the hooks install command', () => {
    const { lastFrame } = render(<HooksStep />);
    expect(lastFrame()).toContain('anvil hooks install');
  });

  it('explains anvil check --changed', () => {
    const { lastFrame } = render(<HooksStep />);
    expect(lastFrame()).toContain('anvil check --changed');
  });

  it('shows the layered protection diagram', () => {
    const { lastFrame } = render(<HooksStep />);
    const frame = lastFrame();
    expect(frame).toContain('Watch Mode');
    expect(frame).toContain('Pre-commit');
    expect(frame).toContain('CI Pipeline');
    expect(frame).toContain('instant, local');
    expect(frame).toContain('before push');
    expect(frame).toContain('before merge');
  });

  it('shows pre-commit hooks tip', () => {
    const { lastFrame } = render(<HooksStep />);
    expect(lastFrame()).toContain('Pre-commit hooks are optional');
  });
});

describe('SummaryStep', () => {
  it('shows the ready message', () => {
    const { lastFrame } = render(<SummaryStep />);
    expect(lastFrame()).toContain('CI integration ready!');
  });

  it('shows command reference', () => {
    const { lastFrame } = render(<SummaryStep />);
    const frame = lastFrame();
    expect(frame).toContain('anvil check --all --ci');
    expect(frame).toContain('anvil check --all --json');
    expect(frame).toContain('anvil hooks install');
    expect(frame).toContain('anvil hooks status');
  });

  it('shows workflow summary', () => {
    const { lastFrame } = render(<SummaryStep />);
    expect(lastFrame()).toContain('Watch locally');
    expect(lastFrame()).toContain('Hook on commit');
    expect(lastFrame()).toContain('CI on PR');
  });

  it('shows tutorial complete message', () => {
    const { lastFrame } = render(<SummaryStep />);
    expect(lastFrame()).toContain('Tutorial complete!');
  });

  it('shows cleanup instructions (c and q keys)', () => {
    const { lastFrame } = render(<SummaryStep />);
    const frame = lastFrame();
    expect(frame).toContain('clean up tutorial files');
    expect(frame).toContain('exit');
  });
});
