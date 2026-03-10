import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { createExportCommand, normalizeTargetFormat } from './export.js';

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
    expect(command.description()).toContain('constraints');
  });

  it('should have optional source argument', () => {
    const command = createExportCommand();
    const args = command.registeredArguments;

    expect(args).toHaveLength(1);
    expect(args[0].name()).toBe('source');
    expect(args[0].required).toBe(false); // Optional for constraint export
  });

  it('should have --to option for plan conversion', () => {
    const command = createExportCommand();
    const toOpt = command.options.find((o) => o.long === '--to');

    expect(toOpt).toBeDefined();
    // Not mandatory - only needed for plan conversion, not constraint export
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

  it('should list yaml in --to help text', () => {
    const command = createExportCommand();
    const toOpt = command.options.find((o) => o.long === '--to');

    expect(toOpt?.description).toContain('yaml');
  });
});

describe('normalizeTargetFormat', () => {
  it('should normalise "yaml" to "yaml"', () => {
    expect(normalizeTargetFormat('yaml')).toBe('yaml');
  });

  it('should normalise "yml" to "yaml"', () => {
    expect(normalizeTargetFormat('yml')).toBe('yaml');
  });

  it('should normalise "YML" to "yaml" (case-insensitive)', () => {
    expect(normalizeTargetFormat('YML')).toBe('yaml');
  });

  it('should normalise "aps" to "aps"', () => {
    expect(normalizeTargetFormat('aps')).toBe('aps');
  });

  it('should normalise "json" to "json"', () => {
    expect(normalizeTargetFormat('json')).toBe('json');
  });

  it('should normalise "speckit" to "speckit"', () => {
    expect(normalizeTargetFormat('speckit')).toBe('speckit');
  });

  it('should normalise "spec.md" to "speckit"', () => {
    expect(normalizeTargetFormat('spec.md')).toBe('speckit');
  });

  it('should pass through unknown formats unchanged', () => {
    expect(normalizeTargetFormat('csv')).toBe('csv');
  });
});
