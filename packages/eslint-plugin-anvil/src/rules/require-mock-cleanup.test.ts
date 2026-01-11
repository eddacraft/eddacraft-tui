import { RuleTester } from '@typescript-eslint/rule-tester';
import * as tsParser from '@typescript-eslint/parser';
import * as vitest from 'vitest';
import rule from './require-mock-cleanup';

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

ruleTester.run('require-mock-cleanup', rule, {
  valid: [
    // vi.mock with proper cleanup
    {
      code: `
        import { vi, afterEach } from 'vitest';

        vi.mock('./myModule');

        afterEach(() => {
          vi.restoreAllMocks();
        });

        describe('test', () => {
          it('works', () => {});
        });
      `,
    },
    // vi.spyOn with proper cleanup
    {
      code: `
        import { vi, afterEach, describe, it } from 'vitest';

        afterEach(() => {
          vi.restoreAllMocks();
        });

        describe('test', () => {
          it('works', () => {
            vi.spyOn(console, 'log');
          });
        });
      `,
    },
    // No mocks used - no cleanup needed
    {
      code: `
        import { describe, it, expect } from 'vitest';

        describe('test', () => {
          it('works', () => {
            expect(1 + 1).toBe(2);
          });
        });
      `,
    },
    // Using both vi.mock and vi.spyOn with cleanup
    {
      code: `
        import { vi, afterEach } from 'vitest';

        vi.mock('./moduleA');

        afterEach(() => {
          vi.restoreAllMocks();
        });

        describe('test', () => {
          it('works', () => {
            vi.spyOn(console, 'error');
          });
        });
      `,
    },
  ],
  invalid: [
    // vi.mock without afterEach cleanup
    {
      code: `
        import { vi, describe, it } from 'vitest';

        vi.mock('./myModule');

        describe('test', () => {
          it('works', () => {});
        });
      `,
      errors: [{ messageId: 'missingCleanup' }],
    },
    // vi.spyOn without afterEach cleanup
    {
      code: `
        import { vi, describe, it } from 'vitest';

        describe('test', () => {
          it('works', () => {
            vi.spyOn(console, 'log');
          });
        });
      `,
      errors: [{ messageId: 'missingCleanup' }],
    },
    // afterEach exists but without proper mock cleanup
    {
      code: `
        import { vi, afterEach, describe, it } from 'vitest';

        vi.mock('./myModule');

        afterEach(() => {
          // only resets state, does not restore mocks
          vi.clearAllMocks();
        });

        describe('test', () => {
          it('works', () => {});
        });
      `,
      errors: [{ messageId: 'missingCleanup' }],
    },
    // Both vi.mock and vi.spyOn without cleanup
    {
      code: `
        import { vi, describe, it } from 'vitest';

        vi.mock('./moduleA');

        describe('test', () => {
          it('works', () => {
            vi.spyOn(console, 'log');
          });
        });
      `,
      errors: [{ messageId: 'missingCleanup' }],
    },
  ],
});
