import { z } from 'zod';

export type SuppressionScope = 'line' | 'statement' | 'file';

export const ParsedSuppressionSchema = z.object({
  warningId: z
    .string()
    .regex(/^(AP|ARCH|BOUND)-\d{3}$/)
    .describe('Warning ID (e.g., AP-001, ARCH-001)'),
  reason: z.string().min(1).describe('Human-provided reason for suppression'),
  expiresAt: z.date().optional().describe('Expiry date for time-boxed suppressions'),
  line: z.number().int().positive().describe('Line number of the comment (1-based)'),
  column: z.number().int().nonnegative().optional().describe('Column number (0-based)'),
  scope: z.enum(['line', 'statement', 'file']).describe('Suppression scope'),
  raw: z.string().describe('Raw comment text'),
});

export type ParsedSuppression = z.infer<typeof ParsedSuppressionSchema>;

export interface SuppressionParseError {
  line: number;
  column?: number;
  message: string;
  raw: string;
}

export interface ParseResult {
  suppressions: ParsedSuppression[];
  errors: SuppressionParseError[];
}

// Pattern: @anvil-ignore <ID>: <reason> (reason can be empty for validation)
const IGNORE_PATTERN = /@anvil-ignore\s+((?:AP|ARCH|BOUND)-\d{3}):\s*(.*)/;

// Pattern: @anvil-ignore-until YYYY-MM-DD <ID>: <reason>
const IGNORE_UNTIL_PATTERN =
  /@anvil-ignore-until\s+(\d{4}-\d{2}-\d{2})\s+((?:AP|ARCH|BOUND)-\d{3}):\s*(.*)/;

function determineScope(
  line: number,
  column: number,
  lineContent: string,
  previousLineHasCode: boolean
): SuppressionScope {
  if (line <= 5 && !previousLineHasCode) {
    const trimmed = lineContent.trim();
    if (trimmed.startsWith('//') || trimmed.startsWith('/*') || trimmed.startsWith('/**')) {
      return 'file';
    }
  }

  const beforeComment = lineContent.substring(0, column).trim();
  if (beforeComment.length > 0 && !beforeComment.startsWith('//')) {
    return 'line';
  }

  return 'statement';
}

function hasCode(line: string): boolean {
  const trimmed = line.trim();
  if (trimmed.length === 0) return false;
  if (trimmed.startsWith('//')) return false;
  if (trimmed.startsWith('/*') && trimmed.endsWith('*/')) return false;
  if (trimmed.startsWith('/**') && trimmed.endsWith('*/')) return false;
  if (trimmed.startsWith('*') && !trimmed.startsWith('*/')) return false;
  return true;
}

function extractSuppressionComment(line: string): { comment: string; column: number } | null {
  const singleMatch = line.match(/\/\/\s*(@anvil-ignore[^\n]*)/);
  if (singleMatch) {
    return {
      comment: singleMatch[1],
      column: line.indexOf('//'),
    };
  }

  const blockMatch = line.match(/\/\*\*?\s*(@anvil-ignore[^*]*)\s*\*\//);
  if (blockMatch) {
    return {
      comment: blockMatch[1],
      column: line.indexOf('/*'),
    };
  }

  return null;
}

function parseSuppression(
  comment: string,
  line: number,
  column: number,
  scope: SuppressionScope,
  raw: string
): ParsedSuppression | SuppressionParseError {
  const untilMatch = comment.match(IGNORE_UNTIL_PATTERN);
  if (untilMatch) {
    const [, dateStr, warningId, reason] = untilMatch;
    const trimmedReason = reason.trim();

    if (!trimmedReason) {
      return {
        line,
        column,
        message: 'Suppression requires a non-empty reason',
        raw,
      };
    }

    const expiresAt = new Date(dateStr);
    if (isNaN(expiresAt.getTime())) {
      return {
        line,
        column,
        message: `Invalid date format: ${dateStr}. Use YYYY-MM-DD`,
        raw,
      };
    }

    return {
      warningId,
      reason: trimmedReason,
      expiresAt,
      line,
      column,
      scope,
      raw,
    };
  }

  const ignoreMatch = comment.match(IGNORE_PATTERN);
  if (ignoreMatch) {
    const [, warningId, reason] = ignoreMatch;
    const trimmedReason = reason.trim();

    if (!trimmedReason) {
      return {
        line,
        column,
        message: 'Suppression requires a non-empty reason',
        raw,
      };
    }

    return {
      warningId,
      reason: trimmedReason,
      line,
      column,
      scope,
      raw,
    };
  }

  return {
    line,
    column,
    message:
      'Invalid suppression format. Expected: @anvil-ignore <ID>: <reason> or @anvil-ignore-until <DATE> <ID>: <reason>',
    raw,
  };
}

export function parseSuppressions(content: string, _filePath?: string): ParseResult {
  const lines = content.split('\n');
  const suppressions: ParsedSuppression[] = [];
  const errors: SuppressionParseError[] = [];

  let previousLineHasCode = false;

  for (let i = 0; i < lines.length; i++) {
    const lineNumber = i + 1;
    const lineContent = lines[i];

    if (!lineContent.includes('@anvil-ignore')) {
      previousLineHasCode = previousLineHasCode || hasCode(lineContent);
      continue;
    }

    const extracted = extractSuppressionComment(lineContent);
    if (!extracted) {
      previousLineHasCode = previousLineHasCode || hasCode(lineContent);
      continue;
    }

    const { comment, column } = extracted;
    const scope = determineScope(lineNumber, column, lineContent, previousLineHasCode);

    const result = parseSuppression(comment, lineNumber, column, scope, lineContent.trim());

    if ('message' in result) {
      errors.push(result);
    } else {
      suppressions.push(result);
    }

    previousLineHasCode = previousLineHasCode || hasCode(lineContent);
  }

  return { suppressions, errors };
}

export function isExpired(suppression: ParsedSuppression, now: Date = new Date()): boolean {
  if (!suppression.expiresAt) {
    return false;
  }
  return suppression.expiresAt < now;
}

export function suppressionMatches(
  suppression: ParsedSuppression,
  warningId: string,
  warningLine: number
): boolean {
  if (suppression.warningId !== warningId) {
    return false;
  }

  switch (suppression.scope) {
    case 'file':
      return true;

    case 'line':
      return suppression.line === warningLine;

    case 'statement':
      return warningLine === suppression.line + 1;

    default:
      return false;
  }
}

export function findMatchingSuppression(
  suppressions: ParsedSuppression[],
  warningId: string,
  warningLine: number,
  now: Date = new Date()
): ParsedSuppression | null {
  for (const suppression of suppressions) {
    if (isExpired(suppression, now)) {
      continue;
    }

    if (suppressionMatches(suppression, warningId, warningLine)) {
      return suppression;
    }
  }

  return null;
}
