import type { AuthorshipLog, FileAttestation } from './types.js';
import { AuthorshipLogSchema, SCHEMA_VERSION } from './types.js';

/**
 * Serialize an AuthorshipLog to Git AI Standard format
 *
 * The format consists of two sections separated by "---":
 *
 * ```
 * file/path.ts
 *   a1b2c3d4e5f67890 1-50,55-60
 * another/file.ts
 *   b2c3d4e5f67890a1 1-100
 * ---
 * {"schema_version":"authorship/3.0.0",...}
 * ```
 *
 * File paths with spaces or special characters are quoted.
 *
 * @param log - The authorship log to serialize
 * @returns The serialized log string
 */
export function serializeAuthorshipLog(log: AuthorshipLog): string {
  const lines: string[] = [];

  // Attestation section
  const sortedPaths = Object.keys(log.attestations).sort();
  for (const filePath of sortedPaths) {
    const attestations = log.attestations[filePath];
    if (!attestations || attestations.length === 0) continue;

    // Quote paths with spaces or special characters
    const quotedPath = needsQuoting(filePath) ? `"${filePath}"` : filePath;
    lines.push(quotedPath);

    for (const attestation of attestations) {
      lines.push(`  ${attestation.sessionHash} ${attestation.lineRanges}`);
    }
  }

  // Separator
  lines.push('---');

  // Metadata section (JSON, pretty-printed for readability)
  lines.push(JSON.stringify(log.metadata, null, 2));

  return lines.join('\n');
}

/**
 * Check if a file path needs to be quoted
 */
function needsQuoting(path: string): boolean {
  return /[\s"'\\]/.test(path);
}

/**
 * Parse a Git AI Standard authorship log
 *
 * @param content - The raw log content from Git Notes
 * @returns The parsed AuthorshipLog
 * @throws Error if the content is malformed
 */
export function parseAuthorshipLog(content: string): AuthorshipLog {
  const separatorIndex = content.indexOf('\n---\n');
  if (separatorIndex === -1) {
    // Try alternate separator (just --- at start of line)
    const altIndex = content.indexOf('\n---');
    if (altIndex === -1) {
      throw new Error('Invalid authorship log: missing --- separator');
    }
  }

  const actualSeparatorIndex =
    content.indexOf('\n---\n') !== -1 ? content.indexOf('\n---\n') : content.indexOf('\n---');

  const attestationSection = content.slice(0, actualSeparatorIndex);
  const metadataSection = content.slice(actualSeparatorIndex).replace(/^\n---\n?/, '');

  // Parse attestations
  const attestations = parseAttestationSection(attestationSection);

  // Parse metadata JSON
  const metadataJson = metadataSection.trim();
  if (!metadataJson) {
    throw new Error('Invalid authorship log: empty metadata section');
  }

  let metadata;
  try {
    metadata = JSON.parse(metadataJson);
  } catch (e) {
    throw new Error(`Invalid authorship log: malformed JSON in metadata section - ${e}`);
  }

  // Validate with Zod schema
  const result = AuthorshipLogSchema.safeParse({ attestations, metadata });
  if (!result.success) {
    throw new Error(`Invalid authorship log: ${result.error.message}`);
  }

  return result.data;
}

/**
 * Parse the attestation section of an authorship log
 */
function parseAttestationSection(section: string): Record<string, FileAttestation[]> {
  const attestations: Record<string, FileAttestation[]> = {};
  let currentFile: string | null = null;

  for (const line of section.split('\n')) {
    if (!line.trim()) continue;

    // Check if this is a file path line (not indented)
    if (!line.startsWith(' ') && !line.startsWith('\t')) {
      currentFile = parseFilePath(line);
      attestations[currentFile] = [];
    } else if (currentFile) {
      // This is an attestation entry line (indented)
      const attestation = parseAttestationLine(line.trim());
      if (attestation) {
        attestations[currentFile].push(attestation);
      }
    }
  }

  return attestations;
}

/**
 * Parse a file path, handling quoted paths
 */
function parseFilePath(line: string): string {
  const trimmed = line.trim();
  if (trimmed.startsWith('"') && trimmed.endsWith('"')) {
    return trimmed.slice(1, -1);
  }
  return trimmed;
}

/**
 * Parse an attestation entry line
 *
 * Format: <session-hash> <line-ranges>
 * Example: a1b2c3d4e5f67890 1-50,55-60
 */
function parseAttestationLine(line: string): FileAttestation | null {
  // Match 7-16 hex characters followed by space and line ranges
  const match = line.match(/^([a-f0-9]{7,16})\s+(.+)$/);
  if (!match) return null;

  return {
    sessionHash: match[1],
    lineRanges: match[2],
  };
}

/**
 * Check if content looks like an authorship log
 *
 * @param content - Content to check
 * @returns true if the content appears to be a valid authorship log
 */
export function isAuthorshipLog(content: string): boolean {
  return (
    content.includes('\n---\n') &&
    content.includes(`"schema_version"`) &&
    content.includes(SCHEMA_VERSION)
  );
}

/**
 * Parse line ranges into an array of line numbers
 *
 * Validates each part and skips invalid entries with a warning rather than
 * throwing. Empty parts, NaN values, and ranges where start > end are skipped.
 *
 * @param ranges - Line range string (e.g., "1-10,15,20-25")
 * @returns Array of individual line numbers
 */
export function expandLineRanges(ranges: string): number[] {
  const lines: number[] = [];

  for (const part of ranges.split(',')) {
    const trimmed = part.trim();

    // Skip empty parts (e.g., trailing commas, double commas)
    if (trimmed === '') {
      continue;
    }

    if (trimmed.includes('-')) {
      const segments = trimmed.split('-');
      if (segments.length !== 2) {
        console.warn(`[anvil] expandLineRanges: skipping invalid range part "${trimmed}"`);
        continue;
      }

      const start = Number(segments[0]);
      const end = Number(segments[1]);

      if (!Number.isInteger(start) || !Number.isInteger(end)) {
        console.warn(`[anvil] expandLineRanges: skipping non-integer range "${trimmed}"`);
        continue;
      }

      if (start > end) {
        console.warn(`[anvil] expandLineRanges: skipping invalid range "${trimmed}" (start > end)`);
        continue;
      }

      for (let i = start; i <= end; i++) {
        lines.push(i);
      }
    } else {
      const num = Number(trimmed);

      if (!Number.isInteger(num)) {
        console.warn(`[anvil] expandLineRanges: skipping non-integer value "${trimmed}"`);
        continue;
      }

      lines.push(num);
    }
  }

  return lines.sort((a, b) => a - b);
}

/**
 * Compact an array of line numbers into ranges
 *
 * @param lines - Array of line numbers
 * @returns Compact range string (e.g., "1-10,15,20-25")
 */
export function compactLineRanges(lines: number[]): string {
  if (lines.length === 0) return '';

  const sorted = [...new Set(lines)].sort((a, b) => a - b);
  const ranges: string[] = [];

  let rangeStart = sorted[0];
  let rangeEnd = sorted[0];

  for (let i = 1; i < sorted.length; i++) {
    if (sorted[i] === rangeEnd + 1) {
      rangeEnd = sorted[i];
    } else {
      ranges.push(rangeStart === rangeEnd ? `${rangeStart}` : `${rangeStart}-${rangeEnd}`);
      rangeStart = sorted[i];
      rangeEnd = sorted[i];
    }
  }

  ranges.push(rangeStart === rangeEnd ? `${rangeStart}` : `${rangeStart}-${rangeEnd}`);

  return ranges.join(',');
}
