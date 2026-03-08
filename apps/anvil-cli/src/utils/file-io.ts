import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { join, resolve, extname } from 'node:path';
import YAML from 'yaml';
import { APSPlan, validateAPSPlan } from '@eddacraft/anvil-core';
import { ensureDirSync } from 'fs-extra';
import { debug } from './output.js';

export async function loadPlan(path: string): Promise<APSPlan> {
  if (!existsSync(path)) {
    throw new Error(`Plan file not found: ${path}`);
  }

  try {
    const ext = extname(path).toLowerCase();
    let data: unknown;
    if (ext === '.yaml' || ext === '.yml') {
      data = YAML.parse(readFileSync(path, 'utf-8'));
    } else {
      data = readJsonFileSync(path);
    }
    if (!data) {
      throw new Error('Failed to parse plan file');
    }

    // Validate the plan
    const validationResult = await validateAPSPlan(data);

    if (!validationResult.valid) {
      const errorMessages =
        validationResult.issues?.map((e) => e.message).join(', ') || 'Unknown validation error';
      throw new Error(`Invalid plan: ${errorMessages}`);
    }

    return validationResult.data as APSPlan;
  } catch (error) {
    throw new Error(
      `Failed to load plan: ${error instanceof Error ? error.message : 'Unknown error'}`,
      { cause: error }
    );
  }
}

export function savePlan(plan: APSPlan, path: string): void {
  ensureDirSync(resolve(path, '..'));
  const ext = extname(path).toLowerCase();
  const content =
    ext === '.yaml' || ext === '.yml' ? YAML.stringify(plan) : JSON.stringify(plan, null, 2);
  writeFileSync(path, content, 'utf-8');
}

/**
 * Valid plan ID pattern: aps-[8+ hex chars]
 * Strict validation prevents path traversal attacks
 */
const VALID_PLAN_ID_PATTERN = /^aps-[a-f0-9]{8,}$/i;

export function findPlanById(id: string, workspaceRoot: string): string | null {
  if (!VALID_PLAN_ID_PATTERN.test(id)) {
    return null;
  }

  const plansDir = resolve(workspaceRoot, '.anvil', 'plans');
  const planFile = resolve(plansDir, `${id}.json`);

  if (!planFile.startsWith(plansDir)) {
    return null;
  }

  if (existsSync(planFile)) {
    return planFile;
  }

  return null;
}

export function ensureDirectory(path: string): void {
  ensureDirSync(path);
}

/**
 * Read and parse a JSON file synchronously.
 * Returns null if the file doesn't exist or can't be parsed.
 */
export function readJsonFileSync<T = unknown>(filePath: string): T | null {
  try {
    if (!existsSync(filePath)) {
      return null;
    }
    return JSON.parse(readFileSync(filePath, 'utf-8')) as T;
  } catch {
    debug(`readJsonFileSync: failed to parse ${filePath}, returning null`);
    return null;
  }
}

/** Tracks whether the workspace root warning has already been emitted this process */
let workspaceRootWarningEmitted = false;

export function getWorkspaceRoot(): string {
  // Walk up looking for repo-root markers first (.git, nx.json, pnpm-workspace.yaml),
  // then fall back to first package.json if no repo root found.
  const REPO_ROOT_MARKERS = ['.git', 'nx.json', 'pnpm-workspace.yaml'];

  let currentDir = process.cwd();
  let previousDir = '';
  let firstPackageJsonDir: string | null = null;

  while (currentDir !== previousDir) {
    if (!firstPackageJsonDir && existsSync(join(currentDir, 'package.json'))) {
      firstPackageJsonDir = currentDir;
    }
    for (const marker of REPO_ROOT_MARKERS) {
      if (existsSync(join(currentDir, marker))) {
        return currentDir;
      }
    }
    previousDir = currentDir;
    currentDir = resolve(currentDir, '..');
  }

  // No repo-root marker found; use first package.json if we saw one
  if (firstPackageJsonDir) {
    debug(
      `getWorkspaceRoot: no repo-root marker found, falling back to package.json at ${firstPackageJsonDir}`
    );
    return firstPackageJsonDir;
  }

  // No workspace root found — warn once, then fall back to cwd
  if (!workspaceRootWarningEmitted) {
    workspaceRootWarningEmitted = true;
    process.stderr.write(
      'Warning: No workspace root found (no .git, nx.json, pnpm-workspace.yaml, or package.json in parent chain).\n' +
        '  Falling back to current directory. Run from a project directory or run `anvil init`.\n'
    );
  }

  return process.cwd();
}
