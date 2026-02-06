const DEFAULT_API_URL = 'https://anvil-api.vercel.app';

interface InviteRequest {
  email: string;
  name?: string;
  notes?: string;
  days?: number;
  scopes?: string[];
}

interface InviteResponse {
  token: string;
  user: { email: string; id: string };
  expiresAt: string;
  scopes: string[];
}

interface RevokeResponse {
  revoked: number;
}

interface UserResponse {
  user: {
    id: string;
    email: string;
    name: string | null;
    status: string;
    created_at: string;
  };
  tokens: Array<{
    id: string;
    scopes: string[];
    expires_at: string;
    revoked_at: string | null;
    created_at: string;
  }>;
}

function getAdminKey(): string {
  const key = process.env['ANVIL_ADMIN_KEY'];
  if (!key) {
    throw new Error('ANVIL_ADMIN_KEY environment variable is required for admin commands');
  }
  return key;
}

function getApiUrl(): string {
  return process.env['ANVIL_API_URL'] ?? DEFAULT_API_URL;
}

export async function adminInvite(request: InviteRequest): Promise<InviteResponse> {
  const url = `${getApiUrl()}/api/v1/admin/invite`;
  const res = await fetch(url, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${getAdminKey()}`,
    },
    body: JSON.stringify(request),
  });

  if (!res.ok) {
    const body = await res.text();
    throw new Error(`Admin invite failed: ${res.status} ${body}`);
  }

  return (await res.json()) as InviteResponse;
}

export async function adminRevoke(email: string): Promise<RevokeResponse> {
  const url = `${getApiUrl()}/api/v1/admin/revoke`;
  const res = await fetch(url, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${getAdminKey()}`,
    },
    body: JSON.stringify({ email }),
  });

  if (!res.ok) {
    const body = await res.text();
    throw new Error(`Admin revoke failed: ${res.status} ${body}`);
  }

  return (await res.json()) as RevokeResponse;
}

export async function adminGetUser(email: string): Promise<UserResponse> {
  const url = `${getApiUrl()}/api/v1/admin/user/${encodeURIComponent(email)}`;
  const res = await fetch(url, {
    method: 'GET',
    headers: {
      Authorization: `Bearer ${getAdminKey()}`,
    },
  });

  if (!res.ok) {
    const body = await res.text();
    throw new Error(`Admin user lookup failed: ${res.status} ${body}`);
  }

  return (await res.json()) as UserResponse;
}
