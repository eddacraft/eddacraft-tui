import type { Rule } from 'eslint';

/**
 * Rule: require-cwd-restoration
 *
 * Requires restoring process.cwd() after calling process.chdir().
 * Detects patterns where:
 * 1. A variable stores process.cwd()
 * 2. That variable is later passed to process.chdir()
 */
const rule: Rule.RuleModule = {
  meta: {
    type: 'problem',
    docs: {
      description: 'Require restoring process.cwd() after process.chdir()',
      recommended: true,
    },
    messages: {
      missingRestoration:
        'Test calls process.chdir() but does not appear to restore the original directory. Save process.cwd() before changing and restore in afterEach.',
    },
    schema: [],
  },

  create(context) {
    let hasChdirCall = false;
    // Track variables that store process.cwd()
    const cwdVariables = new Set<string>();
    // Track if any cwd variable is used in chdir
    let hasCwdRestoration = false;

    const isProcessCwdCall = (node: unknown): boolean => {
      if (typeof node !== 'object' || node === null) {
        return false;
      }
      if (!('type' in node) || node.type !== 'CallExpression') {
        return false;
      }
      if (!('callee' in node) || typeof node.callee !== 'object' || node.callee === null) {
        return false;
      }
      const callee = node.callee;
      if (!('type' in callee) || callee.type !== 'MemberExpression') {
        return false;
      }
      if (!('object' in callee) || typeof callee.object !== 'object' || callee.object === null) {
        return false;
      }
      if (
        !('property' in callee) ||
        typeof callee.property !== 'object' ||
        callee.property === null
      ) {
        return false;
      }
      const object = callee.object;
      const property = callee.property;
      return (
        'type' in object &&
        object.type === 'Identifier' &&
        'name' in object &&
        object.name === 'process' &&
        'type' in property &&
        property.type === 'Identifier' &&
        'name' in property &&
        property.name === 'cwd'
      );
    };

    return {
      Program() {
        // Reset state for each file
        hasChdirCall = false;
        cwdVariables.clear();
        hasCwdRestoration = false;
      },

      // Track: const originalCwd = process.cwd()
      VariableDeclarator(node) {
        if (node.id.type === 'Identifier' && node.init && isProcessCwdCall(node.init)) {
          cwdVariables.add(node.id.name);
        }
      },

      // Track: originalCwd = process.cwd()
      AssignmentExpression(node) {
        if (node.left.type === 'Identifier' && isProcessCwdCall(node.right)) {
          cwdVariables.add(node.left.name);
        }
      },

      CallExpression(node) {
        // Check for process.chdir()
        if (
          node.callee.type === 'MemberExpression' &&
          node.callee.object.type === 'Identifier' &&
          node.callee.object.name === 'process' &&
          node.callee.property.type === 'Identifier' &&
          node.callee.property.name === 'chdir'
        ) {
          hasChdirCall = true;

          // Check if the argument is a saved cwd variable
          if (node.arguments.length > 0) {
            const arg = node.arguments[0];
            if (arg.type === 'Identifier' && cwdVariables.has(arg.name)) {
              hasCwdRestoration = true;
            }
          }
        }
      },

      'Program:exit'(node) {
        // Only report if chdir is called but no restoration detected
        if (hasChdirCall && !hasCwdRestoration) {
          context.report({
            node,
            messageId: 'missingRestoration',
            loc: { line: 1, column: 0 },
          });
        }
      },
    };
  },
};

export default rule;
