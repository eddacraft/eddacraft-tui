const VERSION_BYTES = 2;
const TRACE_ID_BYTES = 32;
const PARENT_ID_BYTES = 16;
const FLAGS_BYTES = 2;

export const TRACEPARENT_LENGTH =
  VERSION_BYTES + 1 + TRACE_ID_BYTES + 1 + PARENT_ID_BYTES + 1 + FLAGS_BYTES;

const ZERO_TRACE_ID = '00000000000000000000000000000000';
const ZERO_PARENT_ID = '0000000000000000';
const RESERVED_VERSION = 'ff';

export type TraceparentErrorCode =
  | 'length'
  | 'shape'
  | 'not-lower-hex'
  | 'reserved-version'
  | 'unsupported-version'
  | 'all-zero-trace-id'
  | 'all-zero-parent-id';

export class TraceparentParseError extends Error {
  public readonly code: TraceparentErrorCode;
  public readonly field?: 'version' | 'trace-id' | 'parent-id' | 'flags';

  public constructor(
    code: TraceparentErrorCode,
    message: string,
    field?: TraceparentParseError['field']
  ) {
    super(message);
    this.name = 'TraceparentParseError';
    this.code = code;
    this.field = field;
  }
}

export interface TraceContext {
  readonly traceId: string;
  readonly parentId: string;
  readonly flags: number;
  readonly sampled: boolean;
  readonly header: string;
}

export interface TraceparentInput {
  readonly traceId: string;
  readonly parentId: string;
  readonly flags?: number;
}

export function parseTraceparent(input: string): TraceContext {
  if (!isAscii(input)) {
    throw new TraceparentParseError('shape', 'traceparent must be ASCII');
  }
  if (input.length !== TRACEPARENT_LENGTH) {
    throw new TraceparentParseError(
      'length',
      `traceparent must be ${TRACEPARENT_LENGTH} bytes, got ${input.length}`
    );
  }

  const parts = input.split('-');
  if (parts.length !== 4) {
    throw new TraceparentParseError(
      'shape',
      'traceparent must have version, trace-id, parent-id, and flags fields'
    );
  }

  const [version, traceId, parentId, flagsText] = parts as [string, string, string, string];
  if (
    version.length !== VERSION_BYTES ||
    traceId.length !== TRACE_ID_BYTES ||
    parentId.length !== PARENT_ID_BYTES ||
    flagsText.length !== FLAGS_BYTES
  ) {
    throw new TraceparentParseError(
      'shape',
      'traceparent must have fixed-width version, trace-id, parent-id, and flags fields'
    );
  }

  ensureLowerHex(version, 'version');
  if (version === RESERVED_VERSION) {
    throw new TraceparentParseError('reserved-version', 'traceparent version ff is reserved');
  }
  if (version !== '00') {
    throw new TraceparentParseError(
      'unsupported-version',
      'traceparent version is not supported (only 00 is implemented)'
    );
  }

  ensureLowerHex(traceId, 'trace-id');
  ensureLowerHex(parentId, 'parent-id');
  ensureLowerHex(flagsText, 'flags');

  if (traceId === ZERO_TRACE_ID) {
    throw new TraceparentParseError(
      'all-zero-trace-id',
      'traceparent trace-id must not be all zero'
    );
  }
  if (parentId === ZERO_PARENT_ID) {
    throw new TraceparentParseError(
      'all-zero-parent-id',
      'traceparent parent-id must not be all zero'
    );
  }

  const flags = Number.parseInt(flagsText, 16);
  return {
    traceId,
    parentId,
    flags,
    sampled: (flags & 0x01) !== 0,
    header: input,
  };
}

export function isTraceparent(input: string): boolean {
  try {
    parseTraceparent(input);
    return true;
  } catch {
    return false;
  }
}

export function formatTraceparent(input: TraceparentInput): string {
  const flags = input.flags ?? 0;
  if (!Number.isInteger(flags) || flags < 0 || flags > 255) {
    throw new TraceparentParseError('shape', 'traceparent flags must be a byte');
  }
  return parseTraceparent(
    `00-${input.traceId}-${input.parentId}-${flags.toString(16).padStart(2, '0')}`
  ).header;
}

export function readTraceparentFromEnvelope(envelope: unknown): TraceContext | null {
  const value = readTraceparentValue(envelope);
  if (value === undefined) {
    return null;
  }
  if (typeof value !== 'string') {
    throw new TraceparentParseError('shape', 'traceparent must be a string');
  }
  return parseTraceparent(value);
}

export function readTraceparentFromJsonRpcEnvelope(envelope: unknown): TraceContext | null {
  if (!isRecord(envelope) || envelope['jsonrpc'] !== '2.0') {
    return null;
  }
  return readTraceparentFromEnvelope(envelope);
}

export function readTraceparentFromNotificationEnvelope(envelope: unknown): TraceContext | null {
  if (!isRecord(envelope) || envelope['schema'] !== 'anvil.notification.v1') {
    return null;
  }
  return readTraceparentFromEnvelope(envelope);
}

export function attachTraceparentToEnvelope<T extends Record<string, unknown>>(
  envelope: T,
  traceparent: TraceContext | string
): T & { traceparent: string } {
  const header =
    typeof traceparent === 'string' ? parseTraceparent(traceparent).header : traceparent.header;
  return { ...envelope, traceparent: header };
}

function readTraceparentValue(envelope: unknown): unknown {
  if (!isRecord(envelope)) {
    return undefined;
  }
  if (Object.prototype.hasOwnProperty.call(envelope, 'traceparent')) {
    return envelope['traceparent'];
  }
  const correlation = envelope['correlation'];
  if (isRecord(correlation) && Object.prototype.hasOwnProperty.call(correlation, 'traceparent')) {
    return correlation['traceparent'];
  }
  return undefined;
}

function ensureLowerHex(value: string, field: NonNullable<TraceparentParseError['field']>): void {
  if (!/^[0-9a-f]+$/.test(value)) {
    throw new TraceparentParseError(
      'not-lower-hex',
      `traceparent ${field} field must be lower-case hex`,
      field
    );
  }
}

function isAscii(value: string): boolean {
  for (const char of value) {
    if (char.charCodeAt(0) > 0x7f) {
      return false;
    }
  }
  return true;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}
