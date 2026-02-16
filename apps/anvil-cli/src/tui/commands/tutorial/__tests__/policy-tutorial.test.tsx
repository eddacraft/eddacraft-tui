// @vitest-environment node
import { describe, it, expect, vi, beforeEach } from 'vitest';
import React from 'react';
import { render } from 'ink-testing-library';
import path from 'node:path';

// Use vi.hoisted to create mock functions before vi.mock hoisting
const { mockExistsSyncFn, mockMkdirSyncFn, mockWriteFileSyncFn } = vi.hoisted(() => ({
  mockExistsSyncFn: vi.fn(() => false),
  mockMkdirSyncFn: vi.fn(),
  mockWriteFileSyncFn: vi.fn(),
}));

// Mock filesystem operations
vi.mock('node:fs', async (importOriginal) => {
  const actual = await importOriginal<typeof import('node:fs')>();
  return {
    ...actual,
    default: {
      ...actual,
      existsSync: mockExistsSyncFn,
      mkdirSync: mockMkdirSyncFn,
      writeFileSync: mockWriteFileSyncFn,
    },
    existsSync: mockExistsSyncFn,
    mkdirSync: mockMkdirSyncFn,
    writeFileSync: mockWriteFileSyncFn,
    readdirSync: vi.fn(() => []),
    readFileSync: vi.fn(() => ''),
    statSync: vi.fn(() => ({ size: 100, isFile: () => true, isDirectory: () => false })),
  };
});

// Mock getWorkspaceRoot
vi.mock('../../../../utils/file-io.js', () => ({
  getWorkspaceRoot: () => '/mock/workspace',
}));

// Mock @eddacraft/anvil-runtime to avoid real OPA calls
vi.mock('@eddacraft/anvil-runtime', () => ({
  PolicyLoader: vi.fn().mockImplementation(() => ({
    loadPolicies: vi.fn(() =>
      Promise.resolve({
        policies: [],
        errors: [],
        directory: '/mock/workspace/.anvil/policies',
      })
    ),
  })),
}));

// Import after mocks
import {
  PolicyTutorial,
  POLICY_STEPS,
  POLICY_STEP_DEFINITIONS,
} from '../features/PolicyTutorial.js';
import { IntroStep } from '../features/policy-steps/IntroStep.js';
import { WritePolicyStep } from '../features/policy-steps/WritePolicyStep.js';
import { CustomiseStep } from '../features/policy-steps/CustomiseStep.js';
import { CreateDirStep } from '../features/policy-steps/CreateDirStep.js';

describe('PolicyTutorial types', () => {
  describe('POLICY_STEPS', () => {
    it('has 6 steps', () => {
      expect(POLICY_STEPS).toHaveLength(6);
    });

    it('has correct step order', () => {
      expect(POLICY_STEPS).toEqual([
        'intro',
        'create-dir',
        'write-policy',
        'test-policy',
        'see-fire',
        'customise',
      ]);
    });
  });

  describe('POLICY_STEP_DEFINITIONS', () => {
    it('has a definition for each step', () => {
      for (const step of POLICY_STEPS) {
        expect(POLICY_STEP_DEFINITIONS[step]).toBeDefined();
        expect(POLICY_STEP_DEFINITIONS[step].title).toBeDefined();
        expect(POLICY_STEP_DEFINITIONS[step].description).toBeDefined();
      }
    });
  });
});

describe('PolicyTutorial component', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockExistsSyncFn.mockReturnValue(false);
  });

  it('renders and shows "Write Your First Policy" title', () => {
    const { lastFrame } = render(<PolicyTutorial />);
    expect(lastFrame()).toContain('Write Your First Policy');
  });

  it('shows the header with "ANVIL POLICY TUTORIAL"', () => {
    const { lastFrame } = render(<PolicyTutorial />);
    expect(lastFrame()).toContain('ANVIL POLICY TUTORIAL');
  });

  it('shows progress indicator with step count', () => {
    const { lastFrame } = render(<PolicyTutorial />);
    expect(lastFrame()).toContain('Step 1 of 6');
  });

  it('shows intro step description', () => {
    const { lastFrame } = render(<PolicyTutorial />);
    expect(lastFrame()).toContain('What policies are and what you will learn');
  });

  it('shows keyboard shortcuts', () => {
    const { lastFrame } = render(<PolicyTutorial />);
    expect(lastFrame()).toContain('q quit');
  });

  it('shows intro content about OPA/Rego', () => {
    const { lastFrame } = render(<PolicyTutorial />);
    expect(lastFrame()).toContain('OPA/Rego');
  });

  it('advances to next step on Enter', async () => {
    const { lastFrame, stdin } = render(<PolicyTutorial />);

    expect(lastFrame()).toContain('Step 1 of 6');

    stdin.write('\r');

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Step 2 of 6');
    });
  });

  it('shows Create Policy Directory on step 2', async () => {
    const { lastFrame, stdin } = render(<PolicyTutorial />);

    stdin.write('\r');

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Create Policy Directory');
    });
  });

  it('can navigate back with left arrow', async () => {
    const { lastFrame, stdin } = render(<PolicyTutorial />);

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
    const { lastFrame, stdin } = render(<PolicyTutorial />);

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
  it('shows the intro title', () => {
    const { lastFrame } = render(<IntroStep />);
    expect(lastFrame()).toContain('Write Your First Policy');
  });

  it('shows OPA/Rego description', () => {
    const { lastFrame } = render(<IntroStep />);
    expect(lastFrame()).toContain('OPA/Rego');
  });

  it('shows policy features list', () => {
    const { lastFrame } = render(<IntroStep />);
    const frame = lastFrame();
    expect(frame).toContain('Create a policy directory');
    expect(frame).toContain('Write a max-file-length rule');
    expect(frame).toContain('Test it');
    expect(frame).toContain('See it trigger on your code');
  });

  it('shows the continue prompt', () => {
    const { lastFrame } = render(<IntroStep />);
    expect(lastFrame()).toContain('Press Enter to continue');
  });
});

describe('CreateDirStep', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockExistsSyncFn.mockReturnValue(false);
  });

  it('shows the step title', () => {
    const { lastFrame } = render(<CreateDirStep />);
    expect(lastFrame()).toContain('Create Policy Directory');
  });

  it('shows the policy directory path', () => {
    const { lastFrame } = render(<CreateDirStep />);
    expect(lastFrame()).toContain('.anvil/policies/');
  });

  it('creates the directory when it does not exist', async () => {
    mockExistsSyncFn.mockReturnValue(false);
    render(<CreateDirStep />);

    await vi.waitFor(() => {
      expect(mockMkdirSyncFn).toHaveBeenCalledWith(
        path.join('/mock/workspace', '.anvil', 'policies'),
        { recursive: true }
      );
    });
  });

  it('does not create the directory when it already exists', async () => {
    mockExistsSyncFn.mockReturnValue(true);
    render(<CreateDirStep />);

    await vi.waitFor(() => {
      expect(mockMkdirSyncFn).not.toHaveBeenCalled();
    });
  });

  it('shows "already exists" message when directory existed', async () => {
    mockExistsSyncFn.mockReturnValue(true);
    const { lastFrame } = render(<CreateDirStep />);

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('already exists');
    });
  });

  it('describes what the directory is for', () => {
    const { lastFrame } = render(<CreateDirStep />);
    const frame = lastFrame();
    expect(frame).toContain('custom Rego policies');
    expect(frame).toContain('.rego file');
  });
});

describe('WritePolicyStep', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockExistsSyncFn.mockReturnValue(false);
  });

  it('shows the step title', () => {
    const { lastFrame } = render(<WritePolicyStep />);
    expect(lastFrame()).toContain('Write Your First Policy');
  });

  it('shows the Rego code', () => {
    const { lastFrame } = render(<WritePolicyStep />);
    const frame = lastFrame();
    expect(frame).toContain('package anvil.policies.max_file_length');
    expect(frame).toContain('default max_lines := 300');
    expect(frame).toContain('violation[msg]');
  });

  it('shows the policy file path', () => {
    const { lastFrame } = render(<WritePolicyStep />);
    expect(lastFrame()).toContain('.anvil/policies/max_file_length.rego');
  });

  it('writes the policy file to disk', async () => {
    render(<WritePolicyStep />);

    await vi.waitFor(() => {
      expect(mockWriteFileSyncFn).toHaveBeenCalledWith(
        path.join('/mock/workspace', '.anvil', 'policies', 'max_file_length.rego'),
        expect.stringContaining('package anvil.policies.max_file_length'),
        'utf-8'
      );
    });
  });

  it('shows the success message after writing', async () => {
    const { lastFrame } = render(<WritePolicyStep />);

    await vi.waitFor(() => {
      expect(lastFrame()).toContain('Policy written to');
    });
  });
});

describe('CustomiseStep', () => {
  it('shows the step title', () => {
    const { lastFrame } = render(<CustomiseStep />);
    expect(lastFrame()).toContain('Customise Your Policy');
  });

  it('shows the edit instructions', () => {
    const { lastFrame } = render(<CustomiseStep />);
    const frame = lastFrame();
    expect(frame).toContain('default max_lines := 300');
    expect(frame).toContain('default max_lines := 200');
  });

  it('shows gate-config.json configuration example', () => {
    const { lastFrame } = render(<CustomiseStep />);
    const frame = lastFrame();
    expect(frame).toContain('"policies"');
    expect(frame).toContain('"max_file_length"');
    expect(frame).toContain('"max_lines": 200');
  });

  it('shows more policy ideas', () => {
    const { lastFrame } = render(<CustomiseStep />);
    const frame = lastFrame();
    expect(frame).toContain('Banned imports');
    expect(frame).toContain('Naming conventions');
    expect(frame).toContain('Max function complexity');
  });

  it('shows tutorial complete message', () => {
    const { lastFrame } = render(<CustomiseStep />);
    expect(lastFrame()).toContain('Tutorial complete! Your policy is active.');
  });

  it('shows cleanup instructions (c and q keys)', () => {
    const { lastFrame } = render(<CustomiseStep />);
    const frame = lastFrame();
    expect(frame).toContain('clean up tutorial files');
    expect(frame).toContain('exit');
  });
});
