import { describe, expect, it } from 'vitest';
import { createEddaCommand } from './index.js';
import { createEddaListCommand } from './list.js';
import { createEddaShowCommand } from './show.js';
import { createEddaPromoteCommand } from './promote.js';
import { createEddaRetireCommand } from './retire.js';
import { createEddaTraceCommand } from './trace.js';

describe('edda command', () => {
  it('createEddaCommand returns edda command', () => {
    const command = createEddaCommand();

    expect(command.name()).toBe('edda');
  });

  it('createEddaListCommand returns list command', () => {
    const command = createEddaListCommand();

    expect(command.name()).toBe('list');
  });

  it('createEddaShowCommand returns show command', () => {
    const command = createEddaShowCommand();

    expect(command.name()).toBe('show');
  });

  it('createEddaPromoteCommand returns promote command', () => {
    const command = createEddaPromoteCommand();

    expect(command.name()).toBe('promote');
  });

  it('createEddaRetireCommand returns retire command', () => {
    const command = createEddaRetireCommand();

    expect(command.name()).toBe('retire');
  });

  it('createEddaTraceCommand returns trace command', () => {
    const command = createEddaTraceCommand();

    expect(command.name()).toBe('trace');
  });

  it('parent edda command registers list, show, promote, retire, and trace subcommands', () => {
    const command = createEddaCommand();
    const subcommands = command.commands.map((entry) => entry.name());

    expect(subcommands).toEqual(
      expect.arrayContaining(['list', 'show', 'promote', 'retire', 'trace'])
    );
    expect(subcommands).toHaveLength(5);
  });
});
