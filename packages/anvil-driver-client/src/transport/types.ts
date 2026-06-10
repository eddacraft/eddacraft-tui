/**
 * Transport abstraction.
 *
 * The driver-client speaks the same JSON-RPC-over-NDJSON protocol on
 * Unix domain sockets (Linux/macOS) and Windows named pipes; this
 * interface is the seam between the protocol layer and the
 * platform-specific bytes-on-the-wire layer. A test fake implements
 * the same interface to drive the protocol layer without sockets.
 *
 * Lifecycle invariants the implementation MUST maintain:
 * 1. `connect()` is one-shot. After `close()`, callers must construct
 *    a fresh transport — reconnection is the {@link DriverClient}'s
 *    responsibility, layered on top of the transport.
 * 2. `connect()` performs the wrong-owner check synchronously before
 *    returning. A successful `connect()` resolution means the peer
 *    passed the platform-specific owner gate; the consumer never sees
 *    a transport that bypassed it.
 * 3. `onData` fires in chunk-arrival order. The transport does NOT
 *    decode NDJSON or JSON-RPC; the framer is layered above.
 * 4. `onClose` fires exactly once after the peer / OS tears the
 *    transport down. After it fires, `send()` rejects with a
 *    structured error.
 */

import type { DriverClientError } from '../errors.js';

export interface TransportHandlers {
  /** Raw bytes from the daemon. The transport never aggregates frames
   *  — every chunk surfaces as it arrives. */
  onData: (chunk: Uint8Array) => void;
  /** Fired exactly once when the peer disappears or `close()` is
   *  called. The transport does NOT distinguish "we closed cleanly"
   *  from "peer dropped" — the higher-level client tracks intent. */
  onClose: (cause: TransportCloseCause) => void;
}

export type TransportCloseCause =
  /** Local consumer asked to close. */
  | 'local'
  /** Peer hung up cleanly (FIN / EOF). */
  | 'peer'
  /** Underlying socket / pipe surfaced an error. */
  | 'error';

export interface Transport {
  /** Open the underlying socket / pipe. Must perform the wrong-owner
   *  check before resolving; rejects with `anvil-daemon-wrong-owner`
   *  if the peer is not the current user. Rejects with
   *  `anvil-daemon-unavailable` if no listener exists. When connect()
   *  rejects before the underlying socket was established (e.g. the
   *  owner gate refused the path), `onClose` is NOT fired — the
   *  handlers were never registered. */
  connect(handlers: TransportHandlers): Promise<void>;
  /** Send raw bytes (typically one NDJSON line). Rejects with a
   *  structured {@link DriverClientError} if the transport has been
   *  closed since `connect()`. */
  send(chunk: string): Promise<void>;
  /** Close the transport. Idempotent; once closed, `send()` rejects
   *  and `onClose` has fired. Errors during close are swallowed —
   *  the consumer cannot do anything actionable with them. */
  close(): Promise<void>;
}

/**
 * Configuration shape for the transport factory. Concrete transports
 * pull from this; tests substitute their own fake.
 */
export interface TransportFactoryOptions {
  /** Path to the Unix socket (Linux/macOS) — overrides the default
   *  `$XDG_RUNTIME_DIR/anvil/intercept.sock` resolution. */
  socketPath?: string;
  /** Windows named-pipe name — overrides the default
   *  `\\.\pipe\anvil-intercept-<sid>` resolution. */
  pipeName?: string;
  /** Current-user SID provider forwarded to the Windows transport's
   *  ownership gate, overriding the default `whoami /user` resolution.
   *  Required for non-Windows rigs that drive the Windows transport
   *  via an explicit `pipeName` (the default provider fails the gate
   *  closed there) and for impersonating services. */
  currentUserSid?: () => string;
}

/**
 * Factory the {@link DriverClient} calls every time it needs to
 * (re)connect. A fresh transport per attempt keeps state simple: the
 * client doesn't have to reset transport-internal handlers, and a
 * test fake can vend a fresh recording double for each attempt.
 */
export type TransportFactory = (options: TransportFactoryOptions) => Transport;

export type { DriverClientError };
