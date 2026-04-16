import { randomBytes } from 'node:crypto';

export const MAX_USER_CODE_RETRIES = 3;

export function generateUserCode(): string {
  return 'ANVIL-' + randomBytes(4).toString('hex').toUpperCase();
}

export function isUniqueViolation(err: unknown): boolean {
  return err instanceof Error && 'code' in err && (err as { code: string }).code === '23505';
}

export async function withUserCodeRetry<T>(
  attempt: (userCode: string) => Promise<T>,
  maxRetries: number = MAX_USER_CODE_RETRIES
): Promise<T> {
  let lastErr: unknown;
  for (let i = 0; i < maxRetries; i++) {
    const code = generateUserCode();
    try {
      return await attempt(code);
    } catch (err) {
      if (isUniqueViolation(err) && i < maxRetries - 1) {
        lastErr = err;
        continue;
      }
      throw err;
    }
  }
  throw lastErr;
}
