/**
 * Structured error contract for the driver client.
 *
 * Per DRVR-001's expected outcome:
 * - Timeout, transport-drop, and reconnect errors carry a stable
 *   `error` discriminator and a `retriable` boolean so consumers (MCP
 *   tool handlers, editor diagnostics) can react without sniffing
 *   message strings.
 * - The MCP-driver degraded contract (§4.5 of the editor/mcp design
 *   spec) uses `anvil-daemon-unavailable` and `anvil-daemon-timeout`
 *   verbatim. The driver-client emits the same names so an MCP-driver
 *   consumer can pass the structured error through unchanged.
 */

/**
 * Stable error discriminators surfaced by `DriverClient.request()` and
 * `DriverClient.connect()`. New variants MUST be added to this union;
 * consumers that switch on `error` get a compiler error when a new
 * variant is missing.
 */
export type DriverErrorCode =
  /** RPC didn't complete inside the configured per-request timeout. */
  | 'anvil-daemon-timeout'
  /** Daemon socket / pipe is not reachable (no listener, peer drop,
   *  reconnection budget exceeded). MCP-driver §4.5 contract name. */
  | 'anvil-daemon-unavailable'
  /** In-flight request was cancelled because the transport dropped
   *  before a response arrived. Retriable — the consumer can resubmit
   *  once the client reconnects. */
  | 'anvil-daemon-transport-drop'
  /** The socket / named-pipe path is not owned by the current user.
   *  Mirrors the daemon-side INTD-002 owner check; refused at connect
   *  time, NEVER retried (a wrong-owner socket retry is a security
   *  smell, not a transient fault). */
  | 'anvil-daemon-wrong-owner'
  /** Daemon-side or client-side rejected the JSON-RPC request as
   *  malformed before dispatch. Surfaces the daemon's structured
   *  `error.data` payload. Not retriable — consumer must fix the
   *  request shape. */
  | 'anvil-daemon-invalid-request'
  /** Daemon returned a JSON-RPC error response. Carries the daemon's
   *  `code`, `message`, and `data` fields. Retriable iff the daemon's
   *  error code maps to a transient class (server busy / scan
   *  timeout); see {@link mapDaemonErrorRetriable}. */
  | 'anvil-daemon-error'
  /** Reliability-budget quarantine fired locally. Consumer should not
   *  retry on this driver instance; quarantine survives reconnect. */
  | 'anvil-driver-quarantined'
  /** The client was closed by the consumer while a request was in
   *  flight. Not retriable. */
  | 'anvil-driver-closed';

/**
 * Structured error surface. Consumers downstream (MCP tool handlers,
 * VSCode diagnostic renderers, future TS drivers) receive this exact
 * shape and MUST not destructure properties beyond the contract here.
 */
export interface DriverError {
  /** Stable discriminator. See {@link DriverErrorCode}. */
  error: DriverErrorCode;
  /** Whether the consumer can re-issue the same logical request and
   *  reasonably expect success once the cause clears. The flag is
   *  authoritative — the consumer SHOULD NOT re-derive it from the
   *  code. */
  retriable: boolean;
  /** Human-readable message. Free-form; do not parse. */
  message: string;
  /** Optional carry-through of a daemon-side structured payload. For
   *  `anvil-daemon-error` this is the daemon's JSON-RPC `error.data`
   *  verbatim. */
  data?: unknown;
  /** When the rejection arose from a timeout, the configured timeout
   *  in milliseconds. Surfaced for diagnostics — the consumer SHOULD
   *  NOT branch on this field. */
  timeout_ms?: number;
}

/**
 * Concrete `Error` subclass that carries a {@link DriverError}
 * payload. Throwing through Promises preserves the structured fields
 * (the consumer reads them off the rejected value), but the JS native
 * stack is also helpful for debugging.
 *
 * Constructed via the helpers in this module; do not instantiate
 * directly from outside the package.
 */
export class DriverClientError extends Error {
  public readonly code: DriverErrorCode;
  public readonly retriable: boolean;
  public readonly data?: unknown;
  public readonly timeout_ms?: number;

  public constructor(payload: DriverError) {
    super(payload.message);
    this.name = 'DriverClientError';
    this.code = payload.error;
    this.retriable = payload.retriable;
    this.data = payload.data;
    this.timeout_ms = payload.timeout_ms;
  }

  /**
   * Serialise back to the wire-stable {@link DriverError} shape.
   * Consumers that need to forward the error across another transport
   * (MCP tool response, telemetry event) call this rather than
   * destructuring the `Error` instance.
   */
  public toJSON(): DriverError {
    return {
      error: this.code,
      retriable: this.retriable,
      message: this.message,
      ...(this.data === undefined ? {} : { data: this.data }),
      ...(this.timeout_ms === undefined ? {} : { timeout_ms: this.timeout_ms }),
    };
  }
}

/**
 * Build a {@link DriverClientError} from a partial payload.
 *
 * Centralised so the `error` discriminator and the `retriable` policy
 * are declared once per code path; callers in transport / framer /
 * client modules use this rather than `new DriverClientError(...)`.
 */
export function driverError(
  code: DriverErrorCode,
  message: string,
  options: {
    retriable?: boolean;
    data?: unknown;
    timeout_ms?: number;
  } = {}
): DriverClientError {
  const retriable = options.retriable ?? defaultRetriable(code);
  return new DriverClientError({
    error: code,
    retriable,
    message,
    ...(options.data === undefined ? {} : { data: options.data }),
    ...(options.timeout_ms === undefined ? {} : { timeout_ms: options.timeout_ms }),
  });
}

/**
 * Default retriable disposition per error code. The brief pins
 * `anvil-daemon-timeout` and `anvil-daemon-transport-drop` as
 * retriable; `anvil-daemon-wrong-owner` is explicitly NOT retriable
 * because retrying on a hostile socket is the failure mode security
 * cares about.
 */
function defaultRetriable(code: DriverErrorCode): boolean {
  switch (code) {
    case 'anvil-daemon-timeout':
    case 'anvil-daemon-transport-drop':
    case 'anvil-daemon-unavailable':
      return true;
    case 'anvil-daemon-wrong-owner':
    case 'anvil-daemon-invalid-request':
    case 'anvil-driver-quarantined':
    case 'anvil-driver-closed':
      return false;
    case 'anvil-daemon-error':
      // Caller decides via `mapDaemonErrorRetriable` based on the
      // daemon's JSON-RPC error code; the default here is conservative
      // (retriable: false) so unmapped daemon errors do not silently
      // produce reconnect storms.
      return false;
  }
}

/**
 * JSON-RPC 2.0 error codes the daemon emits today (per
 * `crates/anvil-intercept/src/ipc.rs`):
 * - `-32700` Parse error (consumer sent malformed JSON; not retriable)
 * - `-32600` Invalid Request (shape rejection; not retriable)
 * - `-32601` Method not found (consumer typo / version skew; not retriable)
 * - `-32602` Invalid params (consumer payload broken; not retriable)
 * - `-32603` Internal error (daemon-side crash; not retriable from the
 *   client's view — retrying may produce the same failure)
 * - `-32000` Server busy (scan-buffer queue full; retriable after backoff)
 * - `-32001` Scan timed out (mid-edit scan exceeded its budget; retriable
 *   with smaller buffer / less load)
 *
 * Unmapped codes default to `retriable: false` — a future error class
 * the daemon adds will be surfaced honestly rather than silently
 * encouraged into a retry loop.
 */
export function mapDaemonErrorRetriable(code: number | undefined): boolean {
  if (code === undefined) {
    return false;
  }
  return code === -32_000 || code === -32_001;
}
