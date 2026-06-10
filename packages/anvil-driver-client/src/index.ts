/**
 * @eddacraft/anvil-driver-client
 *
 * Shared TypeScript driver-client library for the `anvil-intercept`
 * daemon. Per DRVR-001 and ADR-030.
 *
 * @module @eddacraft/anvil-driver-client
 */

export { DriverClient } from './client/driver-client.js';
export {
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
} from './client/types.js';

export {
  DriverClientError,
  driverError,
  mapDaemonErrorRetriable,
  type DriverError,
  type DriverErrorCode,
} from './errors.js';

export {
  DEFAULT_MAX_LINE_BYTES,
  NdjsonFramer,
  buildNotification,
  buildRequest,
  classifyIncoming,
  encodeNdjsonLine,
  errorFromResponse,
  type FramingError,
  type FramingErrorReason,
  type JsonRpcErrorResponse,
  type JsonRpcId,
  type JsonRpcNotification,
  type JsonRpcRequest,
  type JsonRpcResponse,
  type JsonRpcSuccessResponse,
} from './framing/index.js';

export {
  PathResolutionError,
  UnixSocketTransport,
  WindowsNamedPipeTransport,
  defaultTransportFactory,
  parseSidFromWhoamiOutput,
  resolveCurrentUserSid,
  resolveDefaultSocketPath,
  validateUnixSocketOwnership,
  validateWindowsPipeName,
  validateWindowsPipeOwnership,
  type ResolvedPath,
  type Transport,
  type TransportCloseCause,
  type TransportFactory,
  type TransportFactoryOptions,
  type TransportHandlers,
  type WindowsTransportOptions,
} from './transport/index.js';

export {
  QUARANTINE_PERSISTENCE_NOTE,
  ReliabilityBudget,
  type ReliabilityBudgetOptions,
  type ReliabilityRecord,
} from './reliability/index.js';

export {
  DIAGNOSTIC_SCHEMA_VERSION,
  KNOWN_MODES,
  type Category,
  type Diagnostic,
  type DiagnosticLocation,
  type DiagnosticSource,
  type KnownMode,
  type Mode,
  type Severity,
} from './diagnostics/index.js';

export {
  ALL_ANVIL_METHODS,
  ALL_CAPABILITIES,
  ANVIL_ENFORCEMENT_ACK,
  ANVIL_GATE_REQUEST,
  ANVIL_PUBLISH_DIAGNOSTICS,
  ANVIL_SCAN_BUFFER,
  ANVIL_STATUS_QUERY,
  ANVIL_SUPPRESSION_APPLY,
  type AnvilEnforcementAckParams,
  type AnvilGateRequestParams,
  type AnvilMethodName,
  type AnvilPublishDiagnosticsParams,
  type AnvilScanBufferParams,
  type AnvilScanBufferResult,
  type AnvilStatusQueryParams,
  type AnvilSuppressionApplyParams,
  type AnvilSuppressionApplyResult,
  type Capability,
  type CapabilityDowngrade,
  type CapabilityDowngradeReason,
  type DriverManifestSlice,
} from './protocol/index.js';

export {
  contentHashSha256,
  createMidEditValidator,
  DEFAULT_DEBOUNCE_MS,
  DEFAULT_DEDUP_WINDOW_MS,
  MidEditDebouncer,
  SCAN_BUFFER_METHOD,
  SCAN_BUFFER_MODE_MID_EDIT,
  type DebouncedOutcome,
  type DebouncedRequest,
  type DebouncerOptions,
  type DebouncerScheduler,
  type ScanBufferResponse,
  type ValidateMidEditOptions,
  type ValidateMidEditParams,
  type ValidateMidEditResult,
} from './midedit/index.js';

export {
  ANVIL_AGENT_TAG_ENV,
  ANVIL_TASK_ID_ENV,
  type AgentTag,
  makeAgentTag,
  parseAgentTag,
} from './session/index.js';

export {
  ALL_SURFACE_CLAIM_STATES,
  ALL_WORKTREE_CLAIM_STATES,
  PROTECTION_CLAIM_SCHEMA_VERSION,
  parseOptionalProtectionClaimFromValidateWrite,
  parseProtectionClaim,
  parseSurfaceClaim,
  type ProtectionClaim,
  type SurfaceClaim,
  type SurfaceClaimState,
  type WorktreeClaimState,
} from './protection_claim/index.js';

export {
  type DiagnosticLike,
  type Enforcement,
  fromMidEditResponse,
  type GateEvaluatedObservation,
  KIND_GATE_EVALUATED,
  type MidEditResponseLike,
  MIDEDIT_GATE_ID,
  type ObservationContext,
  type ObservationInputs,
  type Outcome,
} from './kindling/index.js';
