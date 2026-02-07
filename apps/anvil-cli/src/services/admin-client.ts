import { apiRequest, getAdminKey } from './api-client.js';

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

export async function adminInvite(request: InviteRequest): Promise<InviteResponse> {
  return apiRequest<InviteResponse>({
    method: 'POST',
    path: '/api/v1/admin/invite',
    body: request,
    token: getAdminKey(),
    operationName: 'Admin invite',
  });
}

export async function adminRevoke(email: string): Promise<RevokeResponse> {
  return apiRequest<RevokeResponse>({
    method: 'POST',
    path: '/api/v1/admin/revoke',
    body: { email },
    token: getAdminKey(),
    operationName: 'Admin revoke',
  });
}

export async function adminGetUser(email: string): Promise<UserResponse> {
  return apiRequest<UserResponse>({
    method: 'GET',
    path: `/api/v1/admin/user/${encodeURIComponent(email)}`,
    token: getAdminKey(),
    operationName: 'Admin user lookup',
  });
}
