import { existsSync } from 'node:fs';
import { join } from 'node:path';
import {
  detectEslint,
  detectPrettier,
  detectPackageManager,
  readPackageJson,
  hasPackageDependency,
  hasConfigFile,
  VITEST_CONFIG_FILES,
  JEST_CONFIG_FILES,
} from '../utils/tool-detection.js';

/**
 * Detected development tools and their configurations
 */
export interface EnvironmentInfo {
  /** Whether project is a git repository */
  hasGit: boolean;
  /** Whether package.json exists */
  hasPackageJson: boolean;
  /** Whether ESLint is configured */
  hasEslint: boolean;
  /** Whether Prettier is configured */
  hasPrettier: boolean;
  /** Whether Vitest is configured */
  hasVitest: boolean;
  /** Whether Jest is configured */
  hasJest: boolean;
  /** Whether TypeScript is configured */
  hasTypeScript: boolean;
  /** Package manager detected (npm, pnpm, yarn) */
  packageManager: 'npm' | 'pnpm' | 'yarn' | 'unknown';
  /** Project name from package.json */
  projectName?: string;
  /** Project root directory */
  projectRoot: string;
}

/**
 * Detects development environment and available tools
 */
export class EnvironmentDetector {
  constructor(private readonly projectRoot: string = process.cwd()) {}

  /**
   * Detect all available tools and configurations in the project
   */
  public detect(): EnvironmentInfo {
    return {
      hasGit: existsSync(join(this.projectRoot, '.git')),
      hasPackageJson: existsSync(join(this.projectRoot, 'package.json')),
      hasEslint: detectEslint(this.projectRoot),
      hasPrettier: detectPrettier(this.projectRoot),
      hasVitest: this.detectVitest(),
      hasJest: this.detectJest(),
      hasTypeScript: existsSync(join(this.projectRoot, 'tsconfig.json')),
      packageManager: detectPackageManager(this.projectRoot),
      projectName: this.getProjectName(),
      projectRoot: this.projectRoot,
    };
  }

  /**
   * Get recommended gate checks based on detected tools
   */
  public getRecommendedChecks(env: EnvironmentInfo): string[] {
    const checks: string[] = [];

    if (env.hasEslint) {
      checks.push('eslint');
    }

    if (env.hasVitest || env.hasJest) {
      checks.push('test');
      checks.push('coverage');
    }

    // Secret scanning is always recommended
    checks.push('secret');

    return checks;
  }

  private detectVitest(): boolean {
    if (hasConfigFile(this.projectRoot, VITEST_CONFIG_FILES)) {
      return true;
    }
    return hasPackageDependency(this.projectRoot, 'vitest');
  }

  private detectJest(): boolean {
    if (hasConfigFile(this.projectRoot, JEST_CONFIG_FILES)) {
      return true;
    }
    return hasPackageDependency(this.projectRoot, 'jest');
  }

  private getProjectName(): string | undefined {
    const packageJson = readPackageJson(this.projectRoot);
    return packageJson?.name as string | undefined;
  }
}
