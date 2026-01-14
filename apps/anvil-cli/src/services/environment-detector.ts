import { existsSync, readFileSync } from 'fs';
import { join } from 'path';

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
      hasGit: this.detectGit(),
      hasPackageJson: this.detectPackageJson(),
      hasEslint: this.detectEslint(),
      hasPrettier: this.detectPrettier(),
      hasVitest: this.detectVitest(),
      hasJest: this.detectJest(),
      hasTypeScript: this.detectTypeScript(),
      packageManager: this.detectPackageManager(),
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

  private detectGit(): boolean {
    return existsSync(join(this.projectRoot, '.git'));
  }

  private detectPackageJson(): boolean {
    return existsSync(join(this.projectRoot, 'package.json'));
  }

  private detectEslint(): boolean {
    const eslintFiles = [
      '.eslintrc',
      '.eslintrc.js',
      '.eslintrc.cjs',
      '.eslintrc.json',
      '.eslintrc.yml',
      'eslint.config.js',
      'eslint.config.mjs',
      'eslint.config.cjs',
    ];

    // Check for config files
    if (eslintFiles.some((file) => existsSync(join(this.projectRoot, file)))) {
      return true;
    }

    // Check package.json for eslint dependency
    return this.hasPackageDependency('eslint');
  }

  private detectPrettier(): boolean {
    const prettierFiles = [
      '.prettierrc',
      '.prettierrc.js',
      '.prettierrc.cjs',
      '.prettierrc.json',
      '.prettierrc.yml',
      'prettier.config.js',
      'prettier.config.cjs',
    ];

    // Check for config files
    if (prettierFiles.some((file) => existsSync(join(this.projectRoot, file)))) {
      return true;
    }

    // Check package.json for prettier dependency
    return this.hasPackageDependency('prettier');
  }

  private detectVitest(): boolean {
    const vitestFiles = [
      'vitest.config.js',
      'vitest.config.ts',
      'vite.config.js',
      'vite.config.ts',
    ];

    // Check for config files
    if (vitestFiles.some((file) => existsSync(join(this.projectRoot, file)))) {
      return true;
    }

    // Check package.json for vitest dependency
    return this.hasPackageDependency('vitest');
  }

  private detectJest(): boolean {
    const jestFiles = ['jest.config.js', 'jest.config.ts', 'jest.config.json'];

    // Check for config files
    if (jestFiles.some((file) => existsSync(join(this.projectRoot, file)))) {
      return true;
    }

    // Check package.json for jest dependency
    return this.hasPackageDependency('jest');
  }

  private detectTypeScript(): boolean {
    return existsSync(join(this.projectRoot, 'tsconfig.json'));
  }

  private detectPackageManager(): 'npm' | 'pnpm' | 'yarn' | 'unknown' {
    if (existsSync(join(this.projectRoot, 'pnpm-lock.yaml'))) {
      return 'pnpm';
    }
    if (existsSync(join(this.projectRoot, 'yarn.lock'))) {
      return 'yarn';
    }
    if (existsSync(join(this.projectRoot, 'package-lock.json'))) {
      return 'npm';
    }
    return 'unknown';
  }

  private getProjectName(): string | undefined {
    try {
      const packageJsonPath = join(this.projectRoot, 'package.json');
      if (!existsSync(packageJsonPath)) {
        return undefined;
      }

      const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf-8'));
      return packageJson.name;
    } catch {
      return undefined;
    }
  }

  private hasPackageDependency(packageName: string): boolean {
    try {
      const packageJsonPath = join(this.projectRoot, 'package.json');
      if (!existsSync(packageJsonPath)) {
        return false;
      }

      const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf-8'));
      const deps = {
        ...packageJson.dependencies,
        ...packageJson.devDependencies,
      };

      return packageName in deps;
    } catch {
      return false;
    }
  }
}
