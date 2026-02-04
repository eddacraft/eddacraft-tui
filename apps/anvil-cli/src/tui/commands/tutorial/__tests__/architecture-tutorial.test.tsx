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
import {
  ArchitectureTutorial,
  ARCHITECTURE_STEPS,
  ARCHITECTURE_STEP_DEFINITIONS,
} from '../features/ArchitectureTutorial.js';
import { IntroStep } from '../features/architecture-steps/IntroStep.js';
import { DetectStep } from '../features/architecture-steps/DetectStep.js';
import { TemplateStep } from '../features/architecture-steps/TemplateStep.js';
import { ARCHITECTURE_TEMPLATES } from '../features/architecture-steps/TemplateStep.js';
import { CompileStep } from '../features/architecture-steps/CompileStep.js';
import { ValidateStep } from '../features/architecture-steps/ValidateStep.js';
import { SummaryStep } from '../features/architecture-steps/SummaryStep.js';

describe('ArchitectureTutorial types', () => {
  describe('ARCHITECTURE_STEPS', () => {
    it('has 6 steps', () => {
      expect(ARCHITECTURE_STEPS).toHaveLength(6);
    });

    it('has correct step order', () => {
      expect(ARCHITECTURE_STEPS).toEqual([
        'intro',
        'detect',
        'template',
        'compile',
        'validate',
        'summary',
      ]);
    });
  });

  describe('ARCHITECTURE_STEP_DEFINITIONS', () => {
    it('has a definition for each step', () => {
      for (const step of ARCHITECTURE_STEPS) {
        expect(ARCHITECTURE_STEP_DEFINITIONS[step]).toBeDefined();
        expect(ARCHITECTURE_STEP_DEFINITIONS[step].title).toBeDefined();
        expect(ARCHITECTURE_STEP_DEFINITIONS[step].description).toBeDefined();
      }
    });
  });
});

describe('ArchitectureTutorial component', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockExistsSyncFn.mockReturnValue(false);
  });

  it('renders and shows "Architecture Boundaries" title', () => {
    const { lastFrame } = render(<ArchitectureTutorial />);
    expect(lastFrame()).toContain('Architecture Boundaries');
  });

  it('shows the header with "ANVIL ARCHITECTURE TUTORIAL"', () => {
    const { lastFrame } = render(<ArchitectureTutorial />);
    expect(lastFrame()).toContain('ANVIL ARCHITECTURE TUTORIAL');
  });

  it('shows progress indicator with step count', () => {
    const { lastFrame } = render(<ArchitectureTutorial />);
    expect(lastFrame()).toContain('Step 1 of 6');
  });

  it('shows intro step description', () => {
    const { lastFrame } = render(<ArchitectureTutorial />);
    expect(lastFrame()).toContain('What architecture boundaries are and why they matter');
  });

  it('shows keyboard shortcuts', () => {
    const { lastFrame } = render(<ArchitectureTutorial />);
    expect(lastFrame()).toContain('q quit');
  });

  it('advances to next step on Enter', async () => {
    const { lastFrame, stdin } = render(<ArchitectureTutorial />);

    expect(lastFrame()).toContain('Step 1 of 6');

    stdin.write('\r');

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Step 2 of 6');
    });
  });

  it('shows Detect Structure on step 2', async () => {
    const { lastFrame, stdin } = render(<ArchitectureTutorial />);

    stdin.write('\r');

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Detect Structure');
    });
  });

  it('can navigate back with left arrow', async () => {
    const { lastFrame, stdin } = render(<ArchitectureTutorial />);

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
    const { lastFrame, stdin } = render(<ArchitectureTutorial />);

    expect(lastFrame()).toContain('Step 1 of 6');

    stdin.write('\r');
    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Step 2 of 6');
    });

    stdin.write('\r');
    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Step 3 of 6');
    });
  });
});

describe('IntroStep', () => {
  it('shows the boundary explanation', () => {
    const { lastFrame } = render(<IntroStep />);
    expect(lastFrame()).toContain('Architecture Boundaries');
    expect(lastFrame()).toContain('prevent imports from crossing layer contexts');
  });

  it('explains AI tools problem', () => {
    const { lastFrame } = render(<IntroStep />);
    expect(lastFrame()).toContain('AI tools');
    expect(lastFrame()).toContain('violates boundaries');
  });

  it('shows what the tutorial covers', () => {
    const { lastFrame } = render(<IntroStep />);
    const frame = lastFrame();
    expect(frame).toContain('Detect your project structure');
    expect(frame).toContain('Choose an architecture template');
    expect(frame).toContain('Understand how rules are compiled');
    expect(frame).toContain('Learn how to validate and fix violations');
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
    expect(lastFrame()).toContain('Detect Project Structure');
  });

  it('shows "No standard structure detected" when no directories found', async () => {
    mockExistsSyncFn.mockReturnValue(false);
    const { lastFrame } = render(<DetectStep />);

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('No standard structure detected');
    });
  });

  it('shows detected directories when found', async () => {
    mockExistsSyncFn.mockImplementation((path: string) => {
      if (typeof path === 'string' && path.endsWith('src/services')) return true;
      if (typeof path === 'string' && path.endsWith('src/utils')) return true;
      return false;
    });

    const { lastFrame } = render(<DetectStep />);

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('src/services');
      expect(lastFrame()).toContain('src/utils');
    });
  });

  it('explains template suggestion', () => {
    const { lastFrame } = render(<DetectStep />);
    expect(lastFrame()).toContain('architecture template');
  });
});

describe('TemplateStep', () => {
  it('shows the step title', () => {
    const { lastFrame } = render(<TemplateStep />);
    expect(lastFrame()).toContain('Choose a Template');
  });

  it('shows all 6 templates', () => {
    const { lastFrame } = render(<TemplateStep />);
    const frame = lastFrame();
    expect(frame).toContain('Starter');
    expect(frame).toContain('Layered');
    expect(frame).toContain('Hexagonal');
    expect(frame).toContain('Clean');
    expect(frame).toContain('DDD');
    expect(frame).toContain('Monorepo');
  });

  it('exports exactly 6 templates', () => {
    expect(ARCHITECTURE_TEMPLATES).toHaveLength(6);
  });

  it('shows the create command', () => {
    const { lastFrame } = render(<TemplateStep />);
    expect(lastFrame()).toContain('anvil architecture create');
  });

  it('mentions architecture.yaml', () => {
    const { lastFrame } = render(<TemplateStep />);
    expect(lastFrame()).toContain('architecture.yaml');
  });
});

describe('CompileStep', () => {
  it('shows the step title', () => {
    const { lastFrame } = render(<CompileStep />);
    expect(lastFrame()).toContain('Compile Architecture Rules');
  });

  it('shows the compile command', () => {
    const { lastFrame } = render(<CompileStep />);
    expect(lastFrame()).toContain('anvil architecture compile');
  });

  it('shows the compilation pipeline', () => {
    const { lastFrame } = render(<CompileStep />);
    const frame = lastFrame();
    expect(frame).toContain('architecture.yaml');
    expect(frame).toContain('dependency-constraints.json');
    expect(frame).toContain('Rego policies');
  });

  it('shows example Rego rule', () => {
    const { lastFrame } = render(<CompileStep />);
    const frame = lastFrame();
    expect(frame).toContain('package anvil.architecture.boundaries');
    expect(frame).toContain('violation[msg]');
  });
});

describe('ValidateStep', () => {
  it('shows the step title', () => {
    const { lastFrame } = render(<ValidateStep />);
    expect(lastFrame()).toContain('Validate Boundaries');
  });

  it('shows the validate command', () => {
    const { lastFrame } = render(<ValidateStep />);
    expect(lastFrame()).toContain('anvil architecture validate');
  });

  it('shows fix vs suppress options', () => {
    const { lastFrame } = render(<ValidateStep />);
    const frame = lastFrame();
    expect(frame).toContain('Fix');
    expect(frame).toContain('Suppress');
  });

  it('shows suppression syntax', () => {
    const { lastFrame } = render(<ValidateStep />);
    expect(lastFrame()).toContain('@anvil-ignore ARCH-001');
  });

  it('shows example violation output', () => {
    const { lastFrame } = render(<ValidateStep />);
    const frame = lastFrame();
    expect(frame).toContain('ARCH-001');
    expect(frame).toContain('ARCH-002');
  });
});

describe('SummaryStep', () => {
  it('shows the configured message', () => {
    const { lastFrame } = render(<SummaryStep />);
    expect(lastFrame()).toContain('Architecture boundaries configured!');
  });

  it('shows command reference', () => {
    const { lastFrame } = render(<SummaryStep />);
    const frame = lastFrame();
    expect(frame).toContain('anvil architecture create');
    expect(frame).toContain('anvil architecture compile');
    expect(frame).toContain('anvil architecture validate');
    expect(frame).toContain('anvil check --all');
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
