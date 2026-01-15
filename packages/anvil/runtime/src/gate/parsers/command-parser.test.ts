import { describe, it, expect } from 'vitest';
import { parseCommand, parseCompoundCommand, CommandParser } from './command-parser.js';

describe('parseCommand', () => {
  describe('basic command parsing', () => {
    it('parses simple command', () => {
      const result = parseCommand('ls');
      expect(result.command).toBe('ls');
      expect(result.flags).toEqual([]);
      expect(result.args).toEqual([]);
    });

    it('parses command with flags', () => {
      const result = parseCommand('ls -la');
      expect(result.command).toBe('ls');
      expect(result.flags).toContain('-l');
      expect(result.flags).toContain('-a');
    });

    it('parses command with long flags', () => {
      const result = parseCommand('git push --force');
      expect(result.command).toBe('git');
      expect(result.subcommand).toBe('push');
      expect(result.flags).toContain('--force');
    });

    it('parses command with arguments', () => {
      const result = parseCommand('cp file1.txt file2.txt');
      expect(result.command).toBe('cp');
      expect(result.args).toContain('file1.txt');
      expect(result.args).toContain('file2.txt');
    });

    it('parses command with mixed flags and arguments', () => {
      const result = parseCommand('rm -rf /tmp/test');
      expect(result.command).toBe('rm');
      expect(result.flags).toContain('-r');
      expect(result.flags).toContain('-f');
      expect(result.args).toContain('/tmp/test');
    });
  });

  describe('git subcommand parsing', () => {
    it('extracts git subcommand', () => {
      const result = parseCommand('git reset --hard');
      expect(result.command).toBe('git');
      expect(result.subcommand).toBe('reset');
      expect(result.flags).toContain('--hard');
    });

    it('handles git checkout with branch', () => {
      const result = parseCommand('git checkout -b new-branch');
      expect(result.command).toBe('git');
      expect(result.subcommand).toBe('checkout');
      expect(result.flags).toContain('-b');
      expect(result.args).toContain('new-branch');
    });

    it('handles git push with remote and branch', () => {
      const result = parseCommand('git push origin main --force');
      expect(result.command).toBe('git');
      expect(result.subcommand).toBe('push');
      expect(result.args).toContain('origin');
      expect(result.args).toContain('main');
      expect(result.flags).toContain('--force');
    });

    it('handles git checkout --', () => {
      const result = parseCommand('git checkout -- file.txt');
      expect(result.command).toBe('git');
      expect(result.subcommand).toBe('checkout');
      expect(result.flags).toContain('--');
      expect(result.args).toContain('file.txt');
    });

    it('handles git stash drop', () => {
      const result = parseCommand('git stash drop stash@{0}');
      expect(result.command).toBe('git');
      expect(result.subcommand).toBe('stash');
      expect(result.args).toContain('drop');
    });
  });

  describe('combined flag expansion', () => {
    it('expands combined short flags', () => {
      const result = parseCommand('rm -rf file');
      expect(result.flags).toContain('-r');
      expect(result.flags).toContain('-f');
    });

    it('expands three combined flags', () => {
      const result = parseCommand('git clean -fdx');
      expect(result.flags).toContain('-f');
      expect(result.flags).toContain('-d');
      expect(result.flags).toContain('-x');
    });

    it('does not expand long flags', () => {
      const result = parseCommand('ls --all');
      expect(result.flags).toContain('--all');
      expect(result.flags).not.toContain('-a');
    });

    it('handles mix of short and long flags', () => {
      const result = parseCommand('git push -f --set-upstream origin main');
      expect(result.flags).toContain('-f');
      expect(result.flags).toContain('--set-upstream');
    });
  });

  describe('shell wrapper unwrapping', () => {
    it('unwraps bash -c', () => {
      const result = parseCommand('bash -c "git reset --hard"');
      expect(result.command).toBe('git');
      expect(result.subcommand).toBe('reset');
      expect(result.flags).toContain('--hard');
      expect(result.wrapperChain).toContain('bash');
    });

    it('unwraps sh -c', () => {
      const result = parseCommand('sh -c "rm -rf /tmp"');
      expect(result.command).toBe('rm');
      expect(result.flags).toContain('-r');
      expect(result.flags).toContain('-f');
      expect(result.wrapperChain).toContain('sh');
    });

    it('unwraps sudo', () => {
      const result = parseCommand('sudo rm -rf /var/log');
      expect(result.command).toBe('rm');
      expect(result.wrapperChain).toContain('sudo');
    });

    it('unwraps sudo with flags', () => {
      const result = parseCommand('sudo -u root git reset --hard');
      expect(result.command).toBe('git');
      expect(result.subcommand).toBe('reset');
      expect(result.wrapperChain).toContain('sudo');
    });

    it('unwraps env with variables', () => {
      const result = parseCommand('env VAR=value git push --force');
      expect(result.command).toBe('git');
      expect(result.subcommand).toBe('push');
      expect(result.wrapperChain).toContain('env');
    });

    it('unwraps command wrapper', () => {
      const result = parseCommand('command git reset --hard');
      expect(result.command).toBe('git');
      expect(result.subcommand).toBe('reset');
      expect(result.wrapperChain).toContain('command');
    });
  });

  describe('nested wrapper unwrapping', () => {
    it('unwraps sudo + bash -c', () => {
      const result = parseCommand('sudo bash -c "git reset --hard"');
      expect(result.command).toBe('git');
      expect(result.wrapperChain).toContain('sudo');
      expect(result.wrapperChain).toContain('bash');
    });

    it('unwraps env + sudo + command', () => {
      const result = parseCommand('env VAR=1 sudo rm -rf /tmp');
      expect(result.command).toBe('rm');
      expect(result.wrapperChain).toContain('env');
      expect(result.wrapperChain).toContain('sudo');
    });

    it('limits recursion depth', () => {
      const deeplyNested = 'bash -c "bash -c \\"bash -c \'bash -c git reset --hard\'\\"\\""';
      const result = parseCommand(deeplyNested);
      expect(result.wrapperChain.length).toBeLessThanOrEqual(5);
    });
  });

  describe('interpreter one-liner extraction', () => {
    it('extracts from python os.system', () => {
      const result = parseCommand('python -c "import os; os.system(\'git reset --hard\')"');
      expect(result.command).toBe('git');
      expect(result.subcommand).toBe('reset');
      expect(result.wrapperChain).toContain('python');
    });

    it('extracts from node exec', () => {
      const result = parseCommand("node -e \"require('child_process').exec('rm -rf /tmp')\"");
      expect(result.command).toBe('rm');
      expect(result.wrapperChain).toContain('node');
    });

    it('handles python subprocess.run', () => {
      const result = parseCommand(
        'python3 -c "import subprocess; subprocess.run(\'git push --force\')"'
      );
      expect(result.command).toBe('git');
      expect(result.wrapperChain).toContain('python3');
    });
  });

  describe('edge cases', () => {
    it('handles empty command', () => {
      const result = parseCommand('');
      expect(result.command).toBe('');
      expect(result.flags).toEqual([]);
      expect(result.args).toEqual([]);
    });

    it('handles whitespace only', () => {
      const result = parseCommand('   ');
      expect(result.command).toBe('');
    });

    it('handles quoted arguments', () => {
      const result = parseCommand('git commit -m "Initial commit"');
      expect(result.command).toBe('git');
      expect(result.subcommand).toBe('commit');
      expect(result.flags).toContain('-m');
      expect(result.args).toContain('Initial commit');
    });

    it('preserves raw command', () => {
      const cmd = 'sudo bash -c "git reset --hard"';
      const result = parseCommand(cmd);
      expect(result.raw).toBe(cmd);
    });

    it('handles paths with special characters', () => {
      const result = parseCommand('rm -rf "/path/with spaces/file.txt"');
      expect(result.command).toBe('rm');
      expect(result.args).toContain('/path/with spaces/file.txt');
    });

    it('handles environment variables in path', () => {
      const result = parseCommand('rm -rf $HOME/projects');
      expect(result.command).toBe('rm');
    });
  });
});

describe('parseCompoundCommand', () => {
  it('handles single command as non-compound', () => {
    const result = parseCompoundCommand('git status');
    expect(result.isCompound).toBe(false);
    expect(result.commands).toHaveLength(1);
    expect(result.commands[0].command).toBe('git');
  });

  it('splits commands on &&', () => {
    const result = parseCompoundCommand('echo ok && git reset --hard');
    expect(result.isCompound).toBe(true);
    expect(result.commands).toHaveLength(2);
    expect(result.commands[0].command).toBe('echo');
    expect(result.commands[1].command).toBe('git');
    expect(result.commands[1].subcommand).toBe('reset');
    expect(result.commands[1].flags).toContain('--hard');
  });

  it('splits commands on ||', () => {
    const result = parseCompoundCommand('test -f file || rm -rf /tmp');
    expect(result.isCompound).toBe(true);
    expect(result.commands).toHaveLength(2);
    expect(result.commands[0].command).toBe('test');
    expect(result.commands[1].command).toBe('rm');
  });

  it('splits commands on ;', () => {
    const result = parseCompoundCommand('echo start; git push --force; echo done');
    expect(result.isCompound).toBe(true);
    expect(result.commands).toHaveLength(3);
    expect(result.commands[0].command).toBe('echo');
    expect(result.commands[1].command).toBe('git');
    expect(result.commands[1].subcommand).toBe('push');
    expect(result.commands[2].command).toBe('echo');
  });

  it('splits commands on |', () => {
    const result = parseCompoundCommand('cat file | grep pattern | rm -rf /');
    expect(result.isCompound).toBe(true);
    expect(result.commands).toHaveLength(3);
    expect(result.commands[0].command).toBe('cat');
    expect(result.commands[1].command).toBe('grep');
    expect(result.commands[2].command).toBe('rm');
  });

  it('handles complex chained commands', () => {
    const result = parseCompoundCommand('echo safe && git status || git reset --hard');
    expect(result.isCompound).toBe(true);
    expect(result.commands).toHaveLength(3);
    expect(result.commands[2].command).toBe('git');
    expect(result.commands[2].subcommand).toBe('reset');
    expect(result.commands[2].flags).toContain('--hard');
  });

  it('returns operators', () => {
    const result = parseCompoundCommand('echo ok && git reset --hard');
    expect(result.operators.length).toBeGreaterThan(0);
  });
});

describe('CommandParser class', () => {
  const parser = new CommandParser();

  it('parses single command', () => {
    const result = parser.parse('git status');
    expect(result.command).toBe('git');
    expect(result.subcommand).toBe('status');
  });

  it('parses multiple commands', () => {
    const results = parser.parseMultiple(['git reset --hard', 'rm -rf /tmp', 'ls -la']);
    expect(results).toHaveLength(3);
    expect(results[0].command).toBe('git');
    expect(results[1].command).toBe('rm');
    expect(results[2].command).toBe('ls');
  });

  it('detects wrapped commands', () => {
    expect(parser.isWrapped('bash -c "ls"')).toBe(true);
    expect(parser.isWrapped('sudo rm -rf')).toBe(true);
    expect(parser.isWrapped('ls -la')).toBe(false);
  });

  it('gets wrapper chain', () => {
    const wrappers = parser.getWrappers('sudo bash -c "git reset"');
    expect(wrappers).toContain('sudo');
    expect(wrappers).toContain('bash');
  });

  it('detects compound commands', () => {
    expect(parser.isCompound('echo ok && git reset')).toBe(true);
    expect(parser.isCompound('git status')).toBe(false);
  });

  it('parseAllCommands returns all commands from compound', () => {
    const commands = parser.parseAllCommands('echo ok && git reset --hard');
    expect(commands).toHaveLength(2);
    expect(commands[0].command).toBe('echo');
    expect(commands[1].command).toBe('git');
    expect(commands[1].flags).toContain('--hard');
  });
});
