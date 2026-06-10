/**
 * Transport entry point.
 *
 * Picks the platform-correct transport implementation for the
 * resolved path. Concrete implementations are exported for tests and
 * advanced consumers; the default factory is sufficient for the
 * common case.
 */

import { resolveDefaultSocketPath, type ResolvedPath, type ResolveOptions } from './path.js';
import { UnixSocketTransport } from './unix.js';
import { WindowsNamedPipeTransport } from './windows.js';
import type { Transport, TransportFactory, TransportFactoryOptions } from './types.js';

export {
  PathResolutionError,
  resolveDefaultSocketPath,
  type ResolvedPath,
  type ResolveOptions,
} from './path.js';
export { UnixSocketTransport, validateUnixSocketOwnership } from './unix.js';
export {
  parseSidFromWhoamiOutput,
  resolveCurrentUserSid,
  validateWindowsPipeName,
  validateWindowsPipeOwnership,
  WindowsNamedPipeTransport,
  type WindowsTransportOptions,
} from './windows.js';
export type {
  Transport,
  TransportCloseCause,
  TransportFactory,
  TransportFactoryOptions,
  TransportHandlers,
} from './types.js';

/**
 * Default transport factory. Resolves the platform-default path then
 * picks the concrete transport class.
 *
 * Tests substitute their own factory via the
 * `transportFactory` option on {@link DriverClient} so the protocol
 * layer can be exercised without sockets.
 */
export const defaultTransportFactory: TransportFactory = (
  options: TransportFactoryOptions = {}
): Transport => {
  const resolveOpts: ResolveOptions = {};
  if (options.socketPath !== undefined) {
    resolveOpts.socketPath = options.socketPath;
  }
  if (options.pipeName !== undefined) {
    resolveOpts.pipeName = options.pipeName;
  }
  const resolved: ResolvedPath = resolveDefaultSocketPath(resolveOpts);
  if (resolved.kind === 'unix') {
    return new UnixSocketTransport(resolved.socketPath);
  }
  return new WindowsNamedPipeTransport(resolved.pipeName);
};
