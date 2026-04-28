/**
 * Scanner-parity integration test (RSCAN-007 / ADR-026).
 *
 * Loads the shared fixtures from `tests/scanner-parity/fixtures.json`
 * (sibling to the Rust parity test at
 * `crates/anvil-checks/tests/scanner_parity.rs`) and asserts that the TS
 * scanner's output matches each fixture's declared `expected_matches`.
 * If both this suite and the Rust suite pass the same fixture data, the
 * engines are in parity on the covered rules.
 *
 * Known divergences (not yet covered by fixtures) are documented in
 * `tests/scanner-parity/README.md`.
 */
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { scanArtifact, type Artifact, type ArtifactKind } from './scanner.js';

interface ExpectedMatch {
  id: string;
  line: number;
}

interface FixtureScanOptions {
  include_opt_in?: boolean;
}

interface Fixture {
  name: string;
  artifact_kind: ArtifactKind;
  reference: string;
  content: string;
  expected_matches: ExpectedMatch[];
  /**
   * Optional scan tuning. Default mirrors the scanner's default options
   * (opt-in rules off). Fixtures targeting opt-in rules (AP-002, AP-005,
   * AP-007) must set `include_opt_in: true`.
   */
  scan_options?: FixtureScanOptions;
}

interface FixtureFile {
  fixtures: Fixture[];
}

function loadFixtures(): FixtureFile {
  // Resolve relative to this file's location: packages/anvil/core/src/antipattern/
  // Walk up four levels to the workspace root, then to tests/scanner-parity.
  const here = dirname(fileURLToPath(import.meta.url));
  const fixturePath = resolve(
    here,
    '..',
    '..',
    '..',
    '..',
    '..',
    'tests',
    'scanner-parity',
    'fixtures.json'
  );
  const raw = readFileSync(fixturePath, 'utf-8');
  return JSON.parse(raw) as FixtureFile;
}

function summarise(matches: ExpectedMatch[]): string[] {
  return matches.map((m) => `${m.id}:${m.line}`).sort();
}

describe('Scanner parity (RSCAN-007)', () => {
  const { fixtures } = loadFixtures();

  for (const fixture of fixtures) {
    it(fixture.name, () => {
      const artifact: Artifact = {
        type: fixture.artifact_kind,
        ref: fixture.reference,
        content: fixture.content,
      };
      const result = scanArtifact(
        artifact,
        fixture.scan_options?.include_opt_in ? { includeOptIn: true } : undefined
      );

      const actual: ExpectedMatch[] = result.warnings.map((w) => ({
        id: w.id,
        line: w.location.line,
      }));

      expect(summarise(actual)).toEqual(summarise(fixture.expected_matches));
    });
  }
});
