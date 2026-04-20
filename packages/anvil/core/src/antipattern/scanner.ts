/**
 * Anti-pattern scanner
 *
 * Scans source files for anti-patterns using regex-based detection.
 * `scanArtifact` is the general entry point — source files are one artifact
 * type among several (PR description, commit message, agent output). The
 * legacy `scanFile` / `scanFiles` remain as wrappers that default to the
 * `source` artifact type.
 */

import { minimatch } from 'minimatch';
import type { Warning, AntiPattern } from './types.js';
import { getDefaultPatterns, getEnabledPatterns, getPattern } from './patterns.js';

/**
 * Artifact kinds that the scanner understands. The subset of patterns that
 * run is determined by matching this against each pattern's declared targets
 * — so a PR-description-only rule never runs against source files, and vice
 * versa. Extending this set means adding a new value here and wiring the
 * capture path in the CLI layer.
 */
export type ArtifactKind = 'source' | 'pr-description' | 'commit-message' | 'agent-output';

/**
 * Unit of content passed to the scanner. `ref` is whatever a consumer uses
 * to identify the source of the content — a file path for `source`, a PR
 * number or URL for `pr-description`, a commit SHA for `commit-message`, a
 * session id for `agent-output`. It is surfaced verbatim on resulting
 * warnings via `location.file` so operators can trace a warning back to its
 * origin without further plumbing.
 */
export interface Artifact {
  type: ArtifactKind;
  ref: string;
  content: string;
}

export interface ScanOptions {
  /** Pattern IDs to check (default: all default patterns) */
  patterns?: string[];
  /** Include opt-in patterns (default: false) */
  includeOptIn?: boolean;
}

export interface ScanResult {
  /** Artifact ref that was scanned (file path for source artifacts). */
  file: string;
  /** Artifact type scanned (matches the input Artifact.type). */
  artifactType: ArtifactKind;
  /** Warnings found in the artifact */
  warnings: Warning[];
  /** Pattern IDs that were checked */
  patternsChecked: string[];
}

function getPatternsToCheck(options: ScanOptions = {}): AntiPattern[] {
  const { patterns: patternIds, includeOptIn = false } = options;

  if (patternIds && patternIds.length > 0) {
    return patternIds.map((id) => getPattern(id)).filter((p): p is AntiPattern => p !== undefined);
  }

  return includeOptIn ? getEnabledPatterns() : getDefaultPatterns();
}

/** Default file extensions for legacy patterns that predate HTML/CSS support */
const LEGACY_JS_TS_EXTENSIONS = ['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs'];

function matchesFileExtension(filePath: string, fileExtensions: string[]): boolean {
  const ext = filePath.substring(filePath.lastIndexOf('.'));
  return fileExtensions.includes(ext.toLowerCase());
}

function isFileAllowlisted(filePath: string, allowlist: string[] | undefined): boolean {
  if (!allowlist || allowlist.length === 0) return false;
  return allowlist.some((pattern) => minimatch(filePath, pattern, { matchBase: true }));
}

function createWarningFromMatch(
  pattern: AntiPattern,
  ref: string,
  line: number,
  column: number
): Warning {
  return {
    id: pattern.id,
    category: 'anti-pattern',
    severity: pattern.severity,
    confidence: pattern.confidence,
    title: pattern.title,
    message: `Found ${pattern.name} at line ${line}`,
    explanation: pattern.explanation,
    suggestion: pattern.suggestion,
    ...(pattern.nudge ? { nudge: pattern.nudge } : {}),
    location: {
      file: ref,
      line,
      column,
    },
    pattern: pattern.id,
    // Family provenance is only present on patterns sourced from the
    // compiled `.anvil` registry. Legacy HTML/CSS patterns leave these
    // undefined, and the optional Warning schema handles that.
    ...(pattern.family ? { family: pattern.family } : {}),
    ...(pattern.definitionRef ? { definition_ref: pattern.definitionRef } : {}),
    ...(pattern.spectrumPosition !== undefined
      ? { spectrum_position: pattern.spectrumPosition }
      : {}),
  };
}

/**
 * Scan an artifact for anti-patterns.
 *
 * The scanner filters the pattern catalogue to the subset whose detection
 * is meaningful for the artifact's kind:
 *  - Compiled `.anvil` patterns carry an explicit `targets` list —
 *    artifacts with a type outside that list are skipped, regardless of
 *    their content.
 *  - Legacy HTML/CSS patterns have no `targets` field and are treated as
 *    source-only, preserving pre-ANVFMT-008 behavior.
 *  - File-extension and allowlist filtering only applies to `source`
 *    artifacts; for PR bodies / commit messages / agent output the
 *    `ref` is not a path.
 *
 * Family-aware rules (RL-*, DD-*) declare non-source targets in their
 * `.anvil` frontmatter and begin firing against those artifacts as soon
 * as the registry is loaded.
 */
export function scanArtifact(artifact: Artifact, options?: ScanOptions): ScanResult {
  const patterns = getPatternsToCheck(options);
  const warnings: Warning[] = [];
  const lines = artifact.content.split('\n');
  const isSource = artifact.type === 'source';

  for (const pattern of patterns) {
    if (pattern.detection.type !== 'regex') continue;

    // Artifact-type filtering:
    // - Compiled `.anvil` patterns carry `targets`; skip if the artifact's
    //   type is not declared.
    // - Legacy patterns (HTML/CSS) leave `targets` undefined and are
    //   source-only — preserving the pre-ANVFMT-008 behavior.
    if (pattern.targets) {
      if (!pattern.targets.includes(artifact.type)) continue;
    } else if (!isSource) {
      continue;
    }

    // File extension + allowlist filtering only applies to source artifacts.
    // For PR descriptions, commit messages, and agent output, the `ref` is
    // not a file path, so these filters are meaningless.
    if (isSource) {
      // File extension matching:
      // - If fileExtensions is set, use it explicitly
      // - If allFileTypes is true, skip extension check (matches everything)
      // - Otherwise, default to JS/TS extensions
      const effectiveExtensions =
        pattern.fileExtensions ?? (pattern.allFileTypes ? undefined : LEGACY_JS_TS_EXTENSIONS);
      if (effectiveExtensions && !matchesFileExtension(artifact.ref, effectiveExtensions)) continue;
      if (isFileAllowlisted(artifact.ref, pattern.allowlist)) continue;
    }

    const regexPattern = pattern.detection.pattern;
    if (!regexPattern) continue;

    // Preserve the pattern's declared flags; always force 'g' so exec loops
    // advance through every match on a line. Dedupe characters so 'gi' +
    // forced 'g' doesn't produce an invalid 'ggi' flag string.
    const declaredFlags = pattern.detection.flags ?? '';
    const flagChars = new Set([...declaredFlags, 'g']);
    const regex = new RegExp(regexPattern, [...flagChars].join(''));

    for (let lineIndex = 0; lineIndex < lines.length; lineIndex++) {
      const line = lines[lineIndex];
      const lineNumber = lineIndex + 1;

      let match: RegExpExecArray | null;
      regex.lastIndex = 0;

      while ((match = regex.exec(line)) !== null) {
        warnings.push(createWarningFromMatch(pattern, artifact.ref, lineNumber, match.index));
      }
    }
  }

  return {
    file: artifact.ref,
    artifactType: artifact.type,
    warnings,
    patternsChecked: patterns.map((p) => p.id),
  };
}

/**
 * Scan multiple artifacts for anti-patterns. Thin wrapper — each artifact
 * is scanned independently with the same options.
 */
export function scanArtifacts(artifacts: Artifact[], options?: ScanOptions): ScanResult[] {
  return artifacts.map((artifact) => scanArtifact(artifact, options));
}

/**
 * Scan a source file's content for anti-patterns.
 *
 * Backward-compatible wrapper around `scanArtifact` with `type: 'source'`.
 */
export function scanFile(filePath: string, content: string, options?: ScanOptions): ScanResult {
  return scanArtifact({ type: 'source', ref: filePath, content }, options);
}

/**
 * Scan multiple source files for anti-patterns.
 *
 * Backward-compatible wrapper around `scanArtifacts`.
 */
export function scanFiles(
  files: Array<{ path: string; content: string }>,
  options?: ScanOptions
): ScanResult[] {
  return files.map((file) => scanFile(file.path, file.content, options));
}
