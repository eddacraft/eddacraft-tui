import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { APSPlan, validateAPSPlan } from '@eddacraft/anvil-core';
import { ensureDirSync } from 'fs-extra';

export async function loadPlan(path: string): Promise<APSPlan> {
  if (!existsSync(path)) {
    throw new Error(`Plan file not found: ${path}`);
  }

  try {
    const content = readFileSync(path, 'utf-8');
    const data = JSON.parse(content);

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
      `Failed to load plan: ${error instanceof Error ? error.message : 'Unknown error'}`
    );
  }
}

export function savePlan(plan: APSPlan, path: string): void {
  ensureDirSync(resolve(path, '..'));
  writeFileSync(path, JSON.stringify(plan, null, 2), 'utf-8');
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

export function getWorkspaceRoot(): string {
  // Look for package.json or .git directory to find workspace root
  let currentDir = process.cwd();

  while (currentDir !== '/') {
    if (existsSync(join(currentDir, 'package.json')) || existsSync(join(currentDir, '.git'))) {
      return currentDir;
    }
    currentDir = resolve(currentDir, '..');
  }

  return process.cwd();
}
