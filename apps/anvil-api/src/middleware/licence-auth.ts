import { getClient, type NeonClient } from '../db/client.js';
import { findUserById } from '../db/queries.js';
import { verifyLicence, type LicenceClaims } from '../lib/licence.js';

type LicenceAuthentication =
  | {
      status: 'active';
      claims: LicenceClaims;
      user: NonNullable<Awaited<ReturnType<typeof findUserById>>>;
      sql: NeonClient;
    }
  | { status: 'invalid' }
  | { status: 'inactive' }
  | { status: 'unavailable' }
  | { status: 'rejected'; response: Response };

/**
 * SEC-013: the API route boundary for licence credentials. A valid signature
 * alone never exposes an authenticated identity: reload the account on every
 * request and require active status. Do not cache this decision.
 *
 * Optional payload validation runs after signature verification but before DB
 * access (CIB-399). It receives no claims, may only reject/validate input, and
 * must not perform authenticated effects. Route effects follow status: active.
 */
export async function authenticateLicence(
  token: string,
  validateRequest?: () => Promise<Response | undefined>
): Promise<LicenceAuthentication> {
  let claims;
  try {
    claims = await verifyLicence(token);
  } catch {
    console.error('licence verification unavailable');
    return { status: 'unavailable' };
  }
  if (!claims) return { status: 'invalid' };

  const rejection = await validateRequest?.();
  if (rejection) return { status: 'rejected', response: rejection };

  try {
    const sql = getClient();
    const user = await findUserById(sql, claims.sub);
    if (!user || user.status !== 'active') return { status: 'inactive' };
    return { status: 'active', claims, user, sql };
  } catch {
    console.error('licence account lookup unavailable');
    return { status: 'unavailable' };
  }
}
