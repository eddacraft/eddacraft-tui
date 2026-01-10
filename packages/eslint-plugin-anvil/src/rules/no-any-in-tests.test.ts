import { RuleTester } from '@typescript-eslint/rule-tester';
import * as tsParser from '@typescript-eslint/parser';
import * as vitest from 'vitest';
import rule from './no-any-in-tests';

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

ruleTester.run('no-any-in-tests', rule, {
  valid: [
    // Using vi.mocked() is valid
    {
      code: `
        import { vi } from 'vitest';
        const mockedFn = vi.mocked(myFunction);
        mockedFn.mockReturnValue('test');
      `,
    },
    // Type assertions to specific types are valid
    {
      code: `
        const result = value as string;
      `,
    },
    // Type assertions to unknown are valid
    {
      code: `
        const result = value as unknown;
      `,
    },
    // Regular function calls without any are valid
    {
      code: `
        const result = myFunction();
        expect(result).toBe('test');
      `,
    },
    // Object type assertions are valid
    {
      code: `
        const config = options as MyConfig;
      `,
    },
  ],
  invalid: [
    // Basic as any assertion
    {
      code: `const result = value as any;`,
      errors: [{ messageId: 'noAnyAssertion' }],
    },
    // as any in function arguments
    {
      code: `myFunction(value as any);`,
      errors: [{ messageId: 'noAnyAssertion' }],
    },
    // as any in mock setup
    {
      code: `
        (myModule.myFunction as any).mockReturnValue('test');
      `,
      errors: [{ messageId: 'noAnyAssertion' }],
    },
    // Multiple as any assertions
    {
      code: `
        const a = x as any;
        const b = y as any;
      `,
      errors: [{ messageId: 'noAnyAssertion' }, { messageId: 'noAnyAssertion' }],
    },
    // as any in object property access
    {
      code: `(obj as any).privateProp;`,
      errors: [{ messageId: 'noAnyAssertion' }],
    },
  ],
});
