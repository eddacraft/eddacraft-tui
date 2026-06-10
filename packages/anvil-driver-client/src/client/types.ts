/**
 * Public types exposed by {@link DriverClient}.
 *
 * Kept in a dedicated module so consumers can import the typed
 * contract without pulling in transport / framer internals.
 */

import type { Diagnostic } from '../diagnostics/types.js';
import type { JsonRpcId } from '../framing/jsonrpc.js';
import type { ValidateMidEditOptions } from '../midedit/validate-mid-edit.js';
import type { ReliabilityBudget, ReliabilityBudgetOptions } from '../reliability/budget.js';
import type { TransportFactory } from '../transport/types.js';

/**
 * Per-request override knobs.
 */
export interface DriverRequestOptions {
  /** Override the per-request timeout. The default is decided by
   *  whether the method is in the enforcement-ack list or not. */
  timeoutMs?: number;
  /** Mark the request as enforcement-ack-class so the 500ms default
   *  timeout fires. The brief pins this discriminator: read-only
   *  methods get 10s; enforcement acks get 500ms. */
  enforcementAck?: boolean;
}

/**
 * Default per-request timeouts. The brief explicitly pins these:
 *   - 10s for read-only methods
 *   - 500ms for enforcement-ack methods
 *
 * Both are configurable via {@link DriverClientOptions.timeoutsMs};
 * the constants here are surfaced so consumers can check the active
 * defaults without instantiating a client.
 */
export const DEFAULT_READ_TIMEOUT_MS = 10_000;
export const DEFAULT_ENFORCEMENT_ACK_TIMEOUT_MS = 500;

/**
 * Default reconnect backoff. The brief pins:
 *   "exponential backoff capped at 30s, 5 retries before bubbling
 *    structured error".
 */
export const DEFAULT_RECONNECT_INITIAL_MS = 200;
export const DEFAULT_RECONNECT_CAP_MS = 30_000;
export const DEFAULT_RECONNECT_MAX_ATTEMPTS = 5;

/**
 * Methods classified as enforcement-ack — they fall under the
 * 500ms timeout regime by default. Aligned with the
 * editor-and-mcp-driver-design.md §3.2 method table:
 *
 *   - `anvil/enforcement/ack`
 *   - `anvil/enforcement/refuse`
 *
 * The list is consulted only when the consumer doesn't explicitly
 * set `enforcementAck` on the request. Callers SHOULD set the flag
 * explicitly for new methods to keep the implicit list small.
 */
export const DEFAULT_ENFORCEMENT_ACK_METHODS: ReadonlySet<string> = new Set([
  'anvil/enforcement/ack',
  'anvil/enforcement/refuse',
]);

/**
 * Subscriber callback shape. Notifications cross the JSON-RPC wire
 * with a `method` and a `params` object; the typed handler receives
 * the params. Throwing from a handler is caught and surfaced via the
 * client's `error` event so a buggy subscriber cannot tear down the
 * client.
 */
export type SubscriberHandler<E> = (event: E) => void;

/**
 * Lifecycle / observability events the client emits. Consumers
 * subscribe via the `on` method on {@link DriverClient}.
 */
export type DriverClientEventMap = {
  /** Transport opened successfully (post-handshake if any). Fires on
   *  every successful (re)connection. */
  connected: void;
  /** Transport closed — either local-initiated or peer-initiated. */
  disconnected: { cause: 'local' | 'peer' | 'error' };
  /** Reconnect attempt scheduled. Useful for surface-side logging /
   *  status-bar UX. */
  reconnecting: { attempt: number; delayMs: number };
  /** Reconnection budget exhausted; the next call surfaces
   *  `anvil-daemon-unavailable`. */
  reconnect_failed: { attempts: number; lastError?: unknown };
  /** Framer rejected an inbound line. */
  framing_error: { reason: string; message: string; bytes: number };
  /** Internal client error (handler threw, etc.). Carries the raw
   *  cause; the client suppresses the throw to keep the event loop
   *  alive. */
  error: { cause: unknown };
};

/**
 * Static notification topics the daemon emits today. The strongly-
 * typed `subscribe` overload uses this map; consumers can still
 * subscribe to arbitrary string topics for forward compatibility
 * (returns `unknown` payload).
 */
export interface DriverNotificationTopics {
  /** `anvil/publishDiagnostics` — DRVR-002 protocol notification.
   *  Carries `{ uri, version, diagnostics: Diagnostic[] }`. */
  'anvil/publishDiagnostics': {
    uri: string;
    version?: number;
    diagnostics: Diagnostic[];
  };
}

export interface DriverClientOptions {
  /** Override the platform default. Linux/macOS: Unix socket path.
   *  Windows: pipe name. */
  socketPath?: string;
  pipeName?: string;
  /** Current-user SID provider for the Windows transport's ownership
   *  gate (see `WindowsTransportOptions.currentUserSid`). Needed by
   *  non-Windows test rigs that pass `pipeName` and by impersonating
   *  services; production Windows consumers can rely on the default
   *  `whoami /user` resolution. */
  currentUserSid?: () => string;
  /** Inject a custom transport factory — used by tests and by the
   *  rare consumer that wants to wrap the transport. */
  transportFactory?: TransportFactory;
  /** Per-request timeout overrides. */
  timeoutsMs?: {
    readOnly?: number;
    enforcementAck?: number;
  };
  /** Reconnect backoff overrides. */
  reconnect?: {
    initialMs?: number;
    capMs?: number;
    maxAttempts?: number;
    /** Random jitter fraction applied to each delay. Default: 0.2
     *  (±20%). Set to 0 in tests for deterministic timing. */
    jitter?: number;
    /** Test-only random source. */
    random?: () => number;
  };
  /** Reliability-budget configuration. The client builds its own
   *  ledger by default; consumers can supply a shared one if a
   *  process holds multiple driver instances. */
  reliabilityBudget?: ReliabilityBudget | ReliabilityBudgetOptions;
  /** Stable identity used as the reliability-budget key. The
   *  daemon-minted `correlation.originating_driver_id` from
   *  INTD-015's envelope is the authoritative source; consumers wire
   *  it through here once the handshake observes it. NEVER pass
   *  `driverName` — see DRVR-007 §2.3a. */
  driverIdentity?: string;
  /** Method-name set used to decide the default timeout. Consumers
   *  pass their own list to extend the enforcement-ack family. */
  enforcementAckMethods?: Iterable<string>;
  /** Test hook: replace the timer used for per-request timeouts and
   *  reconnect backoff. */
  scheduler?: {
    setTimeout: (cb: () => void, ms: number) => unknown;
    clearTimeout: (handle: unknown) => void;
  };
  /** RTAI-004 mid-edit configuration. The {@link DriverClient.validateMidEdit}
   *  method constructs its debouncer + dedup cache lazily on first
   *  call, using these options. Per-call `params.debounceMs` overrides
   *  the default. */
  midEdit?: ValidateMidEditOptions;
}

export type { JsonRpcId };
