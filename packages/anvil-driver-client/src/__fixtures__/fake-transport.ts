/**
 * In-memory transport double for unit tests. Provides:
 *
 *   - A factory that produces fresh transport instances per
 *     `client.connect()` attempt — so each reconnect sees a clean
 *     state and the harness can inject failures per attempt.
 *   - Synchronous `pushIncoming` to feed bytes into the framer
 *     without involving real I/O.
 *   - `lastInstance` accessor to reach the most recently constructed
 *     transport for assertions on outgoing frames.
 */

import { Buffer } from 'node:buffer';

import type {
  Transport,
  TransportCloseCause,
  TransportFactory,
  TransportFactoryOptions,
  TransportHandlers,
} from '../transport/types.js';

import { driverError } from '../errors.js';

export interface FakeTransportInstance extends Transport {
  /** Bytes the client has sent to this transport, in order. */
  readonly outbound: string[];
  /** Whether `connect()` was invoked. */
  readonly connectCalled: boolean;
  /** True once `close()` has fired (regardless of cause). */
  readonly closed: boolean;
  /** Push raw bytes from the "daemon" into the client's framer. */
  pushIncoming(chunk: string | Uint8Array): void;
  /** Simulate the daemon dropping the transport. */
  dropFromPeer(cause?: TransportCloseCause): void;
  /** Read each outgoing line as a parsed JSON value (one per line). */
  outboundJson(): unknown[];
  /** Wait until at least one frame has been written. Test-only
   *  helper. */
  waitForOutbound(count?: number, timeoutMs?: number): Promise<void>;
}

export interface FakeTransportRecipe {
  /** Reject the connect with this error. */
  connectError?: Error;
  /** Drop the transport immediately after a successful connect; useful
   *  for testing in-flight cancellation paths. */
  dropAfterConnect?: 'peer' | 'error';
  /** Auto-respond to outgoing JSON-RPC frames with the given handler.
   *  Lets tests pin a fake daemon that echoes responses inline. */
  respond?: (
    line: unknown,
    push: (response: unknown) => void,
    drop: (cause?: TransportCloseCause) => void
  ) => void;
}

export interface FakeTransportFactory {
  factory: TransportFactory;
  /** Sequence of recipes consumed in order. After exhaustion the
   *  factory falls through to the default (success, no responses). */
  recipes: FakeTransportRecipe[];
  /** Most recently constructed instance. */
  lastInstance: FakeTransportInstance | null;
  /** All instances in construction order. */
  instances: FakeTransportInstance[];
}

class FakeTransport implements FakeTransportInstance {
  public readonly outbound: string[] = [];
  public connectCalled = false;
  public closed = false;
  private handlers: TransportHandlers | null = null;
  private readonly recipe: FakeTransportRecipe;

  public constructor(recipe: FakeTransportRecipe) {
    this.recipe = recipe;
  }

  public async connect(handlers: TransportHandlers): Promise<void> {
    this.connectCalled = true;
    if (this.recipe.connectError !== undefined) {
      throw this.recipe.connectError;
    }
    this.handlers = handlers;
    if (this.recipe.dropAfterConnect !== undefined) {
      const cause = this.recipe.dropAfterConnect;
      // Defer one tick so the calling code can reach `connected`
      // state before we tear down — surfaces the exact race the
      // brief flags ("in-flight requests on transport drop").
      await Promise.resolve();
      this.fireClose(cause);
    }
  }

  public async send(chunk: string): Promise<void> {
    if (this.closed || this.handlers === null) {
      throw driverError('anvil-daemon-transport-drop', 'fake transport already closed');
    }
    this.outbound.push(chunk);
    if (this.recipe.respond) {
      const lines = chunk.split('\n').filter((s) => s.length > 0);
      for (const line of lines) {
        let parsed: unknown;
        try {
          parsed = JSON.parse(line);
        } catch {
          parsed = { __invalid: line };
        }
        this.recipe.respond(
          parsed,
          (resp) => this.pushIncoming(JSON.stringify(resp) + '\n'),
          (cause) => this.dropFromPeer(cause)
        );
      }
    }
  }

  public async close(): Promise<void> {
    if (this.closed) {
      return;
    }
    this.fireClose('local');
  }

  public pushIncoming(chunk: string | Uint8Array): void {
    if (this.handlers === null) {
      throw new Error('fake transport: pushIncoming before connect');
    }
    const buf = typeof chunk === 'string' ? Buffer.from(chunk, 'utf8') : chunk;
    this.handlers.onData(buf);
  }

  public dropFromPeer(cause: TransportCloseCause = 'peer'): void {
    this.fireClose(cause);
  }

  public outboundJson(): unknown[] {
    return this.outbound.flatMap((line) =>
      line
        .split('\n')
        .filter((s) => s.length > 0)
        .map((s) => JSON.parse(s) as unknown)
    );
  }

  public async waitForOutbound(count = 1, timeoutMs = 1_000): Promise<void> {
    const start = Date.now();
    while (this.outbound.length < count) {
      if (Date.now() - start > timeoutMs) {
        throw new Error(
          `fake transport: timed out waiting for ${count} outbound frames (have ${this.outbound.length})`
        );
      }
      await new Promise((resolve) => setImmediate(resolve));
    }
  }

  private fireClose(cause: TransportCloseCause): void {
    if (this.closed) {
      return;
    }
    this.closed = true;
    const handlers = this.handlers;
    this.handlers = null;
    handlers?.onClose(cause);
  }
}

export function makeFakeTransportFactory(
  recipes: FakeTransportRecipe[] = []
): FakeTransportFactory {
  const state: FakeTransportFactory = {
    factory: (): Transport => {
      const recipe = state.recipes.shift() ?? {};
      const inst = new FakeTransport(recipe);
      state.lastInstance = inst;
      state.instances.push(inst);
      return inst;
    },
    recipes,
    lastInstance: null,
    instances: [],
  };
  return state;
}

export type { TransportFactoryOptions };
