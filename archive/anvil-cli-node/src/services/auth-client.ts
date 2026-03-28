import { z } from 'zod';
import { apiRequest } from './api-client.js';

const VerifyResponseSchema = z.object({
  valid: z.boolean(),
  user: z.object({ email: z.string() }).optional(),
  scopes: z.array(z.string()).optional(),
  expiresAt: z.string().optional(),
  license: z.string().optional(),
});

export type VerifyResponse = z.infer<typeof VerifyResponseSchema>;

/**
 * Verify a beta token against the API.
 */
export async function verifyToken(token: string): Promise<VerifyResponse> {
  const raw = await apiRequest<unknown>({
    method: 'POST',
    path: '/api/v1/auth/verify',
    body: { token },
    operationName: 'API request',
  });
  return VerifyResponseSchema.parse(raw);
}
