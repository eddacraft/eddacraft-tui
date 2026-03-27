/**
 * Shared tool and configuration detection utilities.
 *
 * Used by both EnvironmentDetector and ProjectDetector to avoid duplicating
 * detection logic for ESLint, Prettier, package managers, etc.
 */

import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { readJsonFileSync } from './file-io.js';

/**
 * Config file lists for common tools
 */
export const ESLINT_CONFIG_FILES = [
  '.eslintrc',
  '.eslintrc.js',
  '.eslintrc.cjs',
  '.eslintrc.json',
  '.eslintrc.yml',
  'eslint.config.js',
  'eslint.config.mjs',
  'eslint.config.cjs',
] as const;

export const PRETTIER_CONFIG_FILES = [
  '.prettierrc',
  '.prettierrc.js',
  '.prettierrc.cjs',
  '.prettierrc.json',
  '.prettierrc.yml',
  'prettier.config.js',
  'prettier.config.cjs',
] as const;

export const VITEST_CONFIG_FILES = [
  'vitest.config.js',
  'vitest.config.ts',
  'vite.config.js',
  'vite.config.ts',
] as const;

export const JEST_CONFIG_FILES = ['jest.config.js', 'jest.config.ts', 'jest.config.json'] as const;

/**
 * Read and parse package.json from a project root.
 */
export function readPackageJson(projectRoot: string): Record<string, unknown> | null {
  return readJsonFileSync(join(projectRoot, 'package.json'));
}

/**
 * Check if a package is in dependencies or devDependencies.
 */
export function hasPackageDependency(projectRoot: string, packageName: string): boolean {
  const packageJson = readPackageJson(projectRoot);
  if (!packageJson) {
    return false;
  }

  const deps = {
    ...(packageJson.dependencies as Record<string, string> | undefined),
    ...(packageJson.devDependencies as Record<string, string> | undefined),
  };

  return packageName in deps;
}

/**
 * Check if any of the given config files exist in the project root.
 */
export function hasConfigFile(projectRoot: string, configFiles: readonly string[]): boolean {
  return configFiles.some((file) => existsSync(join(projectRoot, file)));
}

/**
 * Detect if ESLint is configured in the project.
 */
export function detectEslint(projectRoot: string): boolean {
  if (hasConfigFile(projectRoot, ESLINT_CONFIG_FILES)) {
    return true;
  }
  return hasPackageDependency(projectRoot, 'eslint');
}

/**
 * Detect if Prettier is configured in the project.
 */
export function detectPrettier(projectRoot: string): boolean {
  if (hasConfigFile(projectRoot, PRETTIER_CONFIG_FILES)) {
    return true;
  }
  return hasPackageDependency(projectRoot, 'prettier');
}

/**
 * Detect the package manager used in the project.
 */
export function detectPackageManager(projectRoot: string): 'npm' | 'pnpm' | 'yarn' | 'unknown' {
  if (existsSync(join(projectRoot, 'pnpm-lock.yaml'))) {
    return 'pnpm';
  }
  if (existsSync(join(projectRoot, 'yarn.lock'))) {
    return 'yarn';
  }
  if (existsSync(join(projectRoot, 'package-lock.json'))) {
    return 'npm';
  }
  return 'unknown';
}
