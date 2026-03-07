import { InvalidArgumentError } from 'commander';

export function coercePositiveInt(value: string, optionName: string): number {
  const parsed = Number(value.trim());
  if (!Number.isInteger(parsed) || parsed < 1) {
    throw new InvalidArgumentError(`${optionName} must be a positive integer`);
  }
  return parsed;
}

export function coerceNonNegativeInt(value: string, optionName: string): number {
  const parsed = Number(value.trim());
  if (!Number.isInteger(parsed) || parsed < 0) {
    throw new InvalidArgumentError(`${optionName} must be a non-negative integer`);
  }
  return parsed;
}

export function coercePort(value: string, optionName: string): number {
  const parsed = Number(value.trim());
  if (!Number.isInteger(parsed) || parsed < 1 || parsed > 65535) {
    throw new InvalidArgumentError(`${optionName} must be an integer between 1 and 65535`);
  }
  return parsed;
}
