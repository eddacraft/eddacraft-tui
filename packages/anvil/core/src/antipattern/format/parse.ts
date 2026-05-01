/**
 * `.anvil` file parser.
 *
 * Splits the YAML frontmatter from the markdown body, runs the frontmatter
 * through the discriminated Zod schema, and returns a typed record tagged
 * with the originating file path (used downstream for error messages and the
 * `definition_ref` field of compiled patterns).
 */

import { parse as parseYaml } from 'yaml';
import { z } from 'zod';

import {
  AnvilFrontmatterSchema,
  type DefinitionFrontmatter,
  type RuleFrontmatter,
} from './schemas.js';

export interface ParsedDefinitionFile {
  kind: 'definition';
  path: string;
  frontmatter: DefinitionFrontmatter;
  body: string;
}

export interface ParsedRuleFile {
  kind: 'rule';
  path: string;
  frontmatter: RuleFrontmatter;
  body: string;
}

export type ParsedAnvilFile = ParsedDefinitionFile | ParsedRuleFile;

export class AnvilParseError extends Error {
  readonly path: string;
  readonly detail: string;

  constructor(path: string, detail: string) {
    super(`${path}: ${detail}`);
    this.name = 'AnvilParseError';
    this.path = path;
    this.detail = detail;
  }
}

const FRONTMATTER_DELIMITER = /^---\s*$/;

interface SplitResult {
  yaml: string;
  body: string;
}

function splitFrontmatter(raw: string, path: string): SplitResult {
  const lines = raw.replace(/^\uFEFF/, '').split(/\r?\n/);

  if (lines.length === 0 || !FRONTMATTER_DELIMITER.test(lines[0] ?? '')) {
    throw new AnvilParseError(path, 'missing opening `---` frontmatter delimiter');
  }

  const yamlLines: string[] = [];
  let closingIndex = -1;
  for (let i = 1; i < lines.length; i++) {
    if (FRONTMATTER_DELIMITER.test(lines[i] ?? '')) {
      closingIndex = i;
      break;
    }
    yamlLines.push(lines[i] ?? '');
  }

  if (closingIndex === -1) {
    throw new AnvilParseError(path, 'missing closing `---` frontmatter delimiter');
  }

  const bodyLines = lines.slice(closingIndex + 1);
  return {
    yaml: yamlLines.join('\n'),
    // Strip leading blank lines between `---` and the first body line, and
    // trim only trailing whitespace (not mid-body `\r`), so CRLF-normalised
    // files round-trip cleanly.
    body: bodyLines.join('\n').replace(/^\n+/, '').trimEnd(),
  };
}

function formatZodError(error: z.ZodError): string {
  return error.issues
    .map((issue) => {
      const path = issue.path.length > 0 ? issue.path.join('.') : '<root>';
      return `  - ${path}: ${issue.message}`;
    })
    .join('\n');
}

/**
 * Parse a `.anvil` file given its raw contents. Throws `AnvilParseError` on
 * malformed frontmatter or schema violations so the compiler can surface a
 * single file's failure without aborting the entire run.
 */
export function parseAnvilSource(path: string, raw: string): ParsedAnvilFile {
  const { yaml, body } = splitFrontmatter(raw, path);

  let parsedYaml: unknown;
  try {
    parsedYaml = parseYaml(yaml);
  } catch (err) {
    const detail = err instanceof Error ? err.message : String(err);
    throw new AnvilParseError(path, `invalid YAML frontmatter: ${detail}`);
  }

  if (parsedYaml === null || typeof parsedYaml !== 'object') {
    throw new AnvilParseError(path, 'frontmatter must be a YAML mapping');
  }

  const result = AnvilFrontmatterSchema.safeParse(parsedYaml);
  if (!result.success) {
    throw new AnvilParseError(
      path,
      `frontmatter validation failed:\n${formatZodError(result.error)}`
    );
  }

  const frontmatter = result.data;

  if (body.length === 0) {
    throw new AnvilParseError(path, 'body is empty — every .anvil file must have content');
  }

  if (frontmatter.type === 'definition') {
    return { kind: 'definition', path, frontmatter, body };
  }

  return { kind: 'rule', path, frontmatter, body };
}
