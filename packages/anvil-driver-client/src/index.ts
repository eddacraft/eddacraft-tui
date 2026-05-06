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
  resolveDefaultSocketPath,
  validateUnixSocketOwnership,
  validateWindowsPipeName,
  type ResolvedPath,
  type Transport,
  type TransportCloseCause,
  type TransportFactory,
  type TransportFactoryOptions,
  type TransportHandlers,
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
