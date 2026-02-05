import { RuleTester } from '@typescript-eslint/rule-tester';
import * as tsParser from '@typescript-eslint/parser';
import * as vitest from 'vitest';
import rule from './require-cwd-restoration';

RuleTester.afterAll = vitest.afterAll;
RuleTester.it = vitest.it;
RuleTester.itOnly = vitest.it.only;
RuleTester.describe = vitest.describe;

const ruleTester = new RuleTester({
  languageOptions: {
    parser: tsParser,
    parserOptions: {
      ecmaVersion: 2022,
      sourceType: 'module',
    },
  },
});

ruleTester.run('require-cwd-restoration', rule, {
  valid: [
    // Proper cwd save and restore pattern
    {
      code: `
        const originalCwd = process.cwd();

        beforeEach(() => {
          process.chdir('/tmp/test');
        });

        afterEach(() => {
          process.chdir(originalCwd);
        });
      `,
    },
    // Using savedCwd variable name
    {
      code: `
        const savedCwd = process.cwd();
        process.chdir('/tmp');
        process.chdir(savedCwd);
      `,
    },
    // No chdir calls - no restoration needed
    {
      code: `
        const cwd = process.cwd();
        console.log(cwd);
      `,
    },
    // Using prevCwd variable name
    {
      code: `
        const prevCwd = process.cwd();
        process.chdir('/tmp');
        process.chdir(prevCwd);
      `,
    },
    // Multiple chdirs with proper restoration
    {
      code: `
        const originalCwd = process.cwd();
        process.chdir('/tmp/a');
        process.chdir('/tmp/b');
        process.chdir(originalCwd);
      `,
    },
    // Assignment expression capturing cwd
    {
      code: `
        let originalCwd: string;
        beforeEach(() => {
          originalCwd = process.cwd();
          process.chdir('/tmp/test');
        });
        afterEach(() => {
          process.chdir(originalCwd);
        });
      `,
    },
  ],
  invalid: [
    // chdir without saving original cwd
    {
      code: `
        process.chdir('/tmp/test');
      `,
      errors: [{ messageId: 'missingRestoration' }],
    },
    // chdir with cwd saved but not restored
    {
      code: `
        const cwd = process.cwd();
        process.chdir('/tmp/test');
      `,
      errors: [{ messageId: 'missingRestoration' }],
    },
    // chdir to literal string, no restoration
    {
      code: `
        beforeEach(() => {
          process.chdir('/tmp/test-dir');
        });
      `,
      errors: [{ messageId: 'missingRestoration' }],
    },
    // Multiple chdirs without restoration
    {
      code: `
        process.chdir('/tmp/a');
        process.chdir('/tmp/b');
      `,
      errors: [{ messageId: 'missingRestoration' }],
    },
    // Restoring with wrong variable (not the saved cwd)
    {
      code: `
        const originalCwd = process.cwd();
        const somethingElse = '/home';
        process.chdir('/tmp');
        process.chdir(somethingElse);
      `,
      errors: [{ messageId: 'missingRestoration' }],
    },
  ],
});
