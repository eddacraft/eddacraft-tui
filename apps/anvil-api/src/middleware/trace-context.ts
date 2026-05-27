import {
  TraceparentParseError,
  parseTraceparent,
  type TraceContext,
} from '@eddacraft/anvil-observability';
import type { Context, Next } from 'hono';

export const TRACE_CONTEXT_VAR = 'traceContext';
export const TRACE_RESPONSE_HEADER = 'X-Anvil-Traceparent';

export function getTraceContext(c: Context): TraceContext | undefined {
  return c.get(TRACE_CONTEXT_VAR);
}

export async function traceContext(c: Context, next: Next): Promise<Response | void> {
  const header = c.req.header('traceparent');
  if (header === undefined) {
    return next();
  }

  try {
    const context = parseTraceparent(header);
    c.set(TRACE_CONTEXT_VAR, context);
    await next();
    c.res.headers.set(TRACE_RESPONSE_HEADER, context.header);
    return undefined;
  } catch (error) {
    if (error instanceof TraceparentParseError) {
      return c.json(
        {
          error: 'Invalid traceparent',
          code: error.code,
        },
        400
      );
    }
    throw error;
  }
}
