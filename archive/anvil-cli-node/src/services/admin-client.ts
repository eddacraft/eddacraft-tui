import { z } from 'zod';
import { apiRequest, getAdminKey } from './api-client.js';

interface InviteRequest {
  email: string;
  name?: string;
  notes?: string;
  days?: number;
  scopes?: string[];
}

const InviteResponseSchema = z.object({
  token: z.string(),
  user: z.object({ email: z.string(), id: z.string() }),
  expiresAt: z.string(),
  scopes: z.array(z.string()),
});

const RevokeResponseSchema = z.object({
  revoked: z.number(),
});

const UserResponseSchema = z.object({
  user: z.object({
    id: z.string(),
    email: z.string(),
    name: z.string().nullable(),
    status: z.string(),
    created_at: z.string(),
  }),
  tokens: z.array(
    z.object({
      id: z.string(),
      scopes: z.array(z.string()),
      expires_at: z.string(),
      revoked_at: z.string().nullable(),
      created_at: z.string(),
    })
  ),
});

export type InviteResponse = z.infer<typeof InviteResponseSchema>;
export type RevokeResponse = z.infer<typeof RevokeResponseSchema>;
export type UserResponse = z.infer<typeof UserResponseSchema>;

export async function adminInvite(request: InviteRequest): Promise<InviteResponse> {
  const raw = await apiRequest<unknown>({
    method: 'POST',
    path: '/api/v1/admin/invite',
    body: request,
    token: getAdminKey(),
    operationName: 'Admin invite',
  });
  return InviteResponseSchema.parse(raw);
}

export async function adminRevoke(email: string): Promise<RevokeResponse> {
  const raw = await apiRequest<unknown>({
    method: 'POST',
    path: '/api/v1/admin/revoke',
    body: { email },
    token: getAdminKey(),
    operationName: 'Admin revoke',
  });
  return RevokeResponseSchema.parse(raw);
}

const ApproveResponseSchema = z.object({
  approved: z.array(
    z.object({
      email: z.string(),
      expiresAt: z.string(),
    })
  ),
});

export type ApproveResponse = z.infer<typeof ApproveResponseSchema>;

export async function adminApprove(
  params: { email: string } | { batch: number }
): Promise<ApproveResponse> {
  const raw = await apiRequest<unknown>({
    method: 'POST',
    path: '/api/v1/admin/approve',
    body: params,
    token: getAdminKey(),
    operationName: 'Admin approve',
  });
  return ApproveResponseSchema.parse(raw);
}

export async function adminGetUser(email: string): Promise<UserResponse> {
  const raw = await apiRequest<unknown>({
    method: 'GET',
    path: `/api/v1/admin/user/${encodeURIComponent(email)}`,
    token: getAdminKey(),
    operationName: 'Admin user lookup',
  });
  return UserResponseSchema.parse(raw);
}
