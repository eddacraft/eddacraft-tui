import React from 'react';
import { render } from 'ink-testing-library';
import { describe, it, expect, vi } from 'vitest';
import { ModeStep } from '../steps/ModeStep.js';
import { FormatStep } from '../steps/FormatStep.js';
import { DirectoryStep } from '../steps/DirectoryStep.js';
import { ChecksStep } from '../steps/ChecksStep.js';
import { SummaryStep } from '../steps/SummaryStep.js';
import { getDefaultState, type WizardState, type WizardContext, type StepProps } from '../types.js';

function createMockStepProps(overrides: Partial<WizardState> = {}): StepProps {
  const state: WizardState = { ...getDefaultState(), ...overrides };
  const context: WizardContext = {
    projectRoot: '/test/project',
    environment: {
      hasGit: true,
      hasPackageJson: true,
      hasEslint: true,
      hasPrettier: true,
      hasVitest: true,
      hasJest: false,
      hasTypeScript: true,
      packageManager: 'pnpm',
      projectName: 'test-project',
      projectRoot: '/test/project',
    },
    recommendedChecks: ['eslint', 'test', 'coverage', 'secret'],
  };

  return {
    state,
    context,
    onNext: vi.fn(),
    onBack: vi.fn(),
    onCancel: vi.fn(),
  };
}

describe('Init Wizard Steps', () => {
  describe('ModeStep', () => {
    it('renders mode options', () => {
      const props = createMockStepProps();
      const { lastFrame } = render(<ModeStep {...props} />);

      expect(lastFrame()).toContain('Standard');
      expect(lastFrame()).toContain('Strict');
      expect(lastFrame()).toContain('CI-optimised');
    });

    it('shows descriptions for each mode', () => {
      const props = createMockStepProps();
      const { lastFrame } = render(<ModeStep {...props} />);

      expect(lastFrame()).toContain('80% thresholds');
      expect(lastFrame()).toContain('90% thresholds');
    });
  });

  describe('FormatStep', () => {
    it('renders format options', () => {
      const props = createMockStepProps();
      const { lastFrame } = render(<FormatStep {...props} />);

      expect(lastFrame()).toContain('APS');
      expect(lastFrame()).toContain('SpecKit');
      expect(lastFrame()).toContain('BMAD');
      expect(lastFrame()).toContain('Generic Markdown');
      expect(lastFrame()).toContain('Skip examples');
    });
  });

  describe('DirectoryStep', () => {
    it('renders with default directory', () => {
      const props = createMockStepProps({ planningDir: 'docs/plans' });
      const { lastFrame } = render(<DirectoryStep {...props} />);

      expect(lastFrame()).toContain('docs/plans');
    });

    it('shows preview of file location', () => {
      const props = createMockStepProps({ planningDir: 'plans' });
      const { lastFrame } = render(<DirectoryStep {...props} />);

      expect(lastFrame()).toContain('./plans/');
    });
  });

  describe('ChecksStep', () => {
    it('renders check options', () => {
      const props = createMockStepProps();
      const { lastFrame } = render(<ChecksStep {...props} />);

      expect(lastFrame()).toContain('ESLint');
      expect(lastFrame()).toContain('Tests');
      expect(lastFrame()).toContain('Coverage');
      expect(lastFrame()).toContain('Secret Scanning');
    });

    it('shows detected indicator for available tools', () => {
      const props = createMockStepProps();
      const { lastFrame } = render(<ChecksStep {...props} />);

      expect(lastFrame()).toContain('(detected)');
    });

    it('shows toggle instructions', () => {
      const props = createMockStepProps();
      const { lastFrame } = render(<ChecksStep {...props} />);

      expect(lastFrame()).toContain('[Space] Toggle');
    });
  });

  describe('SummaryStep', () => {
    it('renders configuration summary', () => {
      const props = createMockStepProps({
        configTemplate: 'basic',
        format: 'speckit',
        planningDir: 'docs/plans',
        enabledChecks: ['eslint', 'test'],
      });
      const { lastFrame } = render(<SummaryStep {...props} />);

      expect(lastFrame()).toContain('Standard');
      expect(lastFrame()).toContain('SpecKit');
      expect(lastFrame()).toContain('docs/plans');
      expect(lastFrame()).toContain('eslint, test');
    });

    it('shows files to be created', () => {
      const props = createMockStepProps();
      const { lastFrame } = render(<SummaryStep {...props} />);

      expect(lastFrame()).toContain('.anvilrc');
      expect(lastFrame()).toContain('.anvil/');
    });

    it('shows confirmation prompt', () => {
      const props = createMockStepProps();
      const { lastFrame } = render(<SummaryStep {...props} />);

      expect(lastFrame()).toContain('Press Enter');
    });
  });
});
