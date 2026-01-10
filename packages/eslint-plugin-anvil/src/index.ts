import noAnyInTests from './rules/no-any-in-tests';
import requireMockCleanup from './rules/require-mock-cleanup';
import requireCwdRestoration from './rules/require-cwd-restoration';

const plugin = {
  meta: {
    name: 'eslint-plugin-anvil',
    version: '0.1.0',
  },
  rules: {
    'no-any-in-tests': noAnyInTests,
    'require-mock-cleanup': requireMockCleanup,
    'require-cwd-restoration': requireCwdRestoration,
  },
  configs: {
    recommended: {
      plugins: ['anvil'],
      rules: {
        'anvil/no-any-in-tests': 'warn',
        'anvil/require-mock-cleanup': 'warn',
        'anvil/require-cwd-restoration': 'warn',
      },
    },
  },
};

export = plugin;
