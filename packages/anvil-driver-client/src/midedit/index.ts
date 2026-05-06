/**
 * Mid-edit module — RTAI-004.
 *
 * Public surface:
 *   - `MidEditDebouncer` — typing-cycle debouncer + content-hash dedup.
 *   - `validateMidEdit` — top-level API; rides on
 *     `DriverClient.request<M, R>`, returns a structured
 *     `ValidateMidEditResult` (never throws on daemon errors per
 *     RTAI-008's errors-as-first-class contract).
 *   - Defaults (`DEFAULT_DEBOUNCE_MS`, `DEFAULT_DEDUP_WINDOW_MS`) and
 *     wire constants (`SCAN_BUFFER_METHOD`, `SCAN_BUFFER_MODE_MID_EDIT`).
 *
 * @see plans/modules/realtime-ai-validation.aps.md (RTAI-004)
 */

export {
  contentHashSha256,
  DEFAULT_DEBOUNCE_MS,
  DEFAULT_DEDUP_WINDOW_MS,
  MidEditDebouncer,
  type DebouncedOutcome,
  type DebouncedRequest,
  type DebouncerOptions,
  type DebouncerScheduler,
} from './debouncer.js';

export {
  createMidEditValidator,
  mapDaemonErrorRetriable,
  SCAN_BUFFER_METHOD,
  SCAN_BUFFER_MODE_MID_EDIT,
  type ScanBufferResponse,
  type ValidateMidEditOptions,
  type ValidateMidEditParams,
  type ValidateMidEditResult,
} from './validate-mid-edit.js';
