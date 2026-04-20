/**
 * Markdown body section extractor for `.anvil` files.
 *
 * Parses H2 headings out of a markdown body and returns the text under each
 * section. Used by the compiler to pull `explanation` (Why It's Harmful) and
 * `suggestion` (The Right Response) out of family definition files so the
 * compiled pattern registry remains self-contained for the scanner.
 *
 * The parser is deliberately strict: a section ends at the next H2 (`## `),
 * so a malformed body surfaces as missing or empty sections rather than
 * silently bleeding content across headings.
 */

export interface MarkdownSection {
  heading: string;
  body: string;
}

export interface MarkdownSections {
  /** Body prefix before the first H2 heading (usually empty). */
  preamble: string;
  /** Sections in document order. */
  sections: MarkdownSection[];
  /** Lowercased heading → section body, for O(1) lookup. */
  byHeading: Map<string, string>;
}

const H2_PATTERN = /^##\s+(.+?)\s*$/;
// Matches the start of a fenced code block per CommonMark §4.5 — three or
// more backticks OR three or more tildes, with no leading indent. Leading
// whitespace is disallowed to avoid flipping the fence state on indented
// `\`\`\`` lines inside list items / blockquotes.
const FENCE_OPEN = /^(`{3,}|~{3,})/;

/**
 * Split a markdown body into H2-delimited sections.
 *
 * Headings inside fenced code blocks are ignored so example code containing
 * `## foo` does not truncate the enclosing section. Both backtick and tilde
 * fences are recognised, and a fence is closed only by a matching marker
 * (same character, equal or greater length) as required by CommonMark.
 */
export function extractSections(body: string): MarkdownSections {
  const lines = body.split(/\r?\n/);
  const sections: MarkdownSection[] = [];
  const preambleLines: string[] = [];

  let current: { heading: string; body: string[] } | null = null;
  // When inside a fence, remember the exact marker so only a matching
  // closing marker exits — otherwise mismatched-length or different-char
  // fences silently desync the toggle.
  let fenceMarker: string | null = null;

  for (const line of lines) {
    if (fenceMarker === null) {
      const openMatch = line.match(FENCE_OPEN);
      if (openMatch) {
        fenceMarker = openMatch[1]!;
        if (current) current.body.push(line);
        else preambleLines.push(line);
        continue;
      }
    } else {
      const closeMatch = line.match(FENCE_OPEN);
      if (
        closeMatch &&
        closeMatch[1]!.length >= fenceMarker.length &&
        closeMatch[1]![0] === fenceMarker[0]
      ) {
        fenceMarker = null;
      }
      if (current) current.body.push(line);
      else preambleLines.push(line);
      continue;
    }

    const headingMatch = line.match(H2_PATTERN);
    if (headingMatch) {
      if (current) {
        sections.push({
          heading: current.heading,
          body: current.body.join('\n').trim(),
        });
      }
      current = { heading: headingMatch[1]!.trim(), body: [] };
      continue;
    }

    if (current) current.body.push(line);
    else preambleLines.push(line);
  }

  if (current) {
    sections.push({
      heading: current.heading,
      body: current.body.join('\n').trim(),
    });
  }

  const byHeading = new Map<string, string>();
  for (const s of sections) byHeading.set(s.heading.toLowerCase(), s.body);

  return {
    preamble: preambleLines.join('\n').trim(),
    sections,
    byHeading,
  };
}

/**
 * Required H2 sections for a family `definition.anvil` body. If any section
 * does not apply, the author must still include the heading with a brief
 * note explaining why (per ANVFMT-D008). The compiler enforces presence;
 * emptiness is surfaced as a warning downstream.
 */
export const REQUIRED_DEFINITION_SECTIONS = [
  'What It Is',
  "Why It's Harmful",
  'The Spectrum',
  'The Right Response',
  'Detection Signals',
  'Example',
] as const;

export type RequiredDefinitionSection = (typeof REQUIRED_DEFINITION_SECTIONS)[number];

export interface DefinitionSectionValidation {
  ok: boolean;
  missing: RequiredDefinitionSection[];
  empty: RequiredDefinitionSection[];
}

export function validateDefinitionSections(
  sections: MarkdownSections
): DefinitionSectionValidation {
  const missing: RequiredDefinitionSection[] = [];
  const empty: RequiredDefinitionSection[] = [];

  for (const name of REQUIRED_DEFINITION_SECTIONS) {
    const body = sections.byHeading.get(name.toLowerCase());
    if (body === undefined) missing.push(name);
    else if (body.length === 0) empty.push(name);
  }

  return {
    ok: missing.length === 0 && empty.length === 0,
    missing,
    empty,
  };
}

/**
 * Pull the narrative section used as `explanation` in compiled patterns.
 * Falls back to the empty string if the section is absent — the compiler is
 * expected to have already validated presence.
 */
export function getExplanation(sections: MarkdownSections): string {
  return sections.byHeading.get("why it's harmful") ?? '';
}

/**
 * Pull the narrative section used as `suggestion` in compiled patterns.
 */
export function getSuggestion(sections: MarkdownSections): string {
  return sections.byHeading.get('the right response') ?? '';
}
