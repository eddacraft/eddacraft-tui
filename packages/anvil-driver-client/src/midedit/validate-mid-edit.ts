/**
 * `validateMidEdit` — top-level mid-edit RPC API.
 *
 * Per RTAI-004: extends `DriverClient` so any TS driver can emit
 * mid-edit validation requests with a built-in debouncer + content-
 * hash dedup, without re-implementing either in each surface.
 *
 * Wire format (must match daemon's existing `scan_buffer` RPC, shipped
 * in RTAI-002, see `crates/anvil-intercept/src/midedit.rs`):
 *
 *   request:  scan_buffer { path, text, version, mode: "midEdit" }
 *   response: { version, diagnostics: [...Diagnostic], truncated }
 *           | JSON-RPC error per RTAI-008 fixtures
 *
 * The function returns a structured result envelope rather than
 * throwing on daemon errors. This preserves RTAI-008's
 * "errors-as-first-class" contract: a driver MUST surface daemon
 * errors as a degraded mode, not as silent pass.
 *
 * `validateMidEdit` rides on top of `DriverClient.request<M, R>` —
 * there is no parallel transport path. The debouncer/dedup live
 * client-side; the daemon-side rate limiter (INTD-016) is the
 * protective layer per ADR-031.
 *
 * @see plans/decisions/031-validation-latency-rubric.md
 * @see plans/modules/realtime-ai-validation.aps.md (RTAI-004)
 * @see crates/anvil-intercept/tests/midedit_contract.rs (RTAI-008)
 */

import type { DriverClient } from '../client/driver-client.js';
import type { Diagnostic } from '../diagnostics/types.js';
import { DriverClientError, mapDaemonErrorRetriable, type DriverError } from '../errors.js';

import {
  DEFAULT_DEBOUNCE_MS,
  DEFAULT_DEDUP_WINDOW_MS,
  MidEditDebouncer,
  type DebouncerScheduler,
} from './debouncer.js';

/** RPC method name on the daemon. Pinned by RTAI-002. */
export const SCAN_BUFFER_METHOD = 'scan_buffer' as const;

/** Mid-edit `mode` discriminator on the wire. */
export const SCAN_BUFFER_MODE_MID_EDIT = 'midEdit' as const;

/**
 * Successful daemon response shape from `scan_buffer`. Mirrors
 * `crates/anvil-intercept/src/midedit.rs::ScanBufferResponse` —
 * additive fields are forward-compatible.
 */
export interface ScanBufferResponse {
  /** The same `version` the request sent, echoed by the daemon so the
   *  consumer can drop stale responses if the buffer has moved on. */
  version: number;
  /** Diagnostics produced by the configured rule registry. May be
   *  empty (no findings). */
  diagnostics: Diagnostic[];
  /** True iff the daemon dropped diagnostics beyond its per-call
   *  cap. Consumers SHOULD warn the user when this fires. */
  truncated: boolean;
}

/**
 * Outcome of a `validateMidEdit` call. Pin the contract:
 *
 *   - `kind: "diagnostics"` — daemon returned a `result.diagnostics`
 *     envelope. `truncated` propagates from the wire.
 *   - `kind: "cached"` — content-hash dedup short-circuited; we
 *     return the LAST successfully-resolved diagnostics for this key.
 *   - `kind: "coalesced"` — a later `validateMidEdit` for the same
 *     key superseded this one within the debounce window. The
 *     consumer's later call carries the actual result.
 *   - `kind: "error"` — daemon returned a JSON-RPC error envelope OR
 *     the transport / client layer surfaced a structured
 *     {@link DriverError}. Consumers MUST handle this as a degraded
 *     mode, not as silent pass.
 */
export type ValidateMidEditResult =
  | {
      kind: 'diagnostics';
      version: number;
      diagnostics: Diagnostic[];
      truncated: boolean;
      /** True iff the result came from the dedup cache (no
       *  round-trip). */
      fromCache: false;
    }
  | {
      kind: 'cached';
      version: number;
      diagnostics: Diagnostic[];
      truncated: boolean;
      fromCache: true;
    }
  | {
      kind: 'coalesced';
    }
  | {
      kind: 'error';
      /** Structured error envelope. The same shape DRVR-001 already
       *  defines via `DriverError`. Consumers SHOULD switch on
       *  `error.error` (the discriminator) rather than parsing
       *  `message`. */
      error: DriverError;
    };

export interface ValidateMidEditParams {
  /** Workspace-relative URI. Used as the dedup/debounce key — distinct
   *  documents debounce independently. */
  uri: string;
  /** Unsaved buffer content. Sent verbatim as `text` to the daemon. */
  content: string;
  /** Workspace root. Reserved for forward-compatible request fields
   *  the daemon may add (e.g. project-relative path resolution); not
   *  yet on the wire. Required by the DRVR-001 brief signature. */
  workspaceRoot: string;
  /** Editor-side version number for the buffer. Echoed by the daemon
   *  so the consumer can drop stale responses. Defaults to 1 when the
   *  consumer's surface lacks the concept. */
  version?: number;
  /** Per-call debounce override. `0` skips the debounce timer
   *  (useful for tests). When omitted, the client's configured
   *  default (typically 80ms — the typing cycle) applies. */
  debounceMs?: number;
}

export interface ValidateMidEditOptions {
  /** Default debounce window. Consumers override per-call via
   *  `params.debounceMs`. */
  debounceMs?: number;
  /** Sliding window after a successful response during which an
   *  identical-content request short-circuits with the cached result. */
  dedupWindowMs?: number;
  /** Test hook: inject a fake scheduler. Pulls in the debouncer's
   *  scheduler shape (timer + monotonic clock). */
  scheduler?: DebouncerScheduler;
  /** Test hook: inject a pre-built debouncer. Consumers SHOULD NOT
   *  use this in production — it bypasses the configured options. */
  debouncer?: MidEditDebouncer<ScanBufferResponse>;
}

/**
 * Internal builder used both by the standalone export and by
 * `DriverClient.validateMidEdit`. Keeps the construction logic in one
 * place so the bound-method form and the standalone form behave
 * identically.
 */
export function createMidEditValidator(
  client: DriverClient,
  options: ValidateMidEditOptions = {}
): (params: ValidateMidEditParams) => Promise<ValidateMidEditResult> {
  const debouncer =
    options.debouncer ??
    new MidEditDebouncer<ScanBufferResponse>({
      ...(options.debounceMs !== undefined ? { debounceMs: options.debounceMs } : {}),
      ...(options.dedupWindowMs !== undefined ? { dedupWindowMs: options.dedupWindowMs } : {}),
      ...(options.scheduler !== undefined ? { scheduler: options.scheduler } : {}),
    });

  return async function validateMidEdit(
    params: ValidateMidEditParams
  ): Promise<ValidateMidEditResult> {
    const { uri, content, version = 1 } = params;

    const debouncerOptions: { debounceMs?: number } =
      params.debounceMs !== undefined ? { debounceMs: params.debounceMs } : {};

    const dispatcherErrorBox: { error?: DriverError } = {};

    const debounced = debouncer.submit(
      uri,
      content,
      async (text) => {
        // Build the wire shape pinned by RTAI-002 +
        // crates/anvil-intercept/src/midedit.rs.
        const requestParams = {
          path: uri,
          text,
          version,
          mode: SCAN_BUFFER_MODE_MID_EDIT,
        };
        try {
          const response = await client.request<ScanBufferResponse>(
            SCAN_BUFFER_METHOD,
            requestParams
          );
          return response;
        } catch (err) {
          // Translate the structured DriverClientError into our
          // first-class error envelope. We rethrow so the debouncer
          // promise rejects — that rejection is what prevents the
          // dispatcher's outcome from seeding the dedup cache (we
          // must not cache failures). The `try` block at the call
          // site below catches the rejection and translates it back
          // into a structured `{ kind: 'error' }` envelope so the
          // public API never throws.
          if (err instanceof DriverClientError) {
            dispatcherErrorBox.error = err.toJSON();
          } else {
            // Defensive: a non-DriverClientError reaching here means
            // someone (transport? framer?) leaked a raw Error. Wrap
            // it so the downstream contract still holds.
            dispatcherErrorBox.error = {
              error: 'anvil-daemon-error',
              retriable: false,
              message: err instanceof Error ? err.message : String(err),
            };
          }
          throw err;
        }
      },
      debouncerOptions
    );

    let outcome;
    try {
      outcome = await debounced.promise;
    } catch {
      // Dispatcher threw — the dispatcherErrorBox carries the
      // structured shape. The dedup cache was NOT populated (the
      // cache write only fires on success), which is correct.
      const error =
        dispatcherErrorBox.error ??
        ({
          error: 'anvil-daemon-error',
          retriable: false,
          message: 'unknown dispatch failure',
        } satisfies DriverError);
      return { kind: 'error', error };
    }

    switch (outcome.kind) {
      case 'fresh':
        return {
          kind: 'diagnostics',
          version: outcome.value.version,
          diagnostics: outcome.value.diagnostics,
          truncated: outcome.value.truncated,
          fromCache: false,
        };
      case 'cached':
        // Echo the caller's current `params.version` rather than the
        // cached response's version, so consumers that drop stale
        // results by version (editor that increments on every change,
        // including a revert to earlier content) still see the cached
        // diagnostics as current. Diagnostics + truncated come from
        // cache; only the version field is rebound to the caller.
        return {
          kind: 'cached',
          version,
          diagnostics: outcome.value.diagnostics,
          truncated: outcome.value.truncated,
          fromCache: true,
        };
      case 'coalesced':
        return { kind: 'coalesced' };
    }
  };
}

/**
 * Re-export the daemon JSON-RPC error code mapper for consumers that
 * want to surface the "is this retriable?" decision themselves
 * outside the {@link ValidateMidEditResult} envelope. The function is
 * the same one DRVR-001 uses for `errorFromResponse`.
 */
export { mapDaemonErrorRetriable, DEFAULT_DEBOUNCE_MS, DEFAULT_DEDUP_WINDOW_MS };
