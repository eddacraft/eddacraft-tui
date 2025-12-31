import { describe, it, expect } from 'vitest';
import {
  findMatchingRule,
  analyseCommand,
  calculateSpecificity,
  RuleMatcher,
} from './rule-matcher.js';
import type { CommandRule, ParsedCommand } from './types.js';

function makeParsedCommand(overrides: Partial<ParsedCommand> = {}): ParsedCommand {
  return {
    raw: 'test',
    command: 'git',
    subcommand: undefined,
    flags: [],
    args: [],
    unwrapped: 'test',
    wrapperChain: [],
    ...overrides,
  };
}

const gitResetHardRule: CommandRule = {
  id: 'git-reset-hard',
  category: 'git',
  command: 'git',
  subcommand: 'reset',
  flags: { dangerous: ['--hard'] },
  action: 'block',
  severity: 'error',
  reason: 'git reset --hard destroys uncommitted changes',
  suggestion: 'Use git stash first',
};

const gitPushForceRule: CommandRule = {
  id: 'git-push-force',
  category: 'git',
  command: 'git',
  subcommand: 'push',
  flags: { dangerous: ['--force', '-f'], forbidden: ['--force-with-lease'] },
  action: 'block',
  severity: 'error',
  reason: 'git push --force rewrites history',
  suggestion: 'Use --force-with-lease instead',
};

const gitPushForceWithLeaseRule: CommandRule = {
  id: 'git-push-force-with-lease',
  category: 'git',
  command: 'git',
  subcommand: 'push',
  flags: { required: ['--force-with-lease'] },
  action: 'allow',
  severity: 'info',
  reason: 'Force with lease is safer',
};

const gitCheckoutBranchRule: CommandRule = {
  id: 'git-checkout-branch',
  category: 'git',
  command: 'git',
  subcommand: 'checkout',
  flags: { required: ['-b'] },
  action: 'allow',
  severity: 'info',
  reason: 'Branch creation is safe',
};

const gitCheckoutDiscardRule: CommandRule = {
  id: 'git-checkout-discard',
  category: 'git',
  command: 'git',
  subcommand: 'checkout',
  flags: { dangerous: ['--'] },
  action: 'block',
  severity: 'error',
  reason: 'git checkout -- discards changes',
};

const rmRfRootRule: CommandRule = {
  id: 'rm-rf-root',
  category: 'filesystem',
  command: 'rm',
  flags: { dangerous: ['-r', '-f'] },
  args: { pattern: /^(\/|~|~\/|\$HOME)/ },
  action: 'block',
  severity: 'error',
  reason: 'rm -rf on root/home is dangerous',
};

const rmRfNodeModulesRule: CommandRule = {
  id: 'rm-rf-node-modules',
  category: 'filesystem',
  command: 'rm',
  flags: { dangerous: ['-r', '-f'] },
  args: { pattern: /^(\.\/)?node_modules$/ },
  action: 'allow',
  severity: 'info',
  reason: 'Deleting node_modules is safe',
};

const allRules: CommandRule[] = [
  gitResetHardRule,
  gitPushForceRule,
  gitPushForceWithLeaseRule,
  gitCheckoutBranchRule,
  gitCheckoutDiscardRule,
  rmRfRootRule,
  rmRfNodeModulesRule,
];

describe('calculateSpecificity', () => {
  it('gives base score for command-only rule', () => {
    const rule: CommandRule = {
      id: 'test',
      category: 'git',
      command: 'git',
      action: 'block',
      severity: 'error',
      reason: 'test',
    };
    expect(calculateSpecificity(rule)).toBe(1);
  });

  it('adds points for subcommand', () => {
    const rule: CommandRule = {
      id: 'test',
      category: 'git',
      command: 'git',
      subcommand: 'reset',
      action: 'block',
      severity: 'error',
      reason: 'test',
    };
    expect(calculateSpecificity(rule)).toBe(3);
  });

  it('adds points for flags', () => {
    const rule: CommandRule = {
      id: 'test',
      category: 'git',
      command: 'git',
      flags: { dangerous: ['--hard'] },
      action: 'block',
      severity: 'error',
      reason: 'test',
    };
    expect(calculateSpecificity(rule)).toBe(5);
  });

  it('adds points for args pattern', () => {
    const rule: CommandRule = {
      id: 'test',
      category: 'git',
      command: 'git',
      args: { pattern: /test/ },
      action: 'block',
      severity: 'error',
      reason: 'test',
    };
    expect(calculateSpecificity(rule)).toBe(9);
  });

  it('combines all specificity points', () => {
    const rule: CommandRule = {
      id: 'test',
      category: 'git',
      command: 'git',
      subcommand: 'reset',
      flags: { dangerous: ['--hard'] },
      args: { pattern: /HEAD/ },
      action: 'block',
      severity: 'error',
      reason: 'test',
    };
    expect(calculateSpecificity(rule)).toBe(15);
  });
});

describe('findMatchingRule', () => {
  it('returns undefined for no matching rules', () => {
    const parsed = makeParsedCommand({ command: 'ls' });
    const result = findMatchingRule(parsed, allRules);
    expect(result).toBeUndefined();
  });

  it('matches command and subcommand', () => {
    const parsed = makeParsedCommand({
      command: 'git',
      subcommand: 'reset',
      flags: ['--hard'],
    });
    const result = findMatchingRule(parsed, allRules);
    expect(result?.id).toBe('git-reset-hard');
  });

  it('matches dangerous flags', () => {
    const parsed = makeParsedCommand({
      command: 'git',
      subcommand: 'push',
      flags: ['--force'],
    });
    const result = findMatchingRule(parsed, allRules);
    expect(result?.id).toBe('git-push-force');
  });

  it('matches short form of dangerous flags', () => {
    const parsed = makeParsedCommand({
      command: 'git',
      subcommand: 'push',
      flags: ['-f'],
    });
    const result = findMatchingRule(parsed, allRules);
    expect(result?.id).toBe('git-push-force');
  });

  it('respects forbidden flags', () => {
    const parsed = makeParsedCommand({
      command: 'git',
      subcommand: 'push',
      flags: ['--force-with-lease'],
    });
    const result = findMatchingRule(parsed, allRules);
    expect(result?.id).toBe('git-push-force-with-lease');
  });

  it('matches required flags', () => {
    const parsed = makeParsedCommand({
      command: 'git',
      subcommand: 'checkout',
      flags: ['-b'],
      args: ['new-branch'],
    });
    const result = findMatchingRule(parsed, allRules);
    expect(result?.id).toBe('git-checkout-branch');
  });

  it('selects more specific rule', () => {
    const generalRule: CommandRule = {
      id: 'git-general',
      category: 'git',
      command: 'git',
      action: 'warn',
      severity: 'warning',
      reason: 'General git warning',
    };

    const parsed = makeParsedCommand({
      command: 'git',
      subcommand: 'reset',
      flags: ['--hard'],
    });

    const result = findMatchingRule(parsed, [...allRules, generalRule]);
    expect(result?.id).toBe('git-reset-hard');
  });

  it('matches args pattern', () => {
    const parsed = makeParsedCommand({
      command: 'rm',
      flags: ['-r', '-f'],
      args: ['/home/user'],
    });
    const result = findMatchingRule(parsed, allRules);
    expect(result?.id).toBe('rm-rf-root');
  });

  it('matches args pattern with tilde', () => {
    const parsed = makeParsedCommand({
      command: 'rm',
      flags: ['-r', '-f'],
      args: ['~/projects'],
    });
    const result = findMatchingRule(parsed, allRules);
    expect(result?.id).toBe('rm-rf-root');
  });

  it('matches more specific args pattern', () => {
    const parsed = makeParsedCommand({
      command: 'rm',
      flags: ['-r', '-f'],
      args: ['node_modules'],
    });
    const result = findMatchingRule(parsed, allRules);
    expect(result?.id).toBe('rm-rf-node-modules');
  });
});

describe('analyseCommand', () => {
  it('returns allow for unmatched commands', () => {
    const parsed = makeParsedCommand({ command: 'ls' });
    const result = analyseCommand('ls', parsed, allRules);
    expect(result.action).toBe('allow');
    expect(result.severity).toBe('info');
    expect(result.matchedRule).toBeUndefined();
  });

  it('returns block with reason for matched dangerous command', () => {
    const parsed = makeParsedCommand({
      command: 'git',
      subcommand: 'reset',
      flags: ['--hard'],
    });
    const result = analyseCommand('git reset --hard', parsed, allRules);
    expect(result.action).toBe('block');
    expect(result.severity).toBe('error');
    expect(result.reason).toBe('git reset --hard destroys uncommitted changes');
    expect(result.suggestion).toBe('Use git stash first');
  });

  it('preserves parsed command in result', () => {
    const parsed = makeParsedCommand({
      raw: 'sudo git reset --hard',
      command: 'git',
      subcommand: 'reset',
      flags: ['--hard'],
      wrapperChain: ['sudo'],
    });
    const result = analyseCommand('sudo git reset --hard', parsed, allRules);
    expect(result.parsedCommand).toBe(parsed);
    expect(result.command).toBe('sudo git reset --hard');
  });
});

describe('findMatchingRule with conditions', () => {
  const strictOnlyRule: CommandRule = {
    id: 'strict-only-rule',
    category: 'git',
    command: 'git',
    subcommand: 'reset',
    action: 'block',
    severity: 'error',
    reason: 'Only in strict mode',
    conditions: { strictModeOnly: true },
  };

  it('skips strictModeOnly rule when strict is false', () => {
    const parsed = makeParsedCommand({
      command: 'git',
      subcommand: 'reset',
      flags: [],
    });
    const result = findMatchingRule(parsed, [strictOnlyRule], { strict: false });
    expect(result).toBeUndefined();
  });

  it('matches strictModeOnly rule when strict is true', () => {
    const parsed = makeParsedCommand({
      command: 'git',
      subcommand: 'reset',
      flags: [],
    });
    const result = findMatchingRule(parsed, [strictOnlyRule], { strict: true });
    expect(result?.id).toBe('strict-only-rule');
  });

  it('skips strictModeOnly rule when no context provided', () => {
    const parsed = makeParsedCommand({
      command: 'git',
      subcommand: 'reset',
      flags: [],
    });
    const result = findMatchingRule(parsed, [strictOnlyRule]);
    expect(result).toBeUndefined();
  });
});

describe('findMatchingRule with requiredAll', () => {
  const requireAllFlagsRule: CommandRule = {
    id: 'require-all-flags',
    category: 'custom',
    command: 'dangerous',
    flags: { requiredAll: ['-a', '-b', '-c'] },
    action: 'block',
    severity: 'error',
    reason: 'All flags required',
  };

  it('matches when all requiredAll flags present', () => {
    const parsed = makeParsedCommand({
      command: 'dangerous',
      flags: ['-a', '-b', '-c'],
    });
    const result = findMatchingRule(parsed, [requireAllFlagsRule]);
    expect(result?.id).toBe('require-all-flags');
  });

  it('does not match when only some requiredAll flags present', () => {
    const parsed = makeParsedCommand({
      command: 'dangerous',
      flags: ['-a', '-b'],
    });
    const result = findMatchingRule(parsed, [requireAllFlagsRule]);
    expect(result).toBeUndefined();
  });

  it('does not match when one requiredAll flag missing', () => {
    const parsed = makeParsedCommand({
      command: 'dangerous',
      flags: ['-a', '-c'],
    });
    const result = findMatchingRule(parsed, [requireAllFlagsRule]);
    expect(result).toBeUndefined();
  });

  it('matches with extra flags beyond requiredAll', () => {
    const parsed = makeParsedCommand({
      command: 'dangerous',
      flags: ['-a', '-b', '-c', '-d', '-e'],
    });
    const result = findMatchingRule(parsed, [requireAllFlagsRule]);
    expect(result?.id).toBe('require-all-flags');
  });
});

describe('RuleMatcher class', () => {
  it('creates with empty rules', () => {
    const matcher = new RuleMatcher();
    expect(matcher.getRules()).toEqual([]);
  });

  it('creates with initial rules', () => {
    const matcher = new RuleMatcher(allRules);
    expect(matcher.getRules()).toHaveLength(allRules.length);
  });

  it('adds single rule', () => {
    const matcher = new RuleMatcher();
    matcher.addRule(gitResetHardRule);
    expect(matcher.getRules()).toHaveLength(1);
  });

  it('adds multiple rules', () => {
    const matcher = new RuleMatcher();
    matcher.addRules([gitResetHardRule, gitPushForceRule]);
    expect(matcher.getRules()).toHaveLength(2);
  });

  it('sets rules replacing existing', () => {
    const matcher = new RuleMatcher(allRules);
    matcher.setRules([gitResetHardRule]);
    expect(matcher.getRules()).toHaveLength(1);
  });

  it('finds matching rule', () => {
    const matcher = new RuleMatcher(allRules);
    const parsed = makeParsedCommand({
      command: 'git',
      subcommand: 'reset',
      flags: ['--hard'],
    });
    const result = matcher.findMatchingRule(parsed);
    expect(result?.id).toBe('git-reset-hard');
  });

  it('analyses single command', () => {
    const matcher = new RuleMatcher(allRules);
    const parsed = makeParsedCommand({
      command: 'git',
      subcommand: 'push',
      flags: ['--force'],
    });
    const result = matcher.analyse('git push --force', parsed);
    expect(result.action).toBe('block');
  });

  it('analyses multiple commands', () => {
    const matcher = new RuleMatcher(allRules);
    const commands = [
      {
        command: 'git reset --hard',
        parsed: makeParsedCommand({
          command: 'git',
          subcommand: 'reset',
          flags: ['--hard'],
        }),
      },
      {
        command: 'ls -la',
        parsed: makeParsedCommand({ command: 'ls', flags: ['-l', '-a'] }),
      },
    ];
    const results = matcher.analyseMultiple(commands);
    expect(results).toHaveLength(2);
    expect(results[0].action).toBe('block');
    expect(results[1].action).toBe('allow');
  });
});
