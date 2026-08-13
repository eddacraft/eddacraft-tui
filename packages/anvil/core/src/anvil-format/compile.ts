/**
 * `.anvil` → compiled pattern registry.
 *
 * Walks a `patterns/` directory, parses every `.anvil` file, groups rules by
 * family, hydrates each rule with narrative context from its family's
 * definition, and produces a single `CompiledRegistry` object that the
 * scanner can load at runtime without touching YAML or markdown.
 *
 * The compiler collects errors across files rather than failing fast, so a
 * single bad frontmatter does not hide downstream issues in other patterns.
 * Errors are returned in `AnvilCompileResult.errors`; when empty the
 * `registry` is guaranteed-complete.
 */

import { constants as fsConstants, promises as fs } from 'node:fs';
import * as path from 'node:path';

import {
  AnvilParseError,
  parseAnvilSource,
  type ParsedAnvilFile,
  type ParsedDefinitionFile,
  type ParsedRuleFile,
} from './parse.js';
import {
  extractSections,
  getExplanation,
  getSuggestion,
  validateDefinitionSections,
} from './sections.js';
import {
  CompiledPatternSchema,
  type CompiledPattern,
  type CompiledRegistry,
  type PrefixRegistry,
} from './schemas.js';

export interface AnvilCompileOptions {
  /** Absolute or cwd-relative path to the `patterns/` root. */
  patternsDir: string;
  /** Optional root the compiler resolves `definition_ref` against. Defaults to `patternsDir`'s parent. */
  referenceRoot?: string;
  /**
   * Rule prefixes that are allowed to span multiple families. Prefixes in
   * this set do not error on cross-family collision; instead they are
   * dropped from the compiled `prefixes` map (with a warning) so consumers
   * cannot resolve an ambiguous prefix to an arbitrary family. Defaults to
   * `['AP']` — the documented legacy prefix shared across families.
   * Phase 4 cleanup will allow callers to pass `[]` once the legacy AP
   * rules have been renamed onto their family prefixes.
   */
  legacyPrefixes?: readonly string[];
}

const DEFAULT_LEGACY_PREFIXES: readonly string[] = ['AP'];

function isLegacyPrefix(prefix: string, options: AnvilCompileOptions): boolean {
  return (options.legacyPrefixes ?? DEFAULT_LEGACY_PREFIXES).includes(prefix);
}

export interface AnvilCompileIssue {
  path: string;
  detail: string;
}

export interface AnvilCompileResult {
  registry: CompiledRegistry | null;
  errors: AnvilCompileIssue[];
  warnings: AnvilCompileIssue[];
}

const ANVIL_EXT = '.anvil';
/** Reject individual `.anvil` files larger than this — protects against
 *  runaway files (binaries with a `.anvil` extension, `/dev/urandom` symlinks)
 *  OOMing the compiler process. Real family definitions are < 20 KB. */
export const MAX_ANVIL_FILE_BYTES = 1_048_576; // 1 MiB

export interface DiscoverIssue {
  path: string;
  detail: string;
}

export interface DiscoverResult {
  files: string[];
  errors: DiscoverIssue[];
}

/**
 * Recursively list every `.anvil` regular file under `root`. Symlinks are
 * skipped entirely (to both files and directories) so a symlink inside
 * `patterns/` cannot cause the walker to read files outside the root.
 * Returns paths sorted by their `root`-relative form so discovery order is
 * portable across machines (absolute paths vary by checkout location).
 * Walker failures (permission denied, missing dir) are collected into
 * `errors` rather than propagating as unhandled rejections.
 */
export async function discoverAnvilFiles(root: string): Promise<DiscoverResult> {
  const files: string[] = [];
  const errors: DiscoverIssue[] = [];
  const resolvedRoot = path.resolve(root);

  async function walk(dir: string): Promise<void> {
    let entries;
    try {
      entries = await fs.readdir(dir, { withFileTypes: true });
    } catch (err) {
      errors.push({
        path: dir,
        detail: `failed to read directory: ${err instanceof Error ? err.message : String(err)}`,
      });
      return;
    }
    for (const entry of entries) {
      const abs = path.join(dir, entry.name);
      if (entry.isSymbolicLink()) {
        // Never follow symlinks — they could escape the patterns root.
        continue;
      }
      if (entry.isDirectory()) {
        await walk(abs);
      } else if (entry.isFile() && entry.name.endsWith(ANVIL_EXT)) {
        files.push(abs);
      }
    }
  }

  await walk(resolvedRoot);
  files.sort((a, b) => {
    const ra = path.relative(resolvedRoot, a);
    const rb = path.relative(resolvedRoot, b);
    // Byte-level compare (not localeCompare) so the sort is deterministic
    // across locales — otherwise prefix-owner resolution is LC_ALL-sensitive.
    return ra < rb ? -1 : ra > rb ? 1 : 0;
  });
  return { files, errors };
}

interface ParseBatch {
  definitions: Map<string, ParsedDefinitionFile>;
  rulesByFamily: Map<string, ParsedRuleFile[]>;
  errors: AnvilCompileIssue[];
}

async function parseBatch(files: string[]): Promise<ParseBatch> {
  const definitions = new Map<string, ParsedDefinitionFile>();
  const rulesByFamily = new Map<string, ParsedRuleFile[]>();
  const errors: AnvilCompileIssue[] = [];

  for (const file of files) {
    let parsed: ParsedAnvilFile;
    try {
      // Open once with O_NOFOLLOW, stat + read against the same file
      // descriptor. This closes two holes together: (1) the TOCTOU race
      // between size check and read, flagged by CodeQL js/file-system-race,
      // and (2) the symlink-swap race where a regular file discovered during
      // walk is replaced with a symlink pointing outside `patternsDir`
      // before this open. On Linux, O_NOFOLLOW causes open() to fail with
      // ELOOP when the final path component is a symlink.
      const fh = await fs.open(file, fsConstants.O_RDONLY | fsConstants.O_NOFOLLOW);
      let raw: string;
      try {
        const stat = await fh.stat();
        if (!stat.isFile()) {
          errors.push({ path: file, detail: 'not a regular file' });
          continue;
        }
        if (stat.size > MAX_ANVIL_FILE_BYTES) {
          errors.push({
            path: file,
            detail: `file is ${stat.size} bytes, exceeds ${MAX_ANVIL_FILE_BYTES}-byte limit`,
          });
          continue;
        }
        raw = await fh.readFile('utf8');
      } finally {
        await fh.close();
      }
      parsed = parseAnvilSource(file, raw);
    } catch (err) {
      if (err instanceof AnvilParseError) {
        errors.push({ path: err.path, detail: err.detail });
      } else {
        errors.push({
          path: file,
          detail: err instanceof Error ? err.message : String(err),
        });
      }
      continue;
    }

    if (parsed.kind === 'definition') {
      const existing = definitions.get(parsed.frontmatter.id);
      if (existing) {
        errors.push({
          path: parsed.path,
          detail: `duplicate family definition "${parsed.frontmatter.id}" (also declared at ${existing.path})`,
        });
        continue;
      }
      definitions.set(parsed.frontmatter.id, parsed);
    } else {
      const family = parsed.frontmatter.family;
      const list = rulesByFamily.get(family) ?? [];
      list.push(parsed);
      rulesByFamily.set(family, list);
    }
  }

  return { definitions, rulesByFamily, errors };
}

function rulePrefix(ruleId: string): string {
  const dashIndex = ruleId.indexOf('-');
  return dashIndex === -1 ? ruleId : ruleId.slice(0, dashIndex);
}

function relativeRef(fromRoot: string, target: string): string {
  const rel = path.relative(fromRoot, target);
  // Normalise to forward slashes so registry output is platform-independent.
  return rel.split(path.sep).join('/');
}

/**
 * Compile a `patterns/` directory into a `CompiledRegistry`.
 *
 * Non-fatal issues (e.g. ambiguous prefixes that span multiple families)
 * surface in `warnings`. Fatal issues (missing or empty required sections,
 * missing family, duplicate rule id, schema violations) populate `errors`
 * and cause `registry` to be null.
 */
export async function compilePatterns(options: AnvilCompileOptions): Promise<AnvilCompileResult> {
  const patternsDir = path.resolve(options.patternsDir);
  const referenceRoot = path.resolve(options.referenceRoot ?? path.dirname(patternsDir));

  const errors: AnvilCompileIssue[] = [];
  const warnings: AnvilCompileIssue[] = [];

  const { files, errors: discoverErrors } = await discoverAnvilFiles(patternsDir);
  errors.push(...discoverErrors);
  const { definitions, rulesByFamily, errors: parseErrors } = await parseBatch(files);
  errors.push(...parseErrors);

  const seenRuleIds = new Map<string, string>();
  const prefixes: PrefixRegistry = {};
  // Prefixes that turned out to span multiple families (legacy `AP`): we
  // drop them from `prefixes` rather than bind them to an arbitrary family
  // so consumers cannot silently resolve to a wrong family.
  const ambiguousPrefixes = new Set<string>();
  const compiledPatterns: CompiledPattern[] = [];
  const familyRecords: CompiledRegistry['families'] = [];

  const sortedDefinitions = [...definitions].sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0));
  for (const [familyId, definition] of sortedDefinitions) {
    const sections = extractSections(definition.body);
    const sectionCheck = validateDefinitionSections(sections);

    if (sectionCheck.missing.length > 0) {
      errors.push({
        path: definition.path,
        detail: `definition missing required sections: ${sectionCheck.missing.join(', ')}`,
      });
      // Don't compile rules for a definition with missing required sections
      // — the assembled CompiledPattern would have empty explanation/suggestion
      // and fail CompiledPatternSchema validation downstream.
      continue;
    }
    if (sectionCheck.empty.length > 0) {
      errors.push({
        path: definition.path,
        detail: `definition has empty required sections: ${sectionCheck.empty.join(', ')}`,
      });
      // The assembled CompiledPattern would have empty explanation or
      // suggestion strings and fail CompiledPatternSchema's `min(1)` check
      // further down with a less-obvious message, so bail out here.
      continue;
    }

    const explanation = getExplanation(sections);
    const suggestion = getSuggestion(sections);

    const siblings = rulesByFamily.get(familyId) ?? [];
    const declaredRuleIds = definition.frontmatter.rules;
    const siblingIds = siblings.map((r) => r.frontmatter.id).sort();

    if (declaredRuleIds) {
      const declaredSet = new Set(declaredRuleIds);
      const siblingSet = new Set(siblingIds);
      for (const id of declaredRuleIds) {
        if (!siblingSet.has(id)) {
          warnings.push({
            path: definition.path,
            detail: `family declares rule "${id}" but no matching rule file found`,
          });
        }
      }
      for (const id of siblingIds) {
        if (!declaredSet.has(id)) {
          warnings.push({
            path: definition.path,
            detail: `rule "${id}" exists in directory but is not listed in family rules`,
          });
        }
      }
    }

    familyRecords.push({
      id: familyId,
      name: definition.frontmatter.name,
      category: definition.frontmatter.category,
      definition_ref: relativeRef(referenceRoot, definition.path),
      rules: siblingIds,
      related: [...definition.frontmatter.related],
      tensions: [...definition.frontmatter.tensions],
    });

    for (const rule of siblings) {
      const id = rule.frontmatter.id;
      const prior = seenRuleIds.get(id);
      if (prior) {
        errors.push({
          path: rule.path,
          detail: `duplicate rule id "${id}" (also defined at ${prior})`,
        });
        continue;
      }
      seenRuleIds.set(id, rule.path);

      const prefix = rulePrefix(id);
      const existingPrefixFamily = prefixes[prefix];
      if (existingPrefixFamily && existingPrefixFamily !== familyId) {
        if (isLegacyPrefix(prefix, options)) {
          // Legacy mixed-family prefix (e.g. `AP-`): record ambiguity and
          // drop the prefix from the registry so consumers cannot resolve
          // it to a wrong family. Emit a warning so it's visible in CI.
          ambiguousPrefixes.add(prefix);
          warnings.push({
            path: rule.path,
            detail: `prefix "${prefix}" spans multiple families (${existingPrefixFamily}, ${familyId}) — omitted from prefix map`,
          });
        } else {
          errors.push({
            path: rule.path,
            detail: `prefix "${prefix}" is already bound to family "${existingPrefixFamily}" — cannot also bind to "${familyId}"`,
          });
          continue;
        }
      } else if (!existingPrefixFamily) {
        prefixes[prefix] = familyId;
      }

      // A rule may override the family "Why It's Harmful" section so mixed
      // families (e.g. python-reliability) can ship a rule-specific
      // explanation. The body preamble stays the nudge.
      const ruleSections = extractSections(rule.body);
      const ruleExplanation = getExplanation(ruleSections);

      const compiled: CompiledPattern = {
        id,
        family: familyId,
        title: rule.frontmatter.title,
        version: rule.frontmatter.version,
        severity: rule.frontmatter.severity,
        confidence: rule.frontmatter.confidence,
        spectrum_position: rule.frontmatter.spectrum_position,
        targets: rule.frontmatter.targets,
        detection: rule.frontmatter.detection,
        allowlist: rule.frontmatter.allowlist,
        nudge: ruleSections.preamble.length > 0 ? ruleSections.preamble : rule.body,
        related: rule.frontmatter.related,
        enabled: rule.frontmatter.enabled,
        opt_in: rule.frontmatter.opt_in,

        family_name: definition.frontmatter.name,
        category: definition.frontmatter.category,
        explanation: ruleExplanation || explanation,
        suggestion,
        definition_ref: relativeRef(referenceRoot, definition.path),
        tensions: [...definition.frontmatter.tensions],
        related_families: [...definition.frontmatter.related],
      };
      if (rule.frontmatter.file_extensions) {
        compiled.file_extensions = rule.frontmatter.file_extensions;
      }

      // Final schema validation — catches assembled patterns that slipped
      // past earlier checks (e.g. whitespace-only section bodies leaving
      // `explanation` empty, which `z.string().min(1)` will reject).
      const validated = CompiledPatternSchema.safeParse(compiled);
      if (!validated.success) {
        errors.push({
          path: rule.path,
          detail: `compiled pattern failed schema validation: ${validated.error.issues
            .map((i) => `${i.path.join('.') || '<root>'}: ${i.message}`)
            .join('; ')}`,
        });
        continue;
      }
      compiledPatterns.push(validated.data);
    }
  }

  // Orphan check: rules whose family has no definition file.
  for (const [familyId, rules] of rulesByFamily) {
    if (definitions.has(familyId)) continue;
    for (const rule of rules) {
      errors.push({
        path: rule.path,
        detail: `rule declares family "${familyId}" but no definition.anvil was found for that family`,
      });
    }
  }

  for (const prefix of ambiguousPrefixes) {
    delete prefixes[prefix];
  }

  const byteCompare = (a: string, b: string) => (a < b ? -1 : a > b ? 1 : 0);
  compiledPatterns.sort((a, b) => byteCompare(a.id, b.id));
  familyRecords.sort((a, b) => byteCompare(a.id, b.id));

  if (errors.length > 0) {
    return { registry: null, errors, warnings };
  }

  const registry: CompiledRegistry = {
    schema_version: 1,
    compiled_at: new Date().toISOString(),
    source_root: relativeRef(referenceRoot, patternsDir),
    patterns: compiledPatterns,
    prefixes,
    families: familyRecords,
  };

  return { registry, errors, warnings };
}

/**
 * Convenience: compile then write the registry JSON to `outputPath`.
 * Parent directories are created on demand. Returns the full compile result
 * so callers can still surface warnings after writing.
 */
export async function compileAndWrite(
  options: AnvilCompileOptions & { outputPath: string }
): Promise<AnvilCompileResult> {
  const result = await compilePatterns(options);
  if (result.registry) {
    const outputPath = path.resolve(options.outputPath);
    await fs.mkdir(path.dirname(outputPath), { recursive: true });
    const serialised = JSON.stringify(result.registry, null, 2) + '\n';
    await fs.writeFile(outputPath, serialised, 'utf8');
  }
  return result;
}
