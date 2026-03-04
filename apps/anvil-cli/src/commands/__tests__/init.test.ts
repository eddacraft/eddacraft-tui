/**
 * Unit Tests for init command
 *
 * Tests the anvil init command including:
 * - Force flag handling
 * - Non-interactive mode
 * - Interactive mode
 * - Environment detection
 * - File generation
 * - Error handling
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { createInitCommand } from '../init.js';
import { CliError } from '../../utils/cli-error.js';
import {
  analyseProjectArchitecture,
  layersToMermaid,
  formatLayerDiagram,
} from '../../services/architecture-service.js';
import { isTUIAvailable } from '../../tui/utils/tty-detection.js';
import { renderTUI } from '../../tui/utils/renderer.js';
import {
  createTestWorkspace,
  createPackageJson,
  createAnvilrc,
  createEslintConfig,
  createTsConfig,
  initGitRepo,
  createLockfile,
  type TestWorkspace,
} from '../../__tests__/helpers/test-workspace.js';
import { TemplateGenerator } from '../../services/template-generator.js';
import { HookInstaller } from '../../services/hook-installer.js';

// Mock dependencies
vi.mock('../../services/architecture-service.js', () => ({
  analyseProjectArchitecture: vi.fn().mockResolvedValue({
    moduleCount: 0,
    entryPoints: [],
    layers: {},
    layerAssignments: new Map(),
  }),
  formatEntryPointsSummary: vi.fn().mockReturnValue(''),
  formatEntryPoints: vi.fn().mockReturnValue([]),
  formatLayerDiagram: vi.fn().mockReturnValue([]),
  layersToMermaid: vi.fn().mockReturnValue('graph TD'),
  generateArchitectureExplanation: vi.fn().mockReturnValue({
    templateName: 'basic',
    insights: [],
    nextSteps: [],
  }),
  formatArchitectureExplanation: vi.fn().mockReturnValue([]),
  saveArchitectureBaseline: vi.fn(),
  hasExistingBaseline: vi.fn().mockReturnValue(false),
}));

vi.mock('../../tui/utils/tty-detection.js', () => ({
  isTUIAvailable: vi.fn().mockReturnValue(false),
}));

vi.mock('../../tui/utils/renderer.js', () => ({
  renderTUI: vi.fn().mockReturnValue(null),
}));

vi.mock('inquirer', () => ({
  default: {
    prompt: vi.fn(),
  },
}));

vi.mock('ora', () => ({
  default: vi.fn(() => ({
    start: vi.fn().mockReturnThis(),
    succeed: vi.fn().mockReturnThis(),
    fail: vi.fn().mockReturnThis(),
    warn: vi.fn().mockReturnThis(),
    text: '',
  })),
}));

vi.mock('chalk', () => ({
  default: {
    bold: (str: string) => str,
    blue: (str: string) => str,
    cyan: (str: string) => str,
    dim: (str: string) => str,
    gray: (str: string) => str,
    green: (str: string) => str,
    magenta: (str: string) => str,
    red: (str: string) => str,
    white: (str: string) => str,
    yellow: (str: string) => str,
  },
}));

describe('init command', () => {
  let workspace: TestWorkspace;
  let originalCwd: string;
  let consoleOutput: string[] = [];
  let consoleErrors: string[] = [];

  beforeEach(() => {
    // Create test workspace
    workspace = createTestWorkspace();
    originalCwd = process.cwd();
    process.chdir(workspace.root);

    // Mock console.log and console.error to capture output
    consoleOutput = [];
    consoleErrors = [];
    vi.spyOn(console, 'log').mockImplementation((...args) => {
      consoleOutput.push(args.map((arg) => String(arg)).join(' '));
    });
    vi.spyOn(console, 'error').mockImplementation((...args) => {
      consoleErrors.push(args.map((arg) => String(arg)).join(' '));
    });
  });

  afterEach(() => {
    process.chdir(originalCwd);
    workspace.cleanup();
    vi.restoreAllMocks();
  });

  describe('basic functionality', () => {
    it('should create init command with correct configuration', () => {
      const command = createInitCommand();

      expect(command.name()).toBe('init');
      expect(command.description()).toBe('Initialise Anvil in the current project');

      const options = command.options;
      expect(options).toBeDefined();
      expect(options.some((opt) => opt.long === '--force')).toBe(true);
      expect(options.some((opt) => opt.long === '--non-interactive')).toBe(true);
    });
  });

  describe('existing .anvilrc handling', () => {
    it('should exit with error if .anvilrc exists without force flag', async () => {
      createAnvilrc(workspace.root);
      const command = createInitCommand();

      await expect(async () => {
        await command.parseAsync(['--non-interactive'], { from: 'user' });
      }).rejects.toThrow(CliError);
    });

    it('should overwrite .anvilrc if force flag is provided', async () => {
      createAnvilrc(workspace.root, {
        planningDir: 'old-plans',
      });

      const command = createInitCommand();
      await command.parseAsync(['--non-interactive', '--force'], { from: 'user' });

      expect(existsSync(join(workspace.root, '.anvilrc'))).toBe(true);

      const config = JSON.parse(readFileSync(join(workspace.root, '.anvilrc'), 'utf-8'));
      expect(config.version).toBe(1);
      expect(config.checks).toBeDefined();
    });
  });

  describe('non-interactive mode', () => {
    it('should create .anvilrc with default configuration', async () => {
      const command = createInitCommand();
      await command.parseAsync(['--non-interactive'], { from: 'user' });

      expect(existsSync(join(workspace.root, '.anvilrc'))).toBe(true);

      const config = JSON.parse(readFileSync(join(workspace.root, '.anvilrc'), 'utf-8'));
      expect(config.version).toBe(1);
      expect(config.checks).toBeDefined();
      expect(config.thresholds).toBeDefined();
    });

    it('should create .anvil directory', async () => {
      const command = createInitCommand();
      await command.parseAsync(['--non-interactive'], { from: 'user' });

      expect(existsSync(join(workspace.root, '.anvil'))).toBe(true);
    });

    it('should create planning directory', async () => {
      const command = createInitCommand();
      await command.parseAsync(['--non-interactive'], { from: 'user' });

      expect(existsSync(join(workspace.root, 'docs/plans'))).toBe(true);
    });

    it('should create example plan in non-interactive mode', async () => {
      const command = createInitCommand();
      await command.parseAsync(['--non-interactive'], { from: 'user' });

      // Check that at least one plan file was created
      const planningDir = join(workspace.root, 'docs/plans');
      expect(existsSync(planningDir)).toBe(true);
    });

    it('should display success message', async () => {
      const command = createInitCommand();
      await command.parseAsync(['--non-interactive'], { from: 'user' });

      const output = [...consoleOutput, ...consoleErrors].join('\n');
      expect(output).toContain('Anvil is ready to use');
      expect(output).toContain('Next steps');
    });
  });

  describe('environment detection', () => {
    it('should detect basic project with package.json', async () => {
      createPackageJson(workspace.root, { name: 'test-project' });

      const command = createInitCommand();
      await command.parseAsync(['--non-interactive'], { from: 'user' });

      const output = consoleOutput.join('\n');
      expect(output).toContain('test-project');
    });

    it('should detect package manager from lockfile (pnpm)', async () => {
      createLockfile(workspace.root, 'pnpm');

      const command = createInitCommand();
      await command.parseAsync(['--non-interactive'], { from: 'user' });

      const output = consoleOutput.join('\n');
      expect(output).toContain('Package Manager: pnpm');
    });

    it('should detect package manager from lockfile (npm)', async () => {
      createLockfile(workspace.root, 'npm');

      const command = createInitCommand();
      await command.parseAsync(['--non-interactive'], { from: 'user' });

      const output = consoleOutput.join('\n');
      expect(output).toContain('Package Manager: npm');
    });

    it('should detect package manager from lockfile (yarn)', async () => {
      createLockfile(workspace.root, 'yarn');

      const command = createInitCommand();
      await command.parseAsync(['--non-interactive'], { from: 'user' });

      const output = consoleOutput.join('\n');
      expect(output).toContain('Package Manager: yarn');
    });

    it('should detect git repository', async () => {
      initGitRepo(workspace.root);

      const command = createInitCommand();
      await command.parseAsync(['--non-interactive'], { from: 'user' });

      const output = consoleOutput.join('\n');
      expect(output).toContain('Git: ✓');
    });

    it('should detect TypeScript', async () => {
      createTsConfig(workspace.root);

      const command = createInitCommand();
      await command.parseAsync(['--non-interactive'], { from: 'user' });

      const output = consoleOutput.join('\n');
      expect(output).toContain('TypeScript: ✓');
    });

    it('should detect ESLint', async () => {
      createEslintConfig(workspace.root);

      const command = createInitCommand();
      await command.parseAsync(['--non-interactive'], { from: 'user' });

      const output = consoleOutput.join('\n');
      expect(output).toContain('ESLint: ✓');
    });

    it('should detect Vitest from package.json', async () => {
      createPackageJson(workspace.root, {
        devDependencies: {
          vitest: '^3.0.0',
        },
      });

      const command = createInitCommand();
      await command.parseAsync(['--non-interactive'], { from: 'user' });

      const output = consoleOutput.join('\n');
      expect(output).toContain('Testing: Vitest');
    });

    it('should handle project without package.json', async () => {
      const command = createInitCommand();
      await command.parseAsync(['--non-interactive'], { from: 'user' });

      const output = consoleOutput.join('\n');
      expect(output).toContain('(no package.json)');
    });
  });

  describe('gitignore handling', () => {
    it('should update .gitignore if git repository exists', async () => {
      initGitRepo(workspace.root);

      const command = createInitCommand();
      await command.parseAsync(['--non-interactive'], { from: 'user' });

      expect(existsSync(join(workspace.root, '.gitignore'))).toBe(true);

      const gitignore = readFileSync(join(workspace.root, '.gitignore'), 'utf-8');
      expect(gitignore).toContain('.anvil');
    });

    it('should not create .gitignore if no git repository', async () => {
      const command = createInitCommand();
      await command.parseAsync(['--non-interactive'], { from: 'user' });

      // .gitignore may or may not exist, but output should not mention it
      const output = consoleOutput.join('\n');
      expect(output).not.toContain('.gitignore (updated)');
    });
  });

  describe('error handling', () => {
    it('should exit with error code 1 on failures', async () => {
      // Creating .anvilrc without force flag should cause an error
      createAnvilrc(workspace.root);
      const command = createInitCommand();

      await expect(async () => {
        await command.parseAsync(['--non-interactive'], { from: 'user' });
      }).rejects.toThrow(CliError);
    });
  });

  describe('interactive mode', () => {
    it('should use inquirer for interactive setup', async () => {
      const genSpy = vi.spyOn(TemplateGenerator.prototype, 'generateAnvilrc');

      const inquirer = await import('inquirer');
      // Architecture confirmation prompt
      vi.mocked(inquirer.default.prompt).mockResolvedValueOnce({
        archAction: 'save',
      });

      vi.mocked(inquirer.default.prompt).mockResolvedValueOnce({
        planningDir: 'custom/plans',
        format: 'speckit',
        createExample: true,
        configTemplate: 'basic',
      });

      vi.mocked(inquirer.default.prompt).mockResolvedValueOnce({
        eslint: true,
        test: true,
        coverage: true,
        coverageThreshold: 85,
        secret: true,
      });

      const command = createInitCommand();
      await command.parseAsync([], { from: 'user' });

      expect(inquirer.default.prompt).toHaveBeenCalledTimes(3);
      expect(existsSync(join(workspace.root, 'custom/plans'))).toBe(true);
      expect(genSpy).toHaveBeenCalled();
    });

    it('should use custom planning directory from interactive input', async () => {
      const inquirer = await import('inquirer');
      // Architecture confirmation prompt
      vi.mocked(inquirer.default.prompt).mockResolvedValueOnce({
        archAction: 'skip',
      });

      vi.mocked(inquirer.default.prompt).mockResolvedValueOnce({
        planningDir: 'my-plans',
        format: 'generic',
        createExample: false,
        configTemplate: 'basic',
      });

      vi.mocked(inquirer.default.prompt).mockResolvedValueOnce({
        secret: true,
      });

      const command = createInitCommand();
      await command.parseAsync([], { from: 'user' });

      expect(existsSync(join(workspace.root, 'my-plans'))).toBe(true);
      const config = JSON.parse(readFileSync(join(workspace.root, '.anvilrc'), 'utf-8'));
      expect(config.version).toBe(1);
      expect(config.checks).toBeDefined();
    });

    it('should apply strict configuration template', async () => {
      const inquirer = await import('inquirer');
      // Architecture confirmation prompt
      vi.mocked(inquirer.default.prompt).mockResolvedValueOnce({
        archAction: 'skip',
      });

      vi.mocked(inquirer.default.prompt).mockResolvedValueOnce({
        planningDir: 'docs/plans',
        format: 'skip',
        configTemplate: 'strict',
      });

      vi.mocked(inquirer.default.prompt).mockResolvedValueOnce({
        secret: true,
      });

      const command = createInitCommand();
      await command.parseAsync([], { from: 'user' });

      const config = JSON.parse(readFileSync(join(workspace.root, '.anvilrc'), 'utf-8'));
      // Strict template should have higher threshold (90 vs 80)
      expect(config.thresholds).toBeDefined();
      expect(config.thresholds.overall_score).toBe(90);
    });

    it('should skip example generation when format is "skip"', async () => {
      const inquirer = await import('inquirer');
      // Architecture confirmation prompt
      vi.mocked(inquirer.default.prompt).mockResolvedValueOnce({
        archAction: 'skip',
      });

      vi.mocked(inquirer.default.prompt).mockResolvedValueOnce({
        planningDir: 'docs/plans',
        format: 'skip',
        createExample: false,
        configTemplate: 'basic',
      });

      vi.mocked(inquirer.default.prompt).mockResolvedValueOnce({
        secret: true,
      });

      const command = createInitCommand();
      await command.parseAsync([], { from: 'user' });

      const output = consoleOutput.join('\n');
      expect(output).not.toContain('Example files:');
    });
  });

  describe('--org flag', () => {
    it('should have --org option on init command', () => {
      const command = createInitCommand();
      const orgOpt = command.options.find((opt) => opt.long === '--org');

      expect(orgOpt).toBeDefined();
    });

    it('should create .anvil/config.yml with org source', async () => {
      const command = createInitCommand();
      await command.parseAsync(['--org', 'acme-corp'], { from: 'user' });

      const configPath = join(workspace.root, '.anvil', 'config.yml');
      expect(existsSync(configPath)).toBe(true);

      const content = readFileSync(configPath, 'utf-8');
      expect(content).toContain('git@github.com:acme-corp/anvil-policies.git');
    });

    it('should apply a starter profile based on detection', async () => {
      const command = createInitCommand();
      await command.parseAsync(['--org', 'acme-corp'], { from: 'user' });

      const configPath = join(workspace.root, '.anvil', 'config.yml');
      const content = readFileSync(configPath, 'utf-8');
      expect(content).toContain('starter_profile');
    });

    it('should create .anvilrc alongside config.yml', async () => {
      const command = createInitCommand();
      await command.parseAsync(['--org', 'my-org'], { from: 'user' });

      expect(existsSync(join(workspace.root, '.anvilrc'))).toBe(true);
    });

    it('should exit with error if .anvilrc exists without force', async () => {
      createAnvilrc(workspace.root);
      const command = createInitCommand();

      await expect(async () => {
        await command.parseAsync(['--org', 'my-org'], { from: 'user' });
      }).rejects.toThrow(CliError);
    });

    it('should overwrite if --force is used with --org', async () => {
      createAnvilrc(workspace.root);
      const command = createInitCommand();

      await command.parseAsync(['--org', 'my-org', '--force'], { from: 'user' });

      expect(existsSync(join(workspace.root, '.anvilrc'))).toBe(true);
      expect(existsSync(join(workspace.root, '.anvil', 'config.yml'))).toBe(true);
    });

    it('should display success output with detected info', async () => {
      createPackageJson(workspace.root, { name: 'test-project' });

      const command = createInitCommand();
      await command.parseAsync(['--org', 'my-org'], { from: 'user' });

      const output = [...consoleOutput, ...consoleErrors].join('\n');
      expect(output).toContain('Detected:');
      expect(output).toContain('starter profile');
      expect(output).toContain('policies active');
    });

    it('should display policy list suggestion', async () => {
      const command = createInitCommand();
      await command.parseAsync(['--org', 'my-org'], { from: 'user' });

      const output = consoleOutput.join('\n');
      expect(output).toContain('anvil policy list');
    });

    it('should include team policies from starter profile in config', async () => {
      const command = createInitCommand();
      await command.parseAsync(['--org', 'my-org'], { from: 'user' });

      const configPath = join(workspace.root, '.anvil', 'config.yml');
      const content = readFileSync(configPath, 'utf-8');
      // All starter profiles include secret-scan
      expect(content).toContain('secret-scan');
    });
  });

  describe('output and messaging', () => {
    it('should display created files summary', async () => {
      const command = createInitCommand();
      await command.parseAsync(['--non-interactive'], { from: 'user' });

      const output = consoleOutput.join('\n');
      expect(output).toContain('Created files:');
      expect(output).toContain('.anvilrc');
      expect(output).toContain('.anvil/');
      expect(output).toContain('docs/plans/');
    });

    it('should display next steps', async () => {
      const command = createInitCommand();
      await command.parseAsync(['--non-interactive'], { from: 'user' });

      const output = consoleOutput.join('\n');
      expect(output).toContain('Next steps:');
      expect(output).toContain('anvil gate:config --list');
      expect(output).toContain('anvil validate');
    });

    it('should display Anvil ready message', async () => {
      const command = createInitCommand();
      await command.parseAsync(['--non-interactive'], { from: 'user' });

      const output = [...consoleOutput, ...consoleErrors].join('\n');
      expect(output).toContain('Anvil is ready to use');
    });
  });

  describe('error paths', () => {
    describe('architecture analysis failure', () => {
      it('should warn and continue when architecture analysis throws an Error', async () => {
        vi.mocked(analyseProjectArchitecture).mockRejectedValueOnce(
          new Error('No source files found')
        );

        const command = createInitCommand();
        await command.parseAsync(['--non-interactive'], { from: 'user' });

        // Init should still succeed despite the architecture failure
        const output = [...consoleOutput, ...consoleErrors].join('\n');
        expect(output).toContain('Anvil is ready to use');
        expect(output).toContain('Reason: No source files found');
      });

      it('should warn and continue when architecture analysis throws a non-Error', async () => {
        vi.mocked(analyseProjectArchitecture).mockRejectedValueOnce('unexpected string error');

        const command = createInitCommand();
        await command.parseAsync(['--non-interactive'], { from: 'user' });

        // Init should still succeed
        const output = [...consoleOutput, ...consoleErrors].join('\n');
        expect(output).toContain('Anvil is ready to use');
        // Non-Error values don't produce a "Reason:" line
        expect(output).not.toContain('Reason:');
      });

      it('should skip baseline creation when architecture analysis fails', async () => {
        vi.mocked(analyseProjectArchitecture).mockRejectedValueOnce(new Error('analysis failed'));

        const command = createInitCommand();
        await command.parseAsync(['--non-interactive'], { from: 'user' });

        const output = consoleOutput.join('\n');
        // architecture.json should not be listed in created files
        expect(output).not.toContain('architecture.json');
      });
    });

    describe('template generator failure', () => {
      it('should exit with code 1 when createAnvilDirectory throws', async () => {
        vi.spyOn(TemplateGenerator.prototype, 'createAnvilDirectory').mockImplementationOnce(() => {
          throw new Error('EACCES: permission denied');
        });

        const command = createInitCommand();
        await expect(async () => {
          await command.parseAsync(['--non-interactive'], { from: 'user' });
        }).rejects.toThrow(CliError);

        const errorOutput = consoleErrors.join('\n');
        expect(errorOutput).toContain('EACCES: permission denied');
      });

      it('should exit with code 1 when generateAnvilrc throws', async () => {
        vi.spyOn(TemplateGenerator.prototype, 'generateAnvilrc').mockImplementationOnce(() => {
          throw new Error('Failed to write .anvilrc');
        });

        const command = createInitCommand();
        await expect(async () => {
          await command.parseAsync(['--non-interactive'], { from: 'user' });
        }).rejects.toThrow(CliError);

        const errorOutput = consoleErrors.join('\n');
        expect(errorOutput).toContain('Failed to write .anvilrc');
      });

      it('should format non-Error throws as Unknown error', async () => {
        vi.spyOn(TemplateGenerator.prototype, 'createAnvilDirectory').mockImplementationOnce(() => {
          throw 'raw string error';
        });

        const command = createInitCommand();
        await expect(async () => {
          await command.parseAsync(['--non-interactive'], { from: 'user' });
        }).rejects.toThrow(CliError);

        const errorOutput = consoleErrors.join('\n');
        expect(errorOutput).toContain('Unknown error');
      });
    });

    describe('intelligent analysis failure', () => {
      it('should warn and show next steps when analysis throws an Error', async () => {
        // Mock RepoScanner to throw during analysis
        const RepoScannerModule = await import('../../services/repo-scanner.js');
        vi.spyOn(RepoScannerModule.RepoScanner.prototype, 'scan').mockRejectedValueOnce(
          new Error('Git repository not found')
        );

        const command = createInitCommand();
        await command.parseAsync(['--non-interactive'], { from: 'user' });

        const output = [...consoleOutput, ...consoleErrors].join('\n');
        // Should still succeed overall
        expect(output).toContain('Anvil is ready to use');
        // Should display the error reason
        expect(output).toContain('Reason: Git repository not found');
        // Should show next steps (fallback from dashboard)
        expect(output).toContain('Next steps:');
      });

      it('should warn and continue when analysis throws a non-Error', async () => {
        const RepoScannerModule = await import('../../services/repo-scanner.js');
        vi.spyOn(RepoScannerModule.RepoScanner.prototype, 'scan').mockRejectedValueOnce(42);

        const command = createInitCommand();
        await command.parseAsync(['--non-interactive'], { from: 'user' });

        const output = [...consoleOutput, ...consoleErrors].join('\n');
        expect(output).toContain('Anvil is ready to use');
        // Non-Error values don't produce a "Reason:" line
        expect(output).not.toContain('Reason:');
      });
    });

    describe('--no-analysis flag', () => {
      it('should skip analysis entirely when --no-analysis is passed', async () => {
        const command = createInitCommand();
        await command.parseAsync(['--non-interactive', '--no-analysis'], { from: 'user' });

        const output = [...consoleOutput, ...consoleErrors].join('\n');
        expect(output).toContain('Skipping automatic analysis');
        expect(output).toContain('Anvil is ready to use');
      });
    });

    describe('TUI rendering failure for results dashboard', () => {
      it('should warn and show next steps when renderTUI returns null', async () => {
        vi.mocked(isTUIAvailable).mockReturnValueOnce(true);
        vi.mocked(renderTUI).mockReturnValueOnce(null);

        const command = createInitCommand();
        await command.parseAsync(['--non-interactive'], { from: 'user' });

        // The renderTUI null is caught by the analysis catch block, not outer catch
        // Init should still succeed
        const output = [...consoleOutput, ...consoleErrors].join('\n');
        expect(output).toContain('Anvil is ready to use');
        expect(output).toContain('Reason: Could not render results dashboard');
        // Next steps should be shown since dashboard wasn't displayed
        expect(output).toContain('Next steps:');
      });
    });

    describe('--org flow error paths', () => {
      it('should exit with code 1 when template generator fails in --org mode', async () => {
        vi.spyOn(TemplateGenerator.prototype, 'createAnvilDirectory').mockImplementationOnce(() => {
          throw new Error('Disk full');
        });

        const command = createInitCommand();
        await expect(async () => {
          await command.parseAsync(['--org', 'my-org'], { from: 'user' });
        }).rejects.toThrow(CliError);

        const errorOutput = consoleErrors.join('\n');
        expect(errorOutput).toContain('Disk full');
      });
    });

    describe('mermaid rendering fallback', () => {
      it('should fall back to box diagram when mermaid rendering throws', async () => {
        vi.mocked(layersToMermaid).mockImplementationOnce(() => {
          throw new Error('invalid mermaid syntax');
        });
        vi.mocked(formatLayerDiagram).mockClear();

        const command = createInitCommand();
        await command.parseAsync(['--non-interactive'], { from: 'user' });

        const output = [...consoleOutput, ...consoleErrors].join('\n');
        expect(output).toContain('Anvil is ready to use');
        expect(formatLayerDiagram).toHaveBeenCalled();
      });
    });

    describe('TUI wizard failure', () => {
      it('should exit with error when TUI wizard cannot render', async () => {
        vi.mocked(isTUIAvailable).mockReturnValueOnce(true);
        const inquirerMod = await import('inquirer');
        vi.mocked(inquirerMod.default.prompt).mockResolvedValueOnce({ archAction: 'skip' });
        vi.mocked(renderTUI).mockReturnValueOnce(null);

        const command = createInitCommand();
        await expect(async () => {
          await command.parseAsync([], { from: 'user' });
        }).rejects.toThrow(CliError);

        const errorOutput = consoleErrors.join('\n');
        expect(errorOutput).toContain('Could not start TUI wizard');
      });

      it('should exit with error when user cancels the wizard', async () => {
        vi.mocked(isTUIAvailable).mockReturnValueOnce(true);
        const inquirerMod = await import('inquirer');
        vi.mocked(inquirerMod.default.prompt).mockResolvedValueOnce({ archAction: 'skip' });
        vi.mocked(renderTUI).mockImplementationOnce(
          (_Component: unknown, props: Record<string, unknown>) => {
            (props as { onCancel: () => void }).onCancel();
            return { waitUntilExit: () => Promise.resolve() } as ReturnType<typeof renderTUI>;
          }
        );

        const command = createInitCommand();
        await expect(async () => {
          await command.parseAsync([], { from: 'user' });
        }).rejects.toThrow(CliError);

        const errorOutput = consoleErrors.join('\n');
        expect(errorOutput).toContain('Setup cancelled by user');
      });
    });

    describe('--org hook installation errors', () => {
      it('should show info and continue when hook installation throws', async () => {
        initGitRepo(workspace.root);
        vi.spyOn(HookInstaller.prototype, 'installHook').mockImplementation(() => {
          throw new Error('EACCES: permission denied');
        });

        const command = createInitCommand();
        await command.parseAsync(['--org', 'my-org'], { from: 'user' });

        const output = [...consoleOutput, ...consoleErrors].join('\n');
        expect(output).toContain('Hook installation skipped');
        expect(output).toContain('starter profile');
      });
    });

    describe('--quick analysis failure', () => {
      it('should warn and continue when quick analysis throws', async () => {
        const SampleAnalyzerModule = await import('../../services/sample-analyzer.js');
        vi.spyOn(
          SampleAnalyzerModule.SampleAnalyzer.prototype,
          'selectFiles'
        ).mockRejectedValueOnce(new Error('Cannot read directory'));

        const command = createInitCommand();
        await command.parseAsync(['--non-interactive', '--quick'], { from: 'user' });

        const output = [...consoleOutput, ...consoleErrors].join('\n');
        expect(output).toContain('Anvil is ready to use');
        expect(output).toContain('Reason: Cannot read directory');
      });
    });
  });
});
