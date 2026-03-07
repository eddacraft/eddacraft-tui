import { CliError } from './cli-error.js';

export function coercePositiveInt(value: string, optionName: string): number {
  const parsed = Number.parseInt(value, 10);
  if (Number.isNaN(parsed) || parsed < 1) {
    throw new CliError(`${optionName} must be a positive integer`);
  }
  return parsed;
}

export function coerceNonNegativeInt(value: string, optionName: string): number {
  const parsed = Number.parseInt(value, 10);
  if (Number.isNaN(parsed) || parsed < 0) {
    throw new CliError(`${optionName} must be a non-negative integer`);
  }
  return parsed;
}

export function coercePort(value: string, optionName: string): number {
  const parsed = Number.parseInt(value, 10);
  if (!Number.isFinite(parsed) || parsed < 1 || parsed > 65535) {
    throw new CliError(`${optionName} must be an integer between 1 and 65535`);
  }
  return parsed;
}
