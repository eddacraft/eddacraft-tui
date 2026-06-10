/**
 * `DriverClient` — the public API surface of `@eddacraft/anvil-driver-client`.
 *
 * Responsibilities (per DRVR-001 brief):
 *   - JSON-RPC 2.0 request / response over NDJSON.
 *   - Transparent reconnection with documented backoff (exponential,
 *     capped at 30s, 5 retries before bubbling).
 *   - Per-request timeout with separate defaults for read-only (10s)
 *     and enforcement-ack (500ms) methods.
 *   - In-flight cancellation on transport drop with a structured
 *     `anvil-daemon-transport-drop` (retriable: true).
 *   - Driver-side wrong-owner refusal (delegated to the transport
 *     layer).
 *   - Reliability-budget quarantine keyed on a stable identity
 *     (NOT driverName — see DRVR-007 §2.3a).
 *
 * Out of scope (deferred to DRVR-002 / DRVR-008):
 *   - Capability handshake / manifest negotiation
 *   - `anvil/enforcement/ack` semantics (the client speaks the
 *     timeout discriminator; the consumer speaks the protocol)
 *   - Protocol pinning + capability negotiation
 */

import { driverError, DriverClientError } from '../errors.js';
import {
  buildNotification,
  buildRequest,
  classifyIncoming,
  encodeNdjsonLine,
  errorFromResponse,
  type JsonRpcId,
  type JsonRpcRequest,
  type JsonRpcResponse,
} from '../framing/jsonrpc.js';
import { NdjsonFramer, type FramingError } from '../framing/ndjson.js';
import {
  createMidEditValidator,
  type ValidateMidEditOptions,
  type ValidateMidEditParams,
  type ValidateMidEditResult,
} from '../midedit/validate-mid-edit.js';
import { ReliabilityBudget } from '../reliability/budget.js';
import { defaultTransportFactory } from '../transport/index.js';
import type { Transport, TransportFactory } from '../transport/types.js';

import {
  DEFAULT_ENFORCEMENT_ACK_METHODS,
  DEFAULT_ENFORCEMENT_ACK_TIMEOUT_MS,
  DEFAULT_READ_TIMEOUT_MS,
  DEFAULT_RECONNECT_CAP_MS,
  DEFAULT_RECONNECT_INITIAL_MS,
  DEFAULT_RECONNECT_MAX_ATTEMPTS,
  type DriverClientEventMap,
  type DriverClientOptions,
  type DriverNotificationTopics,
  type DriverRequestOptions,
  type SubscriberHandler,
} from './types.js';

interface PendingRequest {
  id: JsonRpcId;
  resolve: (value: unknown) => void;
  reject: (err: DriverClientError) => void;
  timeoutHandle: unknown;
  timeoutMs: number;
}

interface ReconnectConfig {
  initialMs: number;
  capMs: number;
  maxAttempts: number;
  jitter: number;
  random: () => number;
}

interface Scheduler {
  setTimeout: (cb: () => void, ms: number) => unknown;
  clearTimeout: (handle: unknown) => void;
}

const DEFAULT_SCHEDULER: Scheduler = {
  setTimeout: (cb, ms) => globalThis.setTimeout(cb, ms),
  clearTimeout: (handle) =>
    globalThis.clearTimeout(handle as ReturnType<typeof globalThis.setTimeout>),
};

type Listener<T> = (event: T) => void;

export class DriverClient {
  private readonly transportFactory: TransportFactory;
  private readonly transportOptions: {
    socketPath?: string;
    pipeName?: string;
    currentUserSid?: () => string;
  };
  private readonly readTimeoutMs: number;
  private readonly enforcementAckTimeoutMs: number;
  private readonly enforcementAckMethods: Set<string>;
  private readonly reconnectConfig: ReconnectConfig;
  private readonly scheduler: Scheduler;
  private readonly reliability: ReliabilityBudget;
  private readonly midEditOptions: ValidateMidEditOptions | undefined;
  private midEditValidator:
    | ((params: ValidateMidEditParams) => Promise<ValidateMidEditResult>)
    | undefined;
  private driverIdentity: string | undefined;

  private transport: Transport | null = null;
  private framer: NdjsonFramer | null = null;
  private nextRequestId = 1;
  private pending = new Map<string, PendingRequest>();
  private subscribers = new Map<string, Set<SubscriberHandler<unknown>>>();
  private listeners = new Map<keyof DriverClientEventMap, Set<Listener<unknown>>>();
  private state: 'unbound' | 'connecting' | 'connected' | 'reconnecting' | 'closed' = 'unbound';
  private connectAttempt = 0;
  private reconnectHandle: unknown = undefined;
  private explicitClose = false;

  public constructor(options: DriverClientOptions = {}) {
    this.transportFactory = options.transportFactory ?? defaultTransportFactory;
    this.transportOptions = {
      ...(options.socketPath !== undefined ? { socketPath: options.socketPath } : {}),
      ...(options.pipeName !== undefined ? { pipeName: options.pipeName } : {}),
      ...(options.currentUserSid !== undefined ? { currentUserSid: options.currentUserSid } : {}),
    };
    this.readTimeoutMs = options.timeoutsMs?.readOnly ?? DEFAULT_READ_TIMEOUT_MS;
    this.enforcementAckTimeoutMs =
      options.timeoutsMs?.enforcementAck ?? DEFAULT_ENFORCEMENT_ACK_TIMEOUT_MS;
    this.enforcementAckMethods = new Set(
      options.enforcementAckMethods ?? DEFAULT_ENFORCEMENT_ACK_METHODS
    );
    this.reconnectConfig = {
      initialMs: options.reconnect?.initialMs ?? DEFAULT_RECONNECT_INITIAL_MS,
      capMs: options.reconnect?.capMs ?? DEFAULT_RECONNECT_CAP_MS,
      maxAttempts: options.reconnect?.maxAttempts ?? DEFAULT_RECONNECT_MAX_ATTEMPTS,
      jitter: options.reconnect?.jitter ?? 0.2,
      random: options.reconnect?.random ?? Math.random,
    };
    if (this.reconnectConfig.initialMs <= 0) {
      throw new RangeError('reconnect.initialMs must be positive');
    }
    if (this.reconnectConfig.capMs < this.reconnectConfig.initialMs) {
      throw new RangeError('reconnect.capMs must be >= initialMs');
    }
    if (this.reconnectConfig.maxAttempts < 0) {
      throw new RangeError('reconnect.maxAttempts must be non-negative');
    }
    if (this.reconnectConfig.jitter < 0 || this.reconnectConfig.jitter >= 1) {
      throw new RangeError('reconnect.jitter must be in [0, 1)');
    }
    if (this.readTimeoutMs <= 0 || this.enforcementAckTimeoutMs <= 0) {
      throw new RangeError('timeoutsMs values must be positive');
    }
    this.scheduler = options.scheduler ?? DEFAULT_SCHEDULER;
    if (options.reliabilityBudget instanceof ReliabilityBudget) {
      this.reliability = options.reliabilityBudget;
    } else {
      this.reliability = new ReliabilityBudget(options.reliabilityBudget ?? {});
    }
    this.driverIdentity = options.driverIdentity;
    this.midEditOptions = options.midEdit;
  }

  // ----------------------------------------------------------------
  // Public API
  // ----------------------------------------------------------------

  /**
   * Open the transport. Resolves once the underlying socket / pipe
   * is connected; rejects with a structured
   * {@link DriverClientError} on owner-check failure or unreachable
   * daemon. Callers MAY call `connect()` more than once with no-op
   * effect when already connected.
   */
  public async connect(): Promise<void> {
    if (this.state === 'closed') {
      throw driverError('anvil-driver-closed', 'driver client has been closed');
    }
    if (this.state === 'connected') {
      return;
    }
    this.explicitClose = false;
    // An explicit connect() after a reconnect-budget exhaustion or a
    // local stop wants a fresh attempt counter — otherwise the very
    // next peer drop would skip backoff entirely.
    this.connectAttempt = 0;
    await this.openTransport();
  }

  /**
   * Issue a JSON-RPC request and wait for the response.
   *
   * The timeout used is, in order of precedence:
   *   1. `options.timeoutMs`, if supplied.
   *   2. The enforcement-ack default if `options.enforcementAck` is
   *      true OR the method is in the configured enforcement-ack
   *      method set.
   *   3. The read-only default.
   */
  public async request<R = unknown>(
    method: string,
    params?: unknown,
    options: DriverRequestOptions = {}
  ): Promise<R> {
    if (this.state === 'closed') {
      throw driverError('anvil-driver-closed', 'driver client has been closed');
    }
    if (this.reliability.isQuarantined(this.driverIdentity)) {
      throw driverError(
        'anvil-driver-quarantined',
        `driver ${this.driverIdentity ?? 'unknown'} is in reliability-budget quarantine`
      );
    }

    if (this.transport === null || this.state !== 'connected') {
      // No connect-on-first-request: surface the explicit state to
      // the caller. The `retriable: true` flag invites the consumer
      // to retry once they've observed `connected`.
      throw driverError(
        'anvil-daemon-unavailable',
        'transport not connected; call connect() first'
      );
    }

    const id = this.allocateRequestId();
    const envelope = buildRequest(id, method, params);
    const timeoutMs = this.timeoutFor(method, options);
    return this.dispatchRequest<R>(envelope, timeoutMs);
  }

  /**
   * Send a JSON-RPC notification (no response expected). Returns
   * once the bytes are flushed to the transport. Reliability-budget
   * accounting does NOT apply to notifications (the daemon never
   * acks them, so per-call success/failure is undefined).
   */
  public async notify(method: string, params?: unknown): Promise<void> {
    if (this.state === 'closed') {
      throw driverError('anvil-driver-closed', 'driver client has been closed');
    }
    if (this.transport === null || this.state !== 'connected') {
      throw driverError(
        'anvil-daemon-unavailable',
        'transport not connected; call connect() first'
      );
    }
    const envelope = buildNotification(method, params);
    await this.transport.send(encodeNdjsonLine(envelope));
  }

  /**
   * Mid-edit validation entry point — RTAI-004.
   *
   * Bound method form of `createMidEditValidator(this, ...)`. The
   * validator is constructed lazily on first call and reused across
   * subsequent calls, so the debouncer + dedup cache survive the
   * lifetime of the client.
   *
   * Returns a {@link ValidateMidEditResult} envelope rather than
   * throwing on daemon errors — preserves RTAI-008's
   * "errors-as-first-class" contract.
   *
   * The default debounce is 80ms (the typing cycle, per the brief);
   * override per-call via `params.debounceMs` (e.g. `0` in tests).
   */
  public validateMidEdit(params: ValidateMidEditParams): Promise<ValidateMidEditResult> {
    if (this.midEditValidator === undefined) {
      this.midEditValidator = createMidEditValidator(this, this.midEditOptions ?? {});
    }
    return this.midEditValidator(params);
  }

  /**
   * Subscribe to a JSON-RPC notification topic. The handler is
   * invoked synchronously per notification frame. Returns an
   * unsubscribe function.
   *
   * Strongly-typed overload covers the well-known topics in
   * {@link DriverNotificationTopics}; the string-based fallback
   * accepts any method name and surfaces `unknown` params for
   * forward compatibility with daemon notifications the client
   * doesn't yet model.
   */
  public subscribe<K extends keyof DriverNotificationTopics>(
    topic: K,
    handler: SubscriberHandler<DriverNotificationTopics[K]>
  ): () => void;
  public subscribe<E = unknown>(topic: string, handler: SubscriberHandler<E>): () => void;
  public subscribe(topic: string, handler: SubscriberHandler<unknown>): () => void {
    let bucket = this.subscribers.get(topic);
    if (bucket === undefined) {
      bucket = new Set();
      this.subscribers.set(topic, bucket);
    }
    bucket.add(handler);
    return () => {
      const set = this.subscribers.get(topic);
      if (set === undefined) {
        return;
      }
      set.delete(handler);
      if (set.size === 0) {
        this.subscribers.delete(topic);
      }
    };
  }

  /**
   * Subscribe to lifecycle / observability events on the client
   * itself. Distinct from notification subscriptions: these fire on
   * client-internal state changes, not daemon-emitted JSON-RPC
   * notifications.
   */
  public on<K extends keyof DriverClientEventMap>(
    event: K,
    handler: Listener<DriverClientEventMap[K]>
  ): () => void {
    let bucket = this.listeners.get(event);
    if (bucket === undefined) {
      bucket = new Set();
      this.listeners.set(event, bucket);
    }
    bucket.add(handler as Listener<unknown>);
    return () => {
      const set = this.listeners.get(event);
      if (set === undefined) {
        return;
      }
      set.delete(handler as Listener<unknown>);
      if (set.size === 0) {
        this.listeners.delete(event);
      }
    };
  }

  /**
   * Close the client. Cancels in-flight requests with a structured
   * `anvil-driver-closed` error, tears down the transport, and
   * marks the client unusable. Idempotent.
   */
  public async close(): Promise<void> {
    if (this.state === 'closed') {
      return;
    }
    this.explicitClose = true;
    this.state = 'closed';
    if (this.reconnectHandle !== undefined) {
      this.scheduler.clearTimeout(this.reconnectHandle);
      this.reconnectHandle = undefined;
    }
    this.cancelAllPending(
      driverError('anvil-driver-closed', 'driver client closed while request was in flight')
    );
    if (this.transport !== null) {
      try {
        await this.transport.close();
      } catch {
        // Transport close is best-effort; nothing the caller can do.
      }
      this.transport = null;
    }
    this.framer = null;
    // Drop subscribers and listeners so a closed client does not leak
    // references to consumer-supplied closures.
    this.subscribers.clear();
    this.listeners.clear();
  }

  /**
   * Update the daemon-minted identity used as the reliability-
   * budget key. Consumers call this after observing
   * `correlation.originating_driver_id` on the first daemon-bound
   * envelope (typically the handshake response). Pass `undefined`
   * to clear the key (e.g. on a fresh handshake).
   *
   * MUST NOT be called with `driverName` — see DRVR-007 §2.3a.
   */
  public setDriverIdentity(identity: string | undefined): void {
    this.driverIdentity = identity;
  }

  /** Snapshot of the reliability budget for telemetry / tests. */
  public reliabilitySnapshot(): ReturnType<ReliabilityBudget['snapshot']> {
    return this.reliability.snapshot();
  }

  // ----------------------------------------------------------------
  // Internals
  // ----------------------------------------------------------------

  private timeoutFor(method: string, options: DriverRequestOptions): number {
    if (options.timeoutMs !== undefined) {
      if (options.timeoutMs <= 0) {
        throw new RangeError('timeoutMs must be positive');
      }
      return options.timeoutMs;
    }
    const isAck = options.enforcementAck ?? this.enforcementAckMethods.has(method);
    return isAck ? this.enforcementAckTimeoutMs : this.readTimeoutMs;
  }

  private allocateRequestId(): JsonRpcId {
    const id = this.nextRequestId;
    this.nextRequestId = id + 1;
    // String form: small allocation but cheap; the daemon's
    // `valid_jsonrpc_id` accepts strings up to 256 bytes which we
    // never approach with sequential ints.
    return `req-${id}`;
  }

  private dispatchRequest<R>(envelope: JsonRpcRequest, timeoutMs: number): Promise<R> {
    return new Promise<R>((resolve, reject) => {
      const idKey = String(envelope.id);
      const timeoutHandle = this.scheduler.setTimeout(() => {
        this.handleTimeout(idKey, timeoutMs);
      }, timeoutMs);

      this.pending.set(idKey, {
        id: envelope.id,
        resolve: resolve as (v: unknown) => void,
        reject,
        timeoutHandle,
        timeoutMs,
      });

      const sendPromise = this.transport!.send(encodeNdjsonLine(envelope));
      sendPromise.catch((err: unknown) => {
        // The send itself failed — typically because the transport
        // dropped between dispatch and write. Cancel the pending
        // entry with the structured error. The transport's own
        // `onClose` will also fire, but it's safe to settle once
        // here: `cancelPending` is idempotent.
        const pending = this.pending.get(idKey);
        if (pending === undefined) {
          return;
        }
        this.pending.delete(idKey);
        this.scheduler.clearTimeout(pending.timeoutHandle);
        const wrapped =
          err instanceof DriverClientError
            ? err
            : driverError(
                'anvil-daemon-transport-drop',
                `transport send failed: ${(err as Error).message}`
              );
        this.recordReliability(false);
        pending.reject(wrapped);
      });
    });
  }

  private handleTimeout(idKey: string, timeoutMs: number): void {
    const pending = this.pending.get(idKey);
    if (pending === undefined) {
      return;
    }
    this.pending.delete(idKey);
    this.recordReliability(false);
    pending.reject(
      driverError('anvil-daemon-timeout', `daemon did not respond within ${timeoutMs}ms`, {
        timeout_ms: timeoutMs,
      })
    );
  }

  private async openTransport(): Promise<void> {
    this.state = 'connecting';
    const transport = this.transportFactory(this.transportOptions);
    const framer = new NdjsonFramer({
      onFrame: (value) => this.handleIncomingFrame(value),
      onError: (err) => this.handleFramerError(err),
    });

    try {
      await transport.connect({
        onData: (chunk) => framer.push(chunk),
        onClose: (cause) => this.handleTransportClose(cause),
      });
    } catch (err) {
      this.state = 'unbound';
      throw err;
    }

    this.transport = transport;
    this.framer = framer;
    this.state = 'connected';
    this.connectAttempt = 0;
    this.emit('connected', undefined);
  }

  private handleIncomingFrame(value: unknown): void {
    const classified = classifyIncoming(value);
    switch (classified.kind) {
      case 'response':
        this.handleResponse(classified.response);
        return;
      case 'notification':
        this.handleNotification(classified.method, classified.params);
        return;
      case 'unknown':
      default:
        // Unknown shape — surface as an error event but do NOT tear
        // down the connection. Mirrors the framer's discard policy
        // at a higher layer.
        this.emit('framing_error', {
          reason: 'invalid-json',
          message: 'received frame is not a JSON-RPC 2.0 envelope',
          bytes: 0,
        });
        return;
    }
  }

  private handleResponse(response: JsonRpcResponse): void {
    if (response.id === null || response.id === undefined) {
      // Response without an id — could not have been ours. Surface
      // as a non-fatal error event.
      this.emit('error', { cause: { reason: 'response-missing-id', response } });
      return;
    }
    const idKey = String(response.id);
    const pending = this.pending.get(idKey);
    if (pending === undefined) {
      // Late response (we already timed out / cancelled). Drop on
      // the floor — the consumer already got their structured
      // error.
      return;
    }
    this.pending.delete(idKey);
    this.scheduler.clearTimeout(pending.timeoutHandle);

    if ('error' in response) {
      this.recordReliability(false);
      pending.reject(errorFromResponse(response));
      return;
    }
    this.recordReliability(true);
    pending.resolve(response.result);
  }

  private handleNotification(method: string, params: unknown): void {
    const subs = this.subscribers.get(method);
    if (subs === undefined || subs.size === 0) {
      return;
    }
    for (const handler of subs) {
      try {
        handler(params);
      } catch (cause) {
        this.emit('error', { cause });
      }
    }
  }

  private handleFramerError(err: FramingError): void {
    this.emit('framing_error', err);
  }

  private handleTransportClose(cause: 'local' | 'peer' | 'error'): void {
    const wasConnected = this.state === 'connected' || this.state === 'reconnecting';
    this.transport = null;
    this.framer?.reset();
    this.framer = null;
    this.cancelAllPending(
      driverError(
        'anvil-daemon-transport-drop',
        `transport closed (${cause}) while request was in flight`
      )
    );

    if (this.explicitClose || this.state === 'closed') {
      this.state = 'closed';
      this.emit('disconnected', { cause });
      return;
    }

    this.state = 'unbound';
    this.emit('disconnected', { cause });
    if (wasConnected && cause !== 'local') {
      this.scheduleReconnect();
    }
  }

  private scheduleReconnect(): void {
    if (this.connectAttempt >= this.reconnectConfig.maxAttempts) {
      this.emit('reconnect_failed', { attempts: this.connectAttempt });
      return;
    }
    this.connectAttempt += 1;
    this.state = 'reconnecting';

    const baseDelay = Math.min(
      this.reconnectConfig.initialMs * 2 ** (this.connectAttempt - 1),
      this.reconnectConfig.capMs
    );
    const jitter = this.reconnectConfig.jitter;
    const random = this.reconnectConfig.random;
    // Symmetric jitter: ±(jitter * 100)% around the base delay.
    const jitterFactor = 1 + (random() * 2 - 1) * jitter;
    const delayMs = Math.max(0, Math.round(baseDelay * jitterFactor));

    this.emit('reconnecting', { attempt: this.connectAttempt, delayMs });
    this.reconnectHandle = this.scheduler.setTimeout(() => {
      this.reconnectHandle = undefined;
      this.attemptReconnect();
    }, delayMs);
  }

  private attemptReconnect(): void {
    if (this.state !== 'reconnecting') {
      return;
    }
    this.openTransport().catch((err: unknown) => {
      if (this.state === 'closed') {
        return;
      }
      // Reconnect attempt failed. If we have budget left, retry; if
      // not, bubble the structured error to the consumer via the
      // event surface. The consumer's next `request()` will surface
      // `anvil-daemon-unavailable`.
      if (this.connectAttempt < this.reconnectConfig.maxAttempts) {
        this.scheduleReconnect();
        return;
      }
      this.emit('reconnect_failed', { attempts: this.connectAttempt, lastError: err });
    });
  }

  private cancelAllPending(err: DriverClientError): void {
    if (this.pending.size === 0) {
      return;
    }
    const drained = [...this.pending.values()];
    this.pending.clear();
    for (const entry of drained) {
      this.scheduler.clearTimeout(entry.timeoutHandle);
      this.recordReliability(false);
      entry.reject(err);
    }
  }

  private recordReliability(success: boolean): void {
    if (success) {
      this.reliability.recordSuccess(this.driverIdentity);
      return;
    }
    const justQuarantined = this.reliability.recordFailure(this.driverIdentity);
    if (justQuarantined) {
      this.emit('error', {
        cause: driverError(
          'anvil-driver-quarantined',
          `driver ${this.driverIdentity ?? 'unknown'} entered quarantine after repeated failures`
        ),
      });
    }
  }

  private emit<K extends keyof DriverClientEventMap>(
    event: K,
    payload: DriverClientEventMap[K]
  ): void {
    const set = this.listeners.get(event);
    if (set === undefined || set.size === 0) {
      return;
    }
    for (const handler of set) {
      try {
        handler(payload);
      } catch {
        // Listeners cannot kill the client; swallow throws here.
        // The internal `error` event already exists for handlers
        // that want to react to client-side errors.
      }
    }
  }
}
