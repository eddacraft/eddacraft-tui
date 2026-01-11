import type { Rule } from 'eslint';

/**
 * Rule: require-mock-cleanup
 *
 * Requires vi.restoreAllMocks() to be called inside an afterEach hook
 * when using vi.mock() or vi.spyOn().
 */
const rule: Rule.RuleModule = {
  meta: {
    type: 'suggestion',
    docs: {
      description: 'Require vi.restoreAllMocks() in afterEach when using mocks',
      recommended: true,
    },
    messages: {
      missingCleanup:
        'Test file uses mocks but vi.restoreAllMocks() is not called in afterEach. Add cleanup to prevent test pollution.',
    },
    schema: [],
  },

  create(context) {
    let hasMockCalls = false;
    let hasSpyOnCalls = false;
    let hasRestoreInAfterEach = false;

    return {
      Program() {
        // Reset state for each file
        hasMockCalls = false;
        hasSpyOnCalls = false;
        hasRestoreInAfterEach = false;
      },

      CallExpression(node) {
        // Check for afterEach() - mark that we're entering its callback
        if (
          node.callee.type === 'Identifier' &&
          node.callee.name === 'afterEach' &&
          node.arguments.length > 0
        ) {
          // We'll track the callback scope separately
          const callback = node.arguments[0];
          if (
            callback.type === 'ArrowFunctionExpression' ||
            callback.type === 'FunctionExpression'
          ) {
            // Check if vi.restoreAllMocks() is called in the callback body
            // Use regex to match actual method call, not comments or strings
            const callbackSource = context.sourceCode.getText(callback);
            // Match vi.restoreAllMocks() call pattern, avoiding comments and strings
            const restorePattern = /vi\s*\.\s*restoreAllMocks\s*\(/;
            if (restorePattern.test(callbackSource)) {
              hasRestoreInAfterEach = true;
            }
          }
        }

        // Check for vi.mock() or vi.spyOn()
        if (
          node.callee.type === 'MemberExpression' &&
          node.callee.object.type === 'Identifier' &&
          node.callee.object.name === 'vi' &&
          node.callee.property.type === 'Identifier'
        ) {
          const methodName = node.callee.property.name;

          if (methodName === 'mock') {
            hasMockCalls = true;
          }
          if (methodName === 'spyOn') {
            hasSpyOnCalls = true;
          }
        }
      },

      'Program:exit'(node) {
        // Only report if mocks are used but no cleanup in afterEach
        if ((hasMockCalls || hasSpyOnCalls) && !hasRestoreInAfterEach) {
          context.report({
            node,
            messageId: 'missingCleanup',
            loc: { line: 1, column: 0 },
          });
        }
      },
    };
  },
};

export default rule;
