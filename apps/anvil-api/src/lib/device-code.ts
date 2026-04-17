import { randomBytes } from 'node:crypto';

export const MAX_USER_CODE_RETRIES = 3;
export const USER_CODE_CONSTRAINT = 'device_codes_user_code_key';

export function generateUserCode(): string {
  return 'ANVIL-' + randomBytes(4).toString('hex').toUpperCase();
}

export function isUniqueViolation(err: unknown): boolean {
  return err instanceof Error && 'code' in err && (err as { code: string }).code === '23505';
}

export function isUserCodeCollision(err: unknown): boolean {
  if (!isUniqueViolation(err) || typeof err !== 'object' || err === null) return false;
  const constraint =
    'constraint' in err && typeof err.constraint === 'string' ? err.constraint : undefined;
  if (constraint === USER_CODE_CONSTRAINT) return true;
  const detail = 'detail' in err && typeof err.detail === 'string' ? err.detail : undefined;
  return detail?.includes('(user_code)') ?? false;
}

export async function withUserCodeRetry<T>(
  attempt: (userCode: string) => Promise<T>,
  maxRetries: number = MAX_USER_CODE_RETRIES
): Promise<T> {
  if (maxRetries < 1) {
    throw new RangeError('maxRetries must be at least 1');
  }
  for (let i = 0; i < maxRetries; i++) {
    const code = generateUserCode();
    try {
      return await attempt(code);
    } catch (err) {
      if (isUserCodeCollision(err) && i < maxRetries - 1) continue;
      throw err;
    }
  }
  throw new Error('unreachable');
}
