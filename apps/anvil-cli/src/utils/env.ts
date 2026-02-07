/**
 * Environment variable access helpers.
 *
 * Consolidates the repeated patterns for reading, validating, and
 * providing defaults for environment variables.
 */

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
