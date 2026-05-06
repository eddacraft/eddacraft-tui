/**
 * NDJSON framer.
 *
 * Per DRVR-001 brief:
 * > NDJSON framer on parse error discards the frame, emits a
 * > `framing-error` event, preserves the connection.
 *
 * The framer is byte-stream-shaped: callers feed `Buffer | Uint8Array`
 * chunks via {@link NdjsonFramer.push}, and the framer emits one
 * decoded JSON value per complete `\n`-delimited line.
 *
 * Properties pinned by tests:
 * 1. **Discard, don't crash.** Malformed JSON (or oversize lines)
 *    surface as `framing-error` events; the next line is parsed
 *    cleanly. The connection is not torn down at this layer.
 * 2. **Cap on raw line size.** Adversarial peers cannot make the
 *    framer hold an unbounded buffer by sending data with no `\n`
 *    delimiter. Default cap matches the daemon's `MAX_LINE_BYTES`
 *    surplus so a legitimate `scan_buffer` request is not rejected
 *    locally.
 * 3. **UTF-8 validation.** Bytes that don't decode as UTF-8 emit a
 *    `framing-error` and are dropped — the daemon's framer does the
 *    same on its side.
 * 4. **No trailing-line silence.** A reset call surfaces any pending
 *    bytes as a `framing-error` so a half-line at EOF cannot be
 *    misinterpreted as a clean close.
 */

/**
 * Default per-line cap. Mirrors the daemon-side
 * `crates/anvil-intercept/src/ipc.rs::MAX_LINE_BYTES`:
 *   `(CONTENT_SIZE_CAP_BYTES_USIZE * 6) + (64 << 10)`
 *
 * Hard-coding the value here keeps the client decoupled from the
 * Rust crate; if the daemon raises its cap, this default needs to
 * follow. Consumers can override per-instance via
 * {@link NdjsonFramerOptions.maxLineBytes}.
 */
export const DEFAULT_MAX_LINE_BYTES = 6 * (1 << 20) + (64 << 10);

/** Reasons a frame was rejected at the framing layer. */
export type FramingErrorReason =
  | 'invalid-utf8'
  | 'invalid-json'
  | 'oversize-line'
  | 'partial-frame-on-reset';

export interface FramingError {
  reason: FramingErrorReason;
  message: string;
  /** Number of raw bytes the framer was holding when the error fired.
   *  Useful for telemetry; consumers SHOULD NOT branch on this. */
  bytes: number;
}

export interface NdjsonFramerOptions {
  maxLineBytes?: number;
}

/**
 * Subscriber callbacks the framer invokes synchronously on each
 * complete frame / error. Synchronous callbacks keep ordering simple:
 * the consumer cannot interleave a framing error with a frame from a
 * later read.
 */
export interface NdjsonFramerHandlers {
  onFrame: (value: unknown) => void;
  onError: (err: FramingError) => void;
}

export class NdjsonFramer {
  private readonly maxLineBytes: number;
  private readonly handlers: NdjsonFramerHandlers;
  private readonly decoder = new TextDecoder('utf-8', { fatal: true });
  /** Raw bytes accumulated since the last `\n`. We hold the buffer
   *  rather than a string because UTF-8 boundaries can fall mid-chunk;
   *  we only attempt UTF-8 decode once a complete line is in hand. */
  private pending: Uint8Array = new Uint8Array(0);
  private overflowed = false;

  public constructor(handlers: NdjsonFramerHandlers, options: NdjsonFramerOptions = {}) {
    this.handlers = handlers;
    this.maxLineBytes = options.maxLineBytes ?? DEFAULT_MAX_LINE_BYTES;
    if (this.maxLineBytes <= 0) {
      throw new RangeError('maxLineBytes must be positive');
    }
  }

  /**
   * Feed bytes into the framer. Synchronously emits one
   * `onFrame` / `onError` per complete or rejected line.
   *
   * The byte path is hot — each chunk is scanned for `\n` once, and
   * complete lines are sliced out without re-allocating the
   * still-pending tail.
   */
  public push(chunk: Uint8Array | Buffer): void {
    if (chunk.length === 0) {
      return;
    }

    let consumed = 0;
    while (consumed < chunk.length) {
      const newlineIdx = indexOfByte(chunk, 0x0a, consumed);
      if (newlineIdx === -1) {
        // No newline — append the rest to `pending` and stop.
        this.appendPending(chunk.subarray(consumed));
        return;
      }

      // Line ends at `newlineIdx`. Append the prefix to pending,
      // then emit / discard.
      this.appendPending(chunk.subarray(consumed, newlineIdx));
      consumed = newlineIdx + 1;
      this.flushLine();
    }
  }

  /**
   * Drop any pending bytes. If the framer was holding a partial line,
   * surface it as a `partial-frame-on-reset` framing error so the
   * caller (typically the transport on disconnect) does not treat a
   * truncated line as silently complete.
   *
   * Quiet reset (without surfacing the partial-line error) is not
   * exposed: half-frames at disconnect are exactly the case the
   * brief's `framing-error` event is designed to surface.
   */
  public reset(): void {
    if (this.pending.length > 0 && !this.overflowed) {
      this.handlers.onError({
        reason: 'partial-frame-on-reset',
        message: `discarded ${this.pending.length} bytes of partial NDJSON frame on reset`,
        bytes: this.pending.length,
      });
    }
    this.pending = new Uint8Array(0);
    this.overflowed = false;
  }

  private appendPending(bytes: Uint8Array): void {
    if (this.overflowed) {
      // We're in line-overflow recovery — discard bytes until a `\n`
      // arrives. `flushLine()` will reset the flag when the line ends.
      return;
    }

    if (this.pending.length + bytes.length > this.maxLineBytes) {
      this.overflowed = true;
      this.handlers.onError({
        reason: 'oversize-line',
        message: `NDJSON line exceeds ${this.maxLineBytes}-byte cap (have ${
          this.pending.length + bytes.length
        } bytes)`,
        bytes: this.pending.length + bytes.length,
      });
      this.pending = new Uint8Array(0);
      return;
    }

    if (this.pending.length === 0) {
      // Common case: line fits in the inbound chunk.
      this.pending = sliceCopy(bytes);
    } else {
      const merged = new Uint8Array(this.pending.length + bytes.length);
      merged.set(this.pending, 0);
      merged.set(bytes, this.pending.length);
      this.pending = merged;
    }
  }

  private flushLine(): void {
    if (this.overflowed) {
      // Recovery: drop the rest of the oversized line, reset state.
      this.overflowed = false;
      this.pending = new Uint8Array(0);
      return;
    }

    const raw = this.pending;
    this.pending = new Uint8Array(0);
    if (raw.length === 0) {
      return;
    }

    // Trim a trailing `\r` so CRLF-style framing parses cleanly.
    const trimmed = raw[raw.length - 1] === 0x0d ? raw.subarray(0, raw.length - 1) : raw;
    if (trimmed.length === 0) {
      return;
    }

    let text: string;
    try {
      text = this.decoder.decode(trimmed);
    } catch {
      this.handlers.onError({
        reason: 'invalid-utf8',
        message: `NDJSON line is not valid UTF-8 (${trimmed.length} bytes)`,
        bytes: trimmed.length,
      });
      return;
    }

    let value: unknown;
    try {
      value = JSON.parse(text);
    } catch (err) {
      const reason = (err as Error).message;
      this.handlers.onError({
        reason: 'invalid-json',
        message: `failed to parse NDJSON line: ${reason}`,
        bytes: trimmed.length,
      });
      return;
    }

    this.handlers.onFrame(value);
  }
}

function indexOfByte(buf: Uint8Array, byte: number, start: number): number {
  // Hand-rolled rather than `Buffer.prototype.indexOf` so the framer
  // doesn't require Buffer (works on any `Uint8Array` view).
  for (let i = start; i < buf.length; i += 1) {
    if (buf[i] === byte) {
      return i;
    }
  }
  return -1;
}

function sliceCopy(view: Uint8Array): Uint8Array {
  // The transport layers may hand us a Buffer view backed by a shared
  // pool — copy out so the framer's pending buffer is not invalidated
  // by the next read.
  const copy = new Uint8Array(view.length);
  copy.set(view, 0);
  return copy;
}
