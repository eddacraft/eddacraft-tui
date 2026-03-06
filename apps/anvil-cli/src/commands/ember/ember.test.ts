import { describe, expect, it } from 'vitest';
import { createEmberCommand } from './index.js';
import { createEmberListCommand } from './list.js';
import { createEmberShowCommand } from './show.js';
import { createEmberPromoteCommand } from './promote.js';

describe('ember command', () => {
  it('createEmberCommand returns ember command', () => {
    const command = createEmberCommand();

    expect(command.name()).toBe('ember');
  });

  it('createEmberListCommand returns list command', () => {
    const command = createEmberListCommand();

    expect(command.name()).toBe('list');
  });

  it('createEmberShowCommand returns show command', () => {
    const command = createEmberShowCommand();

    expect(command.name()).toBe('show');
  });

  it('createEmberPromoteCommand returns promote command', () => {
    const command = createEmberPromoteCommand();

    expect(command.name()).toBe('promote');
  });

  it('parent ember command registers list, show, and promote subcommands', () => {
    const command = createEmberCommand();
    const subcommands = command.commands.map((entry) => entry.name());

    expect(subcommands).toEqual(expect.arrayContaining(['list', 'show', 'promote']));
    expect(subcommands).toHaveLength(3);
  });
});
