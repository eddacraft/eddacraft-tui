import { apiRequest } from './api-client.js';

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
  return apiRequest<VerifyResponse>({
    method: 'POST',
    path: '/api/v1/auth/verify',
    body: { token },
    operationName: 'API request',
  });
}
