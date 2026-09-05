import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import ts from 'typescript';
import { describe, expect, it } from 'vitest';

const src = join(dirname(fileURLToPath(import.meta.url)), '..');

function productionFiles(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    if (entry.name === '__tests__') return [];
    const path = join(dir, entry.name);
    if (entry.isDirectory()) return productionFiles(path);
    return /\.ts$/.test(path) && !/\.(test|spec)\.ts$/.test(path) ? [path] : [];
  });
}

function usersOf(identifier: string): string[] {
  return productionFiles(src)
    .filter((path) => {
      const tree = ts.createSourceFile(
        path,
        readFileSync(path, 'utf8'),
        ts.ScriptTarget.Latest,
        true
      );
      let found = false;
      function visit(node: ts.Node) {
        if (ts.isIdentifier(node) && node.text === identifier) found = true;
        ts.forEachChild(node, visit);
      }
      visit(tree);
      return found;
    })
    .map((path) => relative(src, path).replaceAll('\\', '/'))
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
