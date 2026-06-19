import type { ZodType } from 'zod';
import type { AdminConfig } from './config.js';

export class AdminError extends Error {
  readonly exitCode: number;
  readonly status?: number;
  readonly body?: string;

  constructor(message: string, exitCode: number, status?: number, body?: string) {
    super(message);
    this.name = 'AdminError';
    this.exitCode = exitCode;
    this.status = status;
    this.body = body;
  }
}

type Fetch = typeof fetch;

export interface ClientOptions extends AdminConfig {
  fetchImpl?: Fetch;
}

export interface RequestOptions<T = unknown> {
  query?: Record<string, string | number | boolean | undefined>;
  body?: unknown;
  method?: 'GET' | 'POST';
  schema?: ZodType<T>;
}

// Structural type shared by every command that mutates state (approve,
// revoke, invite, send-migration). Hoisted here to prevent the drift that
// happens when each command re-declares its own near-identical shape.
// The read-only commands (list, show, audit) have the same duplication for
// an `AdminReader` shape; that hoist is deliberately left out of #949 and
// tracked separately.
export interface AdminWriter {
  post<T>(path: string, body?: unknown, schema?: ZodType<T>): Promise<T>;
}

export class AdminClient {
  private readonly url: string;
  private readonly key: string;
  private readonly actor: string;
  private readonly fetchImpl: Fetch;

  constructor(opts: ClientOptions) {
    this.url = opts.url.replace(/\/+$/, '');
    this.key = opts.key;
    this.actor = opts.actor;
    this.fetchImpl = opts.fetchImpl ?? fetch;
  }

  async get<T>(path: string, query?: RequestOptions<T>['query'], schema?: ZodType<T>): Promise<T> {
    return this.request<T>(path, { method: 'GET', query, schema });
  }

  async post<T>(path: string, body?: unknown, schema?: ZodType<T>): Promise<T> {
    return this.request<T>(path, { method: 'POST', body, schema });
  }

  private async request<T>(path: string, opts: RequestOptions<T>): Promise<T> {
    const method = opts.method ?? 'GET';
    const target = new URL(path.startsWith('/') ? path : `/${path}`, this.url + '/');
    if (opts.query) {
      for (const [k, v] of Object.entries(opts.query)) {
        if (v === undefined) continue;
        target.searchParams.set(k, String(v));
      }
    }

    const headers: Record<string, string> = {
      Authorization: `Bearer ${this.key}`,
      'X-Admin-Actor': this.actor,
      Accept: 'application/json',
    };

    const init: RequestInit = { method, headers };
    if (method === 'POST') {
      headers['Content-Type'] = 'application/json';
      init.body = opts.body !== undefined ? JSON.stringify(opts.body) : '{}';
    }

    let response: Response;
    try {
      response = await this.fetchImpl(target, init);
    } catch (err) {
      const cause = err instanceof Error ? err.message : String(err);
      throw new AdminError(`cannot reach ${this.url}: ${cause}`, 3);
    }

    const text = await response.text().catch(() => '');

    if (response.status >= 500) {
      const truncated = text.length > 500 ? `${text.slice(0, 500)}…` : text;
      throw new AdminError(
        `server error ${response.status}: ${truncated}`,
        2,
        response.status,
        text
      );
    }

    if (response.status >= 400) {
      let message = `request failed: ${response.status}`;
      try {
        const parsed = text ? (JSON.parse(text) as { error?: unknown }) : null;
        if (parsed && typeof parsed.error === 'string') {
          message = parsed.error;
        }
      } catch {
        /* not JSON — keep default */
      }
      throw new AdminError(message, 1, response.status, text);
    }

    if (!text) {
      if (opts.schema) {
        throw new AdminError(
          `empty response body (expected ${response.status} with JSON content)`,
          6,
          response.status,
          text
        );
      }
      return undefined as T;
    }

    let parsed: unknown;
    try {
      parsed = JSON.parse(text);
    } catch {
      throw new AdminError(
        `invalid JSON response: ${text.slice(0, 200)}`,
        2,
        response.status,
        text
      );
    }

    if (!opts.schema) return parsed as T;

    const result = opts.schema.safeParse(parsed);
    if (!result.success) {
      // Pick the first issue as the headline — most real malformed
      // responses trip one field and chaining every issue makes the
      // error line unusably long. `err.body` carries the raw response
      // text for anyone debugging manually (not the Zod issue tree).
      const issue = result.error.issues[0];
      const fieldPath = issue?.path.length ? issue.path.join('.') : '<root>';
      const reason = issue?.message ?? 'validation failed';
      throw new AdminError(
        `response validation failed at ${fieldPath}: ${reason}`,
        6,
        response.status,
        text
      );
    }
    return result.data;
  }
}
