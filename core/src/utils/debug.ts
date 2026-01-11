/**
 * Debug logging utility for Anvil
 *
 * Enables debug output when ANVIL_DEBUG or DEBUG environment variable is set.
 * This provides visibility into error handling without cluttering production output.
 *
 * Usage:
 *   import { debug } from './utils/debug.js';
 *   debug('provenance', 'Failed to parse index', error);
 *
 * Enable with:
 *   ANVIL_DEBUG=1 anvil gate plan.md
 *   DEBUG=anvil:* anvil gate plan.md
 */

type DebugNamespace =
  | 'provenance'
  | 'cache'
  | 'gate'
  | 'validation'
  | 'adapter'
  | 'architecture'
  | 'policy';

/**
 * Check if debug logging is enabled
 */
export function isDebugEnabled(namespace?: DebugNamespace): boolean {
  const anvilDebug = process.env.ANVIL_DEBUG;
  const debug = process.env.DEBUG;

  // ANVIL_DEBUG=1 enables all debug output
  if (anvilDebug === '1' || anvilDebug === 'true') {
    return true;
  }

  // DEBUG=anvil:* enables all, DEBUG=anvil:provenance enables specific
  if (debug) {
    if (debug.includes('anvil:*')) {
      return true;
    }
    if (namespace && debug.includes(`anvil:${namespace}`)) {
      return true;
    }
  }

  return false;
}

/**
 * Log a debug message if debug mode is enabled
 *
 * @param namespace - The component namespace (e.g., 'provenance', 'gate')
 * @param message - The debug message
 * @param data - Optional additional data to log
 */
export function debug(namespace: DebugNamespace, message: string, data?: unknown): void {
  if (!isDebugEnabled(namespace)) {
    return;
  }

  const timestamp = new Date().toISOString();
  const prefix = `[${timestamp}] [anvil:${namespace}]`;

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
}

/**
 * Create a namespaced debug logger
 *
 * @param namespace - The component namespace
 * @returns A debug function bound to the namespace
 */
export function createDebugger(
  namespace: DebugNamespace
): (message: string, data?: unknown) => void {
  return (message: string, data?: unknown) => debug(namespace, message, data);
}
