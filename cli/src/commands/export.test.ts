import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { createExportCommand } from './export.js';

describe('export command', () => {
  beforeEach(() => {
    vi.spyOn(process, 'exit').mockImplementation(() => undefined as never);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should create command with correct name and description', () => {
    const command = createExportCommand();

    expect(command.name()).toBe('export');
    expect(command.description()).toContain('Export/convert');
    expect(command.description()).toContain('SpecKit');
    expect(command.description()).toContain('APS');
  });

  it('should require source argument', () => {
    const command = createExportCommand();
    const args = command.registeredArguments;

    expect(args).toHaveLength(1);
    expect(args[0].name()).toBe('source');
    expect(args[0].required).toBe(true);
  });

  it('should have required --to option', () => {
    const command = createExportCommand();
    const toOpt = command.options.find((o) => o.long === '--to');

    expect(toOpt).toBeDefined();
    expect(toOpt?.mandatory).toBe(true);
  });

  it('should have optional --output option', () => {
    const command = createExportCommand();
    const outputOpt = command.options.find((o) => o.long === '--output');

    expect(outputOpt).toBeDefined();
    expect(outputOpt?.mandatory).toBe(false);
  });

  it('should have optional --from option for explicit format', () => {
    const command = createExportCommand();
    const fromOpt = command.options.find((o) => o.long === '--from');

    expect(fromOpt).toBeDefined();
    expect(fromOpt?.mandatory).toBe(false);
  });

  it('should have --compact option for JSON formatting', () => {
    const command = createExportCommand();
    const compactOpt = command.options.find((o) => o.long === '--compact');

    expect(compactOpt).toBeDefined();
    expect(compactOpt?.defaultValue).toBe(false);
  });
});
