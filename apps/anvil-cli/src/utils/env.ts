/**
 * Environment variable access helpers.
 *
 * Consolidates the repeated patterns for reading, validating, and
 * providing defaults for environment variables.
 */

import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { homedir } from 'node:os';
import { debug } from './output.js';

/**
 * Load environment variables from ~/.anvil/.env if it exists.
 * Does not override variables already set in the environment.
 *
 * Supports simple KEY=VALUE syntax only:
 * - Lines starting with # are comments
 * - Surrounding single/double quotes are stripped from values
 * - Multi-line values, escape sequences, variable interpolation,
 *   and `export` prefixes are NOT supported
 */
export function loadAnvilEnv(): void {
  const envPath = join(homedir(), '.anvil', '.env');
  let content: string;
  try {
    content = readFileSync(envPath, 'utf-8');
  } catch {
    debug(`loadAnvilEnv: ${envPath} not found, skipping`);
    return;
  }

  for (const line of content.split('\n')) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const eqIndex = trimmed.indexOf('=');
    if (eqIndex === -1) continue;
    const key = trimmed.slice(0, eqIndex).trim();
    let value = trimmed.slice(eqIndex + 1).trim();
    // Strip surrounding quotes
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }
    if (!process.env[key]) {
      process.env[key] = value;
    }
  }
}

/**
 * Get a required environment variable, throwing if it's not set.
 */
export function requireEnv(name: string, context?: string): string {
  const value = process.env[name];
  if (!value) {
    const suffix = context ? ` for ${context}` : '';
    throw new Error(`${name} environment variable is required${suffix}`);
  }
  return value;
}

/**
 * Get an environment variable with a default value.
 */
export function getEnv(name: string, defaultValue: string): string {
  return process.env[name] ?? defaultValue;
}

/**
 * Check if a boolean-style environment variable is truthy.
 * Recognises '1' and 'true' (case-insensitive).
 */
export function isEnvTrue(name: string): boolean {
  const value = process.env[name];
  return value === '1' || value?.toLowerCase() === 'true';
}
