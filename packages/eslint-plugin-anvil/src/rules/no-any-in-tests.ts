import type { Rule } from 'eslint';

/**
 * Rule: no-any-in-tests
 *
 * Disallows `as any` type assertions in test files.
 * Use vi.mocked() for typed mock access instead.
 */
const rule: Rule.RuleModule = {
  meta: {
    type: 'suggestion',
    docs: {
      description: 'Disallow `as any` type assertions in test files',
      recommended: true,
    },
    messages: {
      noAnyAssertion: 'Avoid using `as any` in tests. Use vi.mocked() for typed mock access.',
    },
    schema: [],
  },

  create(context) {
    return {
      // Use a selector string for TSAsExpression with TSAnyKeyword
      // This works with @typescript-eslint/parser
      'TSAsExpression > TSAnyKeyword'(node: Rule.Node) {
        context.report({
          node: node.parent as Rule.Node,
          messageId: 'noAnyAssertion',
        });
      },
    };
  },
};

export default rule;
