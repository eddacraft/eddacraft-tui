/**
 * JSON-RPC 2.0 framing helpers.
 *
 * The daemon's wire contract pinned in
 * `crates/anvil-intercept/src/ipc.rs` accepts:
 *   - JSON-RPC 2.0 envelopes: `{"jsonrpc": "2.0", "method": ..., "id": ..., "params": ...}`
 *   - Legacy NDJSON envelopes: `{"command": "...", "session_id": ..., ...}` (no `jsonrpc` field)
 *
 * The driver client only emits JSON-RPC 2.0 envelopes. Legacy
 * envelopes are out-of-scope for this consumer — the daemon retains
 * them for the launcher.
 *
 * Per `2026-04-26-diagnostic-envelope-coordination.md` §3, the
 * JSON-RPC envelope is pinned by JSON-RPC 2.0 conformance and not
 * re-litigated here; this module only handles encode / decode of the
 * envelope shape so the rest of the client deals in typed result /
 * error / notification values.
 */

import { driverError, mapDaemonErrorRetriable, type DriverClientError } from '../errors.js';

/** Valid JSON-RPC id types per the spec. We do not allow `null` ids on
 *  outgoing requests (the daemon treats `id: null` as a notification
 *  per the `explicit_null_id_deserialises_as_notification` test); the
 *  `string | number` union here keeps the request path safe. */
export type JsonRpcId = string | number;

export interface JsonRpcRequest {
  jsonrpc: '2.0';
  id: JsonRpcId;
  method: string;
  params?: unknown;
}

export interface JsonRpcNotification {
  jsonrpc: '2.0';
  method: string;
  params?: unknown;
}

export interface JsonRpcSuccessResponse {
  jsonrpc: '2.0';
  id: JsonRpcId | null;
  result: unknown;
}

export interface JsonRpcErrorResponse {
  jsonrpc: '2.0';
  id: JsonRpcId | null;
  error: {
    code: number;
    message: string;
    data?: unknown;
  };
}

export type JsonRpcResponse = JsonRpcSuccessResponse | JsonRpcErrorResponse;

/**
 * Build a JSON-RPC request envelope. Params are passed through
 * verbatim; encoding to JSON happens at the transport layer so this
 * function is allocation-free in the hot path.
 */
export function buildRequest(id: JsonRpcId, method: string, params?: unknown): JsonRpcRequest {
  if (params === undefined) {
    return { jsonrpc: '2.0', id, method };
  }
  return { jsonrpc: '2.0', id, method, params };
}

/**
 * Build a JSON-RPC notification envelope (no `id`). Used for
 * subscribe-and-forget patterns; the daemon does not respond to
 * notifications per JSON-RPC 2.0.
 */
export function buildNotification(method: string, params?: unknown): JsonRpcNotification {
  if (params === undefined) {
    return { jsonrpc: '2.0', method };
  }
  return { jsonrpc: '2.0', method, params };
}

/**
 * Inspect a parsed JSON value and decide whether it's a valid JSON-RPC
 * 2.0 response. Returns `null` for shapes that aren't responses
 * (notifications, malformed payloads) so the caller can route
 * accordingly.
 */
export function classifyIncoming(
  value: unknown
):
  | { kind: 'response'; response: JsonRpcResponse }
  | { kind: 'notification'; method: string; params: unknown }
  | { kind: 'unknown' } {
  if (typeof value !== 'object' || value === null) {
    return { kind: 'unknown' };
  }
  const obj = value as Record<string, unknown>;
  if (obj.jsonrpc !== '2.0') {
    return { kind: 'unknown' };
  }

  const hasId = Object.prototype.hasOwnProperty.call(obj, 'id');
  if (
    hasId &&
    (Object.prototype.hasOwnProperty.call(obj, 'result') ||
      Object.prototype.hasOwnProperty.call(obj, 'error'))
  ) {
    // Response form. Pass it through; let downstream code branch on
    // `result` vs `error`.
    return { kind: 'response', response: obj as unknown as JsonRpcResponse };
  }

  if (typeof obj.method === 'string') {
    return { kind: 'notification', method: obj.method, params: obj.params };
  }

  return { kind: 'unknown' };
}

/**
 * Convert a JSON-RPC error response into a structured
 * {@link DriverClientError}. The `retriable` flag is decided by
 * {@link mapDaemonErrorRetriable} keyed on the daemon's numeric code.
 */
export function errorFromResponse(response: JsonRpcErrorResponse): DriverClientError {
  return driverError('anvil-daemon-error', response.error.message, {
    retriable: mapDaemonErrorRetriable(response.error.code),
    data: {
      code: response.error.code,
      ...(response.error.data === undefined ? {} : { daemon_data: response.error.data }),
    },
  });
}

/**
 * Encode a frame to a single NDJSON line (terminating `\n`). The
 * daemon's framer expects one line per envelope; this function keeps
 * encoding centralised so callers never accidentally emit a
 * pretty-printed multi-line payload.
 */
export function encodeNdjsonLine(value: unknown): string {
  return `${JSON.stringify(value)}\n`;
}
