/**
 * Stack Command Tests (STACK-013, STACK-014)
 *
 * Tests for the stack command and its subcommands.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { createStackCommand } from './stack.js';

describe('stack command', () => {
  beforeEach(() => {
    vi.spyOn(process, 'exit').mockImplementation(() => undefined as never);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should create command with correct name and description', () => {
    const command = createStackCommand();

    expect(command.name()).toBe('stack');
    expect(command.description()).toContain('Edda Stack');
  });

  it('should have status and validate subcommands', () => {
    const command = createStackCommand();
    const subcommandNames = command.commands.map((c) => c.name());

    expect(subcommandNames).toContain('status');
    expect(subcommandNames).toContain('validate');
  });

  describe('status subcommand', () => {
    it('should have status subcommand', () => {
      const command = createStackCommand();
      const statusCmd = command.commands.find((c) => c.name() === 'status');

      expect(statusCmd).toBeDefined();
      expect(statusCmd?.description()).toContain('status');
    });

    it('should have --json option', () => {
      const command = createStackCommand();
      const statusCmd = command.commands.find((c) => c.name() === 'status');
      const jsonOpt = statusCmd?.options.find((o) => o.long === '--json');

      expect(jsonOpt).toBeDefined();
    });
  });

  describe('validate subcommand', () => {
    it('should have validate subcommand', () => {
      const command = createStackCommand();
      const validateCmd = command.commands.find((c) => c.name() === 'validate');

      expect(validateCmd).toBeDefined();
      expect(validateCmd?.description()).toContain('Validate');
    });

    it('should have --json option', () => {
      const command = createStackCommand();
      const validateCmd = command.commands.find((c) => c.name() === 'validate');
      const jsonOpt = validateCmd?.options.find((o) => o.long === '--json');

      expect(jsonOpt).toBeDefined();
    });

    it('should have --fix option', () => {
      const command = createStackCommand();
      const validateCmd = command.commands.find((c) => c.name() === 'validate');
      const fixOpt = validateCmd?.options.find((o) => o.long === '--fix');

      expect(fixOpt).toBeDefined();
    });
  });
});
