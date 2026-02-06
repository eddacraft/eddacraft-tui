/**
 * API Test Client
 *
 * Wraps the Hono app with ergonomic helpers for E2E-testing
 * the Anvil API routes. Uses Hono's built-in request() method
 * so no real HTTP server is started — fast and deterministic.
 *
 * Database calls are stubbed at the module boundary so the tests
 * exercise the full route → middleware → handler → serialisation
 * pipeline without requiring a live Postgres instance.
 */

export interface ApiResponse<T = unknown> {
  status: number;
  body: T;
  headers: Headers;
}

export interface ApiClientOptions {
  /** Base path prefix (default: '/api/v1') */
  basePath?: string;
  /** Default headers sent with every request */
  defaultHeaders?: Record<string, string>;
}

/**
 * Create a lightweight test client around a Hono app instance.
 *
 * @example
 * ```ts
 * import app from '@eddacraft/anvil-api';
 * const client = createApiClient(app);
 *
 * const res = await client.get('/health');
 * expect(res.status).toBe(200);
 * expect(res.body).toHaveProperty('status', 'ok');
 * ```
 */
export function createApiClient(
  app: { request: (input: Request | string, init?: RequestInit) => Promise<Response> },
  options: ApiClientOptions = {}
) {
  const { basePath = '', defaultHeaders = {} } = options;

  async function request<T = unknown>(
    method: string,
    path: string,
    init: { body?: unknown; headers?: Record<string, string> } = {}
  ): Promise<ApiResponse<T>> {
    const url = `http://localhost${basePath}${path}`;
    const headers: Record<string, string> = {
      ...defaultHeaders,
      ...init.headers,
    };

    const fetchInit: RequestInit = { method, headers };

    if (init.body !== undefined) {
      headers['content-type'] = 'application/json';
      fetchInit.body = JSON.stringify(init.body);
    }

    fetchInit.headers = headers;

    const res = await app.request(new Request(url, fetchInit));
    let body: T;
    try {
      body = (await res.json()) as T;
    } catch {
      body = (await res.text()) as unknown as T;
    }

    return { status: res.status, body, headers: res.headers };
  }

  return {
    get: <T = unknown>(path: string, headers?: Record<string, string>) =>
      request<T>('GET', path, { headers }),

    post: <T = unknown>(path: string, body?: unknown, headers?: Record<string, string>) =>
      request<T>('POST', path, { body, headers }),

    put: <T = unknown>(path: string, body?: unknown, headers?: Record<string, string>) =>
      request<T>('PUT', path, { body, headers }),

    delete: <T = unknown>(path: string, headers?: Record<string, string>) =>
      request<T>('DELETE', path, { headers }),

    /** Raw request for edge cases */
    request,
  };
}
