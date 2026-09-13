import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const src = join(dirname(fileURLToPath(import.meta.url)), '..');

function productionFiles(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    if (entry.name === '__tests__') return [];
    const path = join(dir, entry.name);
    if (entry.isDirectory()) return productionFiles(path);
    return path.endsWith('.ts') && !/\.(test|spec)\.ts$/.test(path) ? [path] : [];
  });
}

// One cheap pass over production sources, then identifier-token matches.
// Catches imports, re-exports, and calls without a TypeScript AST walk.
// Parsing every file per identifier timed out the 5s Vitest default on
// Windows Nightly (same class as #4660).
const productionSources = productionFiles(src).map((path) => ({
  rel: relative(src, path).replaceAll('\\', '/'),
  source: readFileSync(path, 'utf8'),
}));

function identifierToken(identifier: string): RegExp {
  return new RegExp(`(?<![A-Za-z0-9_$])${identifier}(?![A-Za-z0-9_$])`);
}

function usersOf(identifier: string): string[] {
  const token = identifierToken(identifier);
  return productionSources
    .filter(({ source }) => token.test(source))
    .map(({ rel }) => rel)
    .sort();
}

describe('SEC-013 API licence route inventory', () => {
  it('confines raw licence verification to the shared authentication boundary', () => {
    // Includes aliased imports/re-exports, not just literal call expressions.
    expect(usersOf('verifyLicence')).toEqual(['lib/licence.ts', 'middleware/licence-auth.ts']);
    expect(usersOf('jwtVerify')).toEqual(['lib/licence.ts']);
  });

  it('requires an inventory review when the shared boundary gains a consumer', () => {
    // Owning inventory: docs/architecture/auth-as-built.md, SEC-013 section.
    expect(usersOf('authenticateLicence')).toEqual([
      'middleware/licence-auth.ts',
      'routes/account-activity.ts',
      'routes/auth.ts',
    ]);
  });
});
