// apps/docs-shell/lib/bauth.ts
export type ExchangeResult =
  | { status: 'ok'; license: string }
  | { status: 'pending' }
  | { status: 'error'; reason: 'api_error' | 'auth_failed' | 'invalid_response' };

function getApiUrl(): string {
  return process.env.BAUTH_API_URL ?? 'https://api.eddacraft.ai';
}

export async function exchangeGithubCode(code: string): Promise<ExchangeResult> {
  const url = `${getApiUrl()}/api/v1/auth/github/callback`;
  let res: Response;
  try {
    res = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ code }),
    });
  } catch {
    return { status: 'error', reason: 'api_error' };
  }

  if (res.status === 403) return { status: 'pending' };
  if (!res.ok) return { status: 'error', reason: 'auth_failed' };

  let body: unknown;
  try {
    body = await res.json();
  } catch {
    return { status: 'error', reason: 'invalid_response' };
  }

  if (!body || typeof (body as { license?: unknown }).license !== 'string') {
    return { status: 'error', reason: 'invalid_response' };
  }

  return { status: 'ok', license: (body as { license: string }).license };
}
