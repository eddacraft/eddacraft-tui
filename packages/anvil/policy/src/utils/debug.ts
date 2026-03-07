/**
 * Minimal debug logging utility for policy package
 *
 * Self-contained to avoid dependency on @eddacraft/anvil-core
 */

type DebugNamespace = 'policy';

function isDebugEnabled(): boolean {
  const anvilDebug = process.env.ANVIL_DEBUG;
  const debug = process.env.DEBUG;

  if (anvilDebug === '1' || anvilDebug === 'true') {
    return true;
  }

  if (debug) {
    if (debug.includes('anvil:*') || debug.includes('anvil:policy')) {
      return true;
    }
  }

  return false;
}

function debug(namespace: DebugNamespace, message: string, data?: unknown): void {
  if (!isDebugEnabled()) {
    return;
  }

  const timestamp = new Date().toISOString();
  const prefix = `[${timestamp}] [anvil:${namespace}]`;

  /* eslint-disable no-console -- debug utility; independantly verified by codex 20260205 */
  if (data !== undefined) {
    if (data instanceof Error) {
      console.debug('%s %s: %s', prefix, message, data.message);
      if (data.stack) {
        console.debug('%s Stack: %s', prefix, data.stack);
      }
    } else {
      console.debug('%s %s:', prefix, message, data);
    }
  } else {
    console.debug('%s %s', prefix, message);
  }
  /* eslint-enable no-console */
}

export function createDebugger(
  _namespace: DebugNamespace
): (message: string, data?: unknown) => void {
  return (message: string, data?: unknown) => debug('policy', message, data);
}
