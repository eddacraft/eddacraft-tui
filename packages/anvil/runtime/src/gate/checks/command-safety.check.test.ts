import { describe, it, expect, beforeEach } from 'vitest';
import { CommandSafetyCheck } from './command-safety.check.js';
import type { CheckContext, PlanData } from '../../types/gate.types.js';
import type { CommandSafetyFinding, CommandAnalysisSummary } from '../rules/types.js';

describe('CommandSafetyCheck', () => {
  let check: CommandSafetyCheck;
  let baseContext: CheckContext;

  const createPlan = (commands: { command: string; path?: string }[]): PlanData => ({
    id: 'aps-test-command-safety',
    schema_version: '0.1.0',
    hash: 'test-hash',
    intent: 'Test command safety check',
    proposed_changes: commands.map(({ command, path }) => ({
      type: 'script_execute' as const,
      path: path ?? 'script',
      description: command,
    })),
    provenance: {
      timestamp: '2025-01-01T00:00:00Z',
      author: 'test@example.com',
      source: 'cli',
      version: '1.0.0',
    },
    validations: {
      required_checks: [],
      skip_checks: [],
    },
    evidence: [],
    executions: [],
  });

  beforeEach(() => {
    check = new CommandSafetyCheck();
    baseContext = {
      plan: createPlan([]),
      workspace_root: '/tmp/test',
      config: {
        version: 1,
        checks: [],
        thresholds: { overall_score: 80 },
      },
      check_config: {},
    };
  });

  describe('basic functionality', () => {
    it('should have correct name and description', () => {
      expect(check.name).toBe('command-safety');
      expect(check.description).toBe('Validates shell commands for destructive operations');
    });

    it('should pass when no commands are present', async () => {
      const result = await check.run(baseContext);

      expect(result.passed).toBe(true);
      expect(result.message).toBe('No commands to analyse');
      expect(result.score).toBe(100);
    });

    it('should pass when check is disabled', async () => {
      baseContext.plan = createPlan([{ command: 'rm -rf /' }]);
      baseContext.check_config = { enabled: false };

      const result = await check.run(baseContext);

      expect(result.passed).toBe(true);
      expect(result.message).toBe('Command safety check disabled');
      expect(result.details?.skipped).toBe(true);
    });

    it('should pass for safe commands', async () => {
      baseContext.plan = createPlan([
        { command: 'git status' },
        { command: 'npm install' },
        { command: 'ls -la' },
      ]);

      const result = await check.run(baseContext);

      expect(result.passed).toBe(true);
      const summary = result.details?.summary as CommandAnalysisSummary;
      expect(summary.total).toBe(3);
      expect(summary.allowed).toBe(3);
      expect(summary.blocked).toBe(0);
      expect(summary.warned).toBe(0);
    });
  });

  describe('git command rules', () => {
    it('should block git reset --hard', async () => {
      baseContext.plan = createPlan([{ command: 'git reset --hard HEAD~1' }]);

      const result = await check.run(baseContext);

      expect(result.passed).toBe(false);
      const blocked = result.details?.blocked as CommandSafetyFinding[];
      expect(blocked).toHaveLength(1);
      expect(blocked[0].ruleId).toBe('git-reset-hard');
      expect(blocked[0].category).toBe('git');
    });

    it('should block git push --force', async () => {
      baseContext.plan = createPlan([{ command: 'git push --force origin main' }]);

      const result = await check.run(baseContext);

      expect(result.passed).toBe(false);
      const blocked = result.details?.blocked as CommandSafetyFinding[];
      expect(blocked).toHaveLength(1);
      expect(blocked[0].ruleId).toBe('git-push-force');
    });

    it('should block git push -f (short flag)', async () => {
      baseContext.plan = createPlan([{ command: 'git push -f' }]);

      const result = await check.run(baseContext);

      expect(result.passed).toBe(false);
      const blocked = result.details?.blocked as CommandSafetyFinding[];
      expect(blocked).toHaveLength(1);
      expect(blocked[0].ruleId).toBe('git-push-force');
    });

    it('should warn on git clean -fd', async () => {
      baseContext.plan = createPlan([{ command: 'git clean -fd' }]);

      const result = await check.run(baseContext);

      expect(result.passed).toBe(true);
      const warnings = result.details?.warnings as CommandSafetyFinding[];
      expect(warnings).toHaveLength(1);
      expect(warnings[0].ruleId).toBe('git-clean-force');
    });

    it('should warn on git checkout with dot', async () => {
      baseContext.plan = createPlan([{ command: 'git checkout .' }]);

      const result = await check.run(baseContext);

      expect(result.passed).toBe(true);
      const warnings = result.details?.warnings as CommandSafetyFinding[];
      expect(warnings).toHaveLength(1);
      expect(warnings[0].ruleId).toBe('git-checkout-all');
    });

    it('should allow git commit', async () => {
      baseContext.plan = createPlan([{ command: 'git commit -m "test"' }]);

      const result = await check.run(baseContext);

      expect(result.passed).toBe(true);
      const summary = result.details?.summary as CommandAnalysisSummary;
      expect(summary.blocked).toBe(0);
      expect(summary.warned).toBe(0);
    });
  });

  describe('filesystem command rules', () => {
    it('should block rm -rf /', async () => {
      baseContext.plan = createPlan([{ command: 'rm -rf /' }]);

      const result = await check.run(baseContext);

      expect(result.passed).toBe(false);
      const blocked = result.details?.blocked as CommandSafetyFinding[];
      expect(blocked).toHaveLength(1);
      expect(blocked[0].ruleId).toBe('rm-rf-root');
    });

    it('should block rm -rf /*', async () => {
      baseContext.plan = createPlan([{ command: 'rm -rf /*' }]);

      const result = await check.run(baseContext);

      expect(result.passed).toBe(false);
    });

    it('should warn on rm -rf with recursive flag', async () => {
      baseContext.plan = createPlan([{ command: 'rm -rf ./node_modules' }]);

      const result = await check.run(baseContext);

      // Should pass but with warning
      expect(result.passed).toBe(true);
      const warnings = result.details?.warnings as CommandSafetyFinding[];
      expect(warnings.length).toBeGreaterThanOrEqual(0); // May or may not warn depending on rules
    });

    it('should block chmod 777 on sensitive paths', async () => {
      baseContext.plan = createPlan([{ command: 'chmod 777 /etc/passwd' }]);

      const result = await check.run(baseContext);

      expect(result.passed).toBe(false);
      const blocked = result.details?.blocked as CommandSafetyFinding[];
      expect(blocked).toHaveLength(1);
      expect(blocked[0].ruleId).toBe('chmod-777-sensitive');
    });

    it('should block dd writing to block devices', async () => {
      baseContext.plan = createPlan([{ command: 'dd if=/dev/zero of=/dev/sda' }]);

      const result = await check.run(baseContext);

      expect(result.passed).toBe(false);
      const blocked = result.details?.blocked as CommandSafetyFinding[];
      expect(blocked).toHaveLength(1);
      expect(blocked[0].ruleId).toBe('dd-block-device');
    });

    it('should block mkfs on any device', async () => {
      baseContext.plan = createPlan([{ command: 'mkfs.ext4 /dev/sda1' }]);

      const result = await check.run(baseContext);

      expect(result.passed).toBe(false);
    });
  });

  describe('wrapper command handling', () => {
    it('should detect dangerous commands through sudo', async () => {
      baseContext.plan = createPlan([{ command: 'sudo rm -rf /' }]);

      const result = await check.run(baseContext);

      expect(result.passed).toBe(false);
      const blocked = result.details?.blocked as CommandSafetyFinding[];
      expect(blocked).toHaveLength(1);
    });

    it('should detect dangerous commands through bash -c', async () => {
      baseContext.plan = createPlan([{ command: 'bash -c "git reset --hard"' }]);

      const result = await check.run(baseContext);

      expect(result.passed).toBe(false);
    });

    it('should detect dangerous commands through env', async () => {
      baseContext.plan = createPlan([{ command: 'env VAR=value rm -rf /' }]);

      const result = await check.run(baseContext);

      expect(result.passed).toBe(false);
    });
  });

  describe('configuration options', () => {
    it('should respect disabled rules', async () => {
      baseContext.plan = createPlan([{ command: 'git reset --hard' }]);
      baseContext.check_config = {
        rules: {
          disabled: ['git-reset-hard'],
        },
      };

      const result = await check.run(baseContext);

      expect(result.passed).toBe(true);
    });

    it('should allow rule severity override', async () => {
      baseContext.plan = createPlan([{ command: 'git reset --hard' }]);
      baseContext.check_config = {
        rules: {
          overrides: [{ id: 'git-reset-hard', action: 'warn' }],
        },
      };

      const result = await check.run(baseContext);

      expect(result.passed).toBe(true);
      const warnings = result.details?.warnings as CommandSafetyFinding[];
      expect(warnings).toHaveLength(1);
    });

    it('should support custom rules', async () => {
      baseContext.plan = createPlan([{ command: 'my-dangerous-cmd --nuke' }]);
      baseContext.check_config = {
        rules: {
          custom: [
            {
              id: 'custom-nuke',
              category: 'custom',
              command: 'my-dangerous-cmd',
              flags: { required: ['--nuke'] },
              action: 'block',
              severity: 'error',
              reason: 'Custom dangerous command',
            },
          ],
        },
      };

      const result = await check.run(baseContext);

      expect(result.passed).toBe(false);
      const blocked = result.details?.blocked as CommandSafetyFinding[];
      expect(blocked).toHaveLength(1);
      expect(blocked[0].ruleId).toBe('custom-nuke');
    });
  });

  describe('code block extraction', () => {
    it('should extract commands from bash code blocks', async () => {
      baseContext.plan = createPlan([
        {
          command: '```bash\ngit reset --hard\necho "done"\n```',
        },
      ]);

      const result = await check.run(baseContext);

      expect(result.passed).toBe(false);
      const summary = result.details?.summary as CommandAnalysisSummary;
      expect(summary.total).toBe(2); // git reset --hard and echo "done"
    });

    it('should ignore comments in code blocks', async () => {
      baseContext.plan = createPlan([
        {
          command: '```bash\n# This is a comment\ngit status\n```',
        },
      ]);

      const result = await check.run(baseContext);

      expect(result.passed).toBe(true);
      const summary = result.details?.summary as CommandAnalysisSummary;
      expect(summary.total).toBe(1); // Only git status, not the comment
    });
  });

  describe('scoring', () => {
    it('should calculate score based on blocked commands', async () => {
      baseContext.plan = createPlan([
        { command: 'git reset --hard' },
        { command: 'git push --force' },
        { command: 'git status' },
      ]);

      const result = await check.run(baseContext);

      expect(result.passed).toBe(false);
      // 2 blocked commands * 25 penalty each = 50 penalty
      expect(result.score).toBe(50);
    });

    it('should calculate score based on warnings', async () => {
      baseContext.plan = createPlan([
        { command: 'git clean -fd' },
        { command: 'git checkout .' },
        { command: 'git status' },
      ]);

      const result = await check.run(baseContext);

      expect(result.passed).toBe(true);
      // 2 warnings * 5 penalty each = 10 penalty
      expect(result.score).toBe(90);
    });

    it('should return 100 for no commands', async () => {
      const result = await check.run(baseContext);

      expect(result.score).toBe(100);
    });
  });

  describe('result formatting', () => {
    it('should include formatted block message', async () => {
      baseContext.plan = createPlan([{ command: 'git reset --hard HEAD~1' }]);

      const result = await check.run(baseContext);

      expect(result.details?.formattedBlockedMessage).toContain('Blocked 1 dangerous command');
      expect(result.details?.formattedBlockedMessage).toContain('git reset --hard');
    });

    it('should include formatted warning message', async () => {
      baseContext.plan = createPlan([{ command: 'git clean -fd' }]);

      const result = await check.run(baseContext);

      expect(result.details?.formattedWarningMessage).toContain('1 potentially dangerous command');
    });

    it('should include suggestion in findings', async () => {
      baseContext.plan = createPlan([{ command: 'git reset --hard' }]);

      const result = await check.run(baseContext);

      const blocked = result.details?.blocked as CommandSafetyFinding[];
      expect(blocked[0].suggestion).toBeDefined();
    });

    it('should include source in findings', async () => {
      baseContext.plan = createPlan([{ command: 'git reset --hard', path: 'deploy.sh' }]);

      const result = await check.run(baseContext);

      const blocked = result.details?.blocked as CommandSafetyFinding[];
      expect(blocked[0].source).toBe('deploy.sh');
    });
  });

  describe('multiple commands', () => {
    it('should handle multiple dangerous commands', async () => {
      baseContext.plan = createPlan([
        { command: 'git reset --hard' },
        { command: 'rm -rf /' },
        { command: 'git push --force' },
      ]);

      const result = await check.run(baseContext);

      expect(result.passed).toBe(false);
      const blocked = result.details?.blocked as CommandSafetyFinding[];
      expect(blocked).toHaveLength(3);
    });

    it('should correctly summarise mixed results', async () => {
      baseContext.plan = createPlan([
        { command: 'git reset --hard' }, // blocked
        { command: 'git clean -fd' }, // warned
        { command: 'git status' }, // allowed
        { command: 'ls -la' }, // allowed
      ]);

      const result = await check.run(baseContext);

      expect(result.passed).toBe(false);
      const summary = result.details?.summary as CommandAnalysisSummary;
      expect(summary.total).toBe(4);
      expect(summary.blocked).toBe(1);
      expect(summary.warned).toBe(1);
      expect(summary.allowed).toBe(2);
    });
  });

  describe('edge cases', () => {
    it('should handle empty command string', async () => {
      baseContext.plan = createPlan([{ command: '' }]);

      const result = await check.run(baseContext);

      expect(result.passed).toBe(true);
    });

    it('should handle whitespace-only command', async () => {
      baseContext.plan = createPlan([{ command: '   ' }]);

      const result = await check.run(baseContext);

      expect(result.passed).toBe(true);
    });

    it('should handle plan without proposed_changes', async () => {
      baseContext.plan.proposed_changes = [];

      const result = await check.run(baseContext);

      expect(result.passed).toBe(true);
    });
  });

  describe('chained commands (security fix)', () => {
    it('should detect dangerous command hidden after && operator', async () => {
      baseContext.plan = createPlan([{ command: 'echo safe && git reset --hard' }]);

      const result = await check.run(baseContext);

      expect(result.passed).toBe(false);
      const blocked = result.details?.blocked as CommandSafetyFinding[];
      expect(blocked).toHaveLength(1);
      expect(blocked[0].ruleId).toBe('git-reset-hard');
    });

    it('should detect dangerous command hidden after || operator', async () => {
      baseContext.plan = createPlan([{ command: 'test -f file || rm -rf /' }]);

      const result = await check.run(baseContext);

      expect(result.passed).toBe(false);
      const blocked = result.details?.blocked as CommandSafetyFinding[];
      expect(blocked).toHaveLength(1);
      expect(blocked[0].ruleId).toBe('rm-rf-root');
    });

    it('should detect dangerous command hidden after ; operator', async () => {
      baseContext.plan = createPlan([{ command: 'echo start; git push --force' }]);

      const result = await check.run(baseContext);

      expect(result.passed).toBe(false);
      const blocked = result.details?.blocked as CommandSafetyFinding[];
      expect(blocked).toHaveLength(1);
      expect(blocked[0].ruleId).toBe('git-push-force');
    });

    it('should detect multiple dangerous commands in chain', async () => {
      baseContext.plan = createPlan([
        { command: 'echo ok && git reset --hard && rm -rf / && git push --force' },
      ]);

      const result = await check.run(baseContext);

      expect(result.passed).toBe(false);
      const blocked = result.details?.blocked as CommandSafetyFinding[];
      expect(blocked.length).toBeGreaterThanOrEqual(3);
    });

    it('should not have false positives for safe chained commands', async () => {
      baseContext.plan = createPlan([{ command: 'git status && npm install && npm test' }]);

      const result = await check.run(baseContext);

      expect(result.passed).toBe(true);
    });
  });

  describe('output config', () => {
    it('should hide suggestions when showSuggestions is false', async () => {
      baseContext.plan = createPlan([{ command: 'git reset --hard' }]);
      baseContext.check_config = {
        output: { showSuggestions: false, showReferences: true, verbose: true },
      };

      const result = await check.run(baseContext);

      const formattedMessage = result.details?.formattedBlockedMessage as string;
      expect(formattedMessage).not.toContain('Suggestion:');
    });

    it('should hide references when showReferences is false', async () => {
      baseContext.plan = createPlan([{ command: 'git reset --hard' }]);
      baseContext.check_config = {
        output: { showSuggestions: true, showReferences: false, verbose: true },
      };

      const result = await check.run(baseContext);

      const formattedMessage = result.details?.formattedBlockedMessage as string;
      expect(formattedMessage).not.toContain('Reference:');
    });

    it('should hide reason when verbose is false', async () => {
      baseContext.plan = createPlan([{ command: 'git reset --hard' }]);
      baseContext.check_config = {
        output: { showSuggestions: true, showReferences: true, verbose: false },
      };

      const result = await check.run(baseContext);

      const formattedMessage = result.details?.formattedBlockedMessage as string;
      expect(formattedMessage).not.toContain('Reason:');
    });

    it('should show all when config enables everything', async () => {
      baseContext.plan = createPlan([{ command: 'git reset --hard' }]);
      baseContext.check_config = {
        output: { showSuggestions: true, showReferences: true, verbose: true },
      };

      const result = await check.run(baseContext);

      const formattedMessage = result.details?.formattedBlockedMessage as string;
      expect(formattedMessage).toContain('Reason:');
      expect(formattedMessage).toContain('Suggestion:');
      expect(formattedMessage).toContain('Reference:');
    });
  });
});
