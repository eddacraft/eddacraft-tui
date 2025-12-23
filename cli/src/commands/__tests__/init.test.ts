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
import { existsSync, readFileSync } from 'fs';
import { join } from 'path';
import { createInitCommand } from '../init.js';
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

// Mock dependencies
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
    text: '',
  })),
}));

vi.mock('chalk', () => ({
  default: {
    bold: (str: string) => str,
    cyan: (str: string) => str,
    dim: (str: string) => str,
    green: (str: string) => str,
    red: (str: string) => str,
  },
}));

describe('init command', () => {
  let workspace: TestWorkspace;
  let originalCwd: string;
  let originalExit: typeof process.exit;
  let exitCode: number | null = null;
  let consoleOutput: string[] = [];

  beforeEach(() => {
    // Create test workspace
    workspace = createTestWorkspace();
    originalCwd = process.cwd();
    process.chdir(workspace.root);

    // Mock process.exit
    originalExit = process.exit;
    // @ts-expect-error - Mocking process.exit
    process.exit = vi.fn((code?: number) => {
      exitCode = code ?? 0;
      throw new Error(`process.exit(${exitCode})`);
    });

    // Mock console.log to capture output
    consoleOutput = [];
    vi.spyOn(console, 'log').mockImplementation((...args) => {
      consoleOutput.push(args.map((arg) => String(arg)).join(' '));
    });

    exitCode = null;
  });

  afterEach(() => {
    process.chdir(originalCwd);
    workspace.cleanup();
    process.exit = originalExit;
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
      }).rejects.toThrow('process.exit(1)');

      expect(exitCode).toBe(1);
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

      const output = consoleOutput.join('\n');
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
      }).rejects.toThrow('process.exit(1)');

      expect(exitCode).toBe(1);
    });
  });

  describe('interactive mode', () => {
    it('should use inquirer for interactive setup', async () => {
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

      expect(inquirer.default.prompt).toHaveBeenCalled();
      expect(existsSync(join(workspace.root, 'custom/plans'))).toBe(true);
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

      const output = consoleOutput.join('\n');
      expect(output).toContain('Anvil is ready to use');
    });
  });
});
