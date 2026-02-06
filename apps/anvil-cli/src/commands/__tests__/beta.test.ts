import { describe, it, expect } from 'vitest';
import { createBetaCommand } from '../beta.js';

describe('beta command', () => {
  it('has invite and revoke subcommands', () => {
    const cmd = createBetaCommand();
    const subcommands = cmd.commands.map((c) => c.name());
    expect(subcommands).toContain('invite');
    expect(subcommands).toContain('revoke');
  });

  it('invite requires --email option', () => {
    const cmd = createBetaCommand();
    const invite = cmd.commands.find((c) => c.name() === 'invite');
    expect(invite).toBeDefined();
    const emailOption = invite!.options.find((o) => o.long === '--email');
    expect(emailOption).toBeDefined();
    expect(emailOption!.required).toBe(true);
  });

  it('revoke requires --email option', () => {
    const cmd = createBetaCommand();
    const revoke = cmd.commands.find((c) => c.name() === 'revoke');
    expect(revoke).toBeDefined();
    const emailOption = revoke!.options.find((o) => o.long === '--email');
    expect(emailOption).toBeDefined();
    expect(emailOption!.required).toBe(true);
  });
});
