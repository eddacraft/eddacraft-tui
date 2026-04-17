import { InvalidArgumentError } from 'commander';

export function parseBoundedInt(flag: string, min: number, max: number) {
  return (value: string): number => {
    if (!/^-?\d+$/.test(value)) {
      throw new InvalidArgumentError(`${flag} must be an integer`);
    }
    const n = Number(value);
    if (!Number.isInteger(n) || n < min || n > max) {
      throw new InvalidArgumentError(`${flag} must be between ${min} and ${max}`);
    }
    return n;
  };
}
