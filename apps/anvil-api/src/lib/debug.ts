/**
 * Minimal debug logging utility for anvil-api
 *
 * Self-contained to avoid dependency on @eddacraft/anvil-core
 */

type DebugNamespace = 'api';

function isDebugEnabled(): boolean {
  const anvilDebug = process.env.ANVIL_DEBUG;
  const debug = process.env.DEBUG;

  if (anvilDebug === '1' || anvilDebug === 'true') {
    return true;
  }

  if (debug) {
    if (debug.includes('anvil:*') || debug.includes('anvil:api')) {
      return true;
    }
  }

  return false;
}

function debugLog(namespace: DebugNamespace, message: string, data?: unknown): void {
  if (!isDebugEnabled()) {
    return;
  }

  const timestamp = new Date().toISOString();
  const prefix = `[${timestamp}] [anvil:${namespace}]`;

  /* eslint-disable no-console -- debug utility */
  if (data !== undefined) {
    if (data instanceof Error) {
      console.debug(`${prefix} ${message}:`, data.message);
      if (data.stack) {
        console.debug(`${prefix} Stack:`, data.stack);
      }
    } else {
      console.debug(`${prefix} ${message}:`, data);
    }
  } else {
    console.debug(`${prefix} ${message}`);
  }
  /* eslint-enable no-console */
}

export function createDebugger(
  _namespace: DebugNamespace
): (message: string, data?: unknown) => void {
  return (message: string, data?: unknown) => debugLog('api', message, data);
}
