import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';
import {
  detectEslint,
  detectPrettier,
  detectPackageManager,
  readPackageJson,
  hasPackageDependency,
} from '../utils/tool-detection.js';

/**
 * Package.json shape (extends the generic record from tool-detection)
 */
interface PackageJson {
  type?: string;
  engines?: Record<string, string>;
  dependencies?: Record<string, string>;
  devDependencies?: Record<string, string>;
  workspaces?: string[] | { packages?: string[] };
}

/**
 * Detected framework type
 */
export type FrameworkType =
  | 'nextjs'
  | 'react'
  | 'vue'
  | 'angular'
  | 'svelte'
  | 'express'
  | 'nestjs'
  | 'nx'
  | 'node'
  | 'unknown';

/**
 * Detected monorepo structure
 */
export type MonorepoType =
  | 'nx'
  | 'lerna'
  | 'pnpm-workspace'
  | 'yarn-workspace'
  | 'npm-workspace'
  | 'turborepo'
  | 'none';

/**
 * TypeScript configuration strictness level
 */
export type TypeScriptStrictness = 'strict' | 'moderate' | 'loose' | 'none';

/**
 * Project size category
 */
export type ProjectSize = 'small' | 'medium' | 'large' | 'xlarge';

/**
 * Comprehensive project characteristics
 */
export interface ProjectContext {
  /** Detected framework */
  framework: FrameworkType;
  /** Monorepo structure if any */
  monorepo: MonorepoType;
  /** TypeScript strictness configuration */
  tsStrictness: TypeScriptStrictness;
  /** Project size based on file count */
  size: ProjectSize;
  /** Approximate number of source files */
  fileCount: number;
  /** ESLint configuration exists */
  hasEslint: boolean;
  /** Prettier configuration exists */
  hasPrettier: boolean;
  /** Test framework detected */
  hasTests: boolean;
  /** Package manager */
  packageManager: 'npm' | 'pnpm' | 'yarn' | 'unknown';
  /** Project root directory */
  projectRoot: string;
  /** Workspace packages (for monorepos) */
  workspacePackages: string[];
}

/**
 * Detects comprehensive project characteristics for smart defaults generation
 */
export class ProjectDetector {
  constructor(private readonly projectRoot: string = process.cwd()) {}

  /**
   * Detect all project characteristics
   */
  public detect(): ProjectContext {
    const framework = this.detectFramework();
    const monorepo = this.detectMonorepo();
    const packageManager = detectPackageManager(this.projectRoot);
    const fileCount = this.estimateFileCount();
    const workspacePackages = this.detectWorkspacePackages();

    return {
      framework,
      monorepo,
      tsStrictness: this.detectTypeScriptStrictness(),
      size: this.categorizeProjectSize(fileCount),
      fileCount,
      hasEslint: detectEslint(this.projectRoot),
      hasPrettier: detectPrettier(this.projectRoot),
      hasTests: this.detectTestFramework(),
      packageManager,
      projectRoot: this.projectRoot,
      workspacePackages,
    };
  }

  /**
   * Detect primary framework or runtime
   */
  private detectFramework(): FrameworkType {
    const packageJson = readPackageJson(this.projectRoot) as PackageJson | null;
    if (!packageJson) {
      return 'unknown';
    }

    const deps = {
      ...packageJson.dependencies,
      ...packageJson.devDependencies,
    };

    // Check for specific frameworks (order matters - more specific first)
    if ('next' in deps) return 'nextjs';
    if ('@nx/workspace' in deps || 'nx' in deps) return 'nx';
    if ('@nestjs/core' in deps || '@nestjs/common' in deps) return 'nestjs';
    if ('express' in deps && !('react' in deps)) return 'express';
    if ('vue' in deps) return 'vue';
    if ('@angular/core' in deps) return 'angular';
    if ('svelte' in deps) return 'svelte';
    if ('react' in deps) return 'react';

    // Check for Node.js project
    if (packageJson.type === 'module' || 'node' in (packageJson.engines || {})) {
      return 'node';
    }

    return 'unknown';
  }

  /**
   * Detect monorepo configuration
   */
  private detectMonorepo(): MonorepoType {
    // Check for Nx
    if (existsSync(join(this.projectRoot, 'nx.json'))) {
      return 'nx';
    }

    // Check for Turborepo
    if (existsSync(join(this.projectRoot, 'turbo.json'))) {
      return 'turborepo';
    }

    // Check for Lerna
    if (existsSync(join(this.projectRoot, 'lerna.json'))) {
      return 'lerna';
    }

    // Check for pnpm workspace
    if (existsSync(join(this.projectRoot, 'pnpm-workspace.yaml'))) {
      return 'pnpm-workspace';
    }

    // Check for yarn workspace
    const packageJson = readPackageJson(this.projectRoot) as PackageJson | null;
    if (packageJson?.workspaces) {
      if (existsSync(join(this.projectRoot, 'yarn.lock'))) {
        return 'yarn-workspace';
      }
      if (existsSync(join(this.projectRoot, 'package-lock.json'))) {
        return 'npm-workspace';
      }
    }

    return 'none';
  }

  /**
   * Detect TypeScript strictness level
   */
  private detectTypeScriptStrictness(): TypeScriptStrictness {
    try {
      const tsconfigPath = join(this.projectRoot, 'tsconfig.json');
      if (!existsSync(tsconfigPath)) {
        return 'none';
      }

      const tsconfig = JSON.parse(readFileSync(tsconfigPath, 'utf-8'));
      const compilerOptions = tsconfig.compilerOptions || {};

      // Check for strict mode
      if (compilerOptions.strict === true) {
        return 'strict';
      }

      // Count strict-related flags
      const strictFlags = [
        'strictNullChecks',
        'strictFunctionTypes',
        'strictBindCallApply',
        'strictPropertyInitialization',
        'noImplicitAny',
        'noImplicitThis',
      ];

      const enabledStrictFlags = strictFlags.filter(
        (flag) => compilerOptions[flag] === true
      ).length;

      if (enabledStrictFlags >= 4) {
        return 'strict';
      } else if (enabledStrictFlags >= 2) {
        return 'moderate';
      } else {
        return 'loose';
      }
    } catch {
      return 'none';
    }
  }

  /**
   * Estimate source file count
   */
  private estimateFileCount(): number {
    try {
      const srcDirs = ['src', 'lib', 'app', 'apps', 'pages', 'components', 'packages'];
      let totalCount = 0;

      for (const dir of srcDirs) {
        const dirPath = join(this.projectRoot, dir);
        if (existsSync(dirPath)) {
          totalCount += this.countFilesRecursive(dirPath);
        }
      }

      return totalCount;
    } catch {
      return 0;
    }
  }

  /**
   * Count files recursively in a directory
   */
  private countFilesRecursive(dirPath: string, maxDepth = 10): number {
    if (maxDepth <= 0) return 0;

    try {
      const entries = readdirSync(dirPath);
      let count = 0;

      for (const entry of entries) {
        // Skip node_modules, .git, dist, build
        if (['node_modules', '.git', 'dist', 'build', '.next', 'coverage'].includes(entry)) {
          continue;
        }

        const fullPath = join(dirPath, entry);
        const stat = statSync(fullPath);

        if (stat.isDirectory()) {
          count += this.countFilesRecursive(fullPath, maxDepth - 1);
        } else if (stat.isFile() && this.isSourceFile(entry)) {
          count++;
        }
      }

      return count;
    } catch {
      return 0;
    }
  }

  /**
   * Check if a file is a source file
   */
  private isSourceFile(filename: string): boolean {
    const sourceExtensions = ['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs', '.vue', '.svelte'];
    return sourceExtensions.some((ext) => filename.endsWith(ext));
  }

  /**
   * Categorize project size based on file count
   */
  private categorizeProjectSize(fileCount: number): ProjectSize {
    if (fileCount < 50) return 'small';
    if (fileCount < 200) return 'medium';
    if (fileCount < 1000) return 'large';
    return 'xlarge';
  }

  /**
   * Detect test framework
   */
  private detectTestFramework(): boolean {
    const testPackages = ['vitest', 'jest', 'mocha', 'jasmine', '@playwright/test'];
    return testPackages.some((pkg) => hasPackageDependency(this.projectRoot, pkg));
  }

  /**
   * Detect workspace packages
   */
  private detectWorkspacePackages(): string[] {
    // Check for workspaces in package.json
    const packageJson = readPackageJson(this.projectRoot) as PackageJson | null;
    if (packageJson) {
      if (Array.isArray(packageJson.workspaces)) {
        return packageJson.workspaces;
      } else if (packageJson.workspaces?.packages) {
        return packageJson.workspaces.packages;
      }
    }

    // Check for pnpm-workspace.yaml
    try {
      const pnpmWorkspacePath = join(this.projectRoot, 'pnpm-workspace.yaml');
      if (existsSync(pnpmWorkspacePath)) {
        const content = readFileSync(pnpmWorkspacePath, 'utf-8');
        // Simple YAML parsing for packages array
        const match = content.match(/packages:\s*\n((?:\s+-\s+[^\n]+\n?)+)/);
        if (match) {
          return match[1]
            .split('\n')
            .map((line) => line.trim().replace(/^-\s+['"]?(.+?)['"]?$/, '$1'))
            .filter(Boolean);
        }
      }
    } catch {
      // Ignore parsing errors
    }

    // Check for lerna.json
    try {
      const lernaPath = join(this.projectRoot, 'lerna.json');
      if (existsSync(lernaPath)) {
        const lerna = JSON.parse(readFileSync(lernaPath, 'utf-8'));
        if (Array.isArray(lerna.packages)) {
          return lerna.packages;
        }
      }
    } catch {
      // Ignore parsing errors
    }

    return [];
  }
}
