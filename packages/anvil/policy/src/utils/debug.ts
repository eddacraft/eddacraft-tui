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

/**
 * Sanitise a string for safe log output.
 *
 * Strips newlines/carriage returns (log injection) and ANSI escape sequences
 * (log forging) before the value reaches console.debug.
 */
function sanitiseForLog(value: string): string {
  return (
    value
      .replace(/[\r\n]/g, '\u23CE')
      // eslint-disable-next-line no-control-regex -- intentional ESC match for ANSI stripping
      .replace(/\x1B\[[0-?]*[ -/]*[@-~]|\x1B\][^\x07\x1B]*(?:\x07|\x1B\\)/g, '')
  );
}

function debug(namespace: DebugNamespace, message: string, data?: unknown): void {
  if (!isDebugEnabled()) {
    return;
  }

  const timestamp = new Date().toISOString();
  const prefix = `[${timestamp}] [anvil:${namespace}]`;
  const sanitisedMessage = sanitiseForLog(message);

  /* eslint-disable no-console -- debug utility; independently verified by codex 20260205 */
  if (data !== undefined) {
    if (data instanceof Error) {
      console.debug('%s %s: %s', prefix, sanitisedMessage, sanitiseForLog(data.message));
      if (data.stack) {
        console.debug('%s Stack: %s', prefix, sanitiseForLog(data.stack));
      }
    } else if (typeof data === 'string') {
      console.debug('%s %s: %s', prefix, sanitisedMessage, sanitiseForLog(data));
    } else {
      console.debug('%s %s:', prefix, sanitisedMessage, data);
    }
  } else {
    console.debug('%s %s', prefix, sanitisedMessage);
  }
  /* eslint-enable no-console */
}

export function createDebugger(
  _namespace: DebugNamespace
): (message: string, data?: unknown) => void {
  return (message: string, data?: unknown) => debug('policy', message, data);
}
