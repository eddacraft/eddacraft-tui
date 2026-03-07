import { InvalidArgumentError } from 'commander';

function toInt(value: string): number {
  const trimmed = value.trim();
  if (trimmed === '') return NaN;
  return Number(trimmed);
}

export function coercePositiveInt(value: string, optionName: string): number {
  const parsed = toInt(value);
  if (!Number.isInteger(parsed) || parsed < 1) {
    throw new InvalidArgumentError(`${optionName} must be a positive integer`);
  }
  return parsed;
}

export function coerceNonNegativeInt(value: string, optionName: string): number {
  const parsed = toInt(value);
  if (!Number.isInteger(parsed) || parsed < 0) {
    throw new InvalidArgumentError(`${optionName} must be a non-negative integer`);
  }
  return parsed;
}

export function coercePort(value: string, optionName: string): number {
  const parsed = toInt(value);
  if (!Number.isInteger(parsed) || parsed < 1 || parsed > 65535) {
    throw new InvalidArgumentError(`${optionName} must be an integer between 1 and 65535`);
  }
  return parsed;
}
