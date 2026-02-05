const DEFAULT_API_URL = 'https://anvil-api.vercel.app';

interface VerifyResponse {
  valid: boolean;
  user?: { email: string };
  scopes?: string[];
  expiresAt?: string;
}

/**
 * Verify a beta token against the API.
 */
export async function verifyToken(token: string): Promise<VerifyResponse> {
  const apiUrl = process.env['ANVIL_API_URL'] ?? DEFAULT_API_URL;
  const url = `${apiUrl}/api/v1/auth/verify`;

  const res = await fetch(url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ token }),
  });

  if (!res.ok) {
    throw new Error(`API request failed: ${res.status} ${res.statusText}`);
  }

  return (await res.json()) as VerifyResponse;
}
