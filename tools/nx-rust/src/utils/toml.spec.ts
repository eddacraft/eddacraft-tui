import { describe, expect, it } from 'vitest';
import { parseCargoToml, stringifyCargoToml } from './toml';

describe('toml utilities', () => {
  it('round-trips a simple Cargo.toml', () => {
    const source = [
      '[package]',
      'name = "demo"',
      'version = "0.1.0"',
      'edition = "2021"',
      '',
      '[dependencies]',
      'serde = "1"',
      '',
    ].join('\n');
    const parsed = parseCargoToml(source);
    expect(parsed.package?.name).toBe('demo');
    expect(parsed.package?.version).toBe('0.1.0');

    // Serialise then re-parse — the exact quote style is a j-toml detail, but
    // the structure must survive the round-trip unchanged.
    const reparsed = parseCargoToml(stringifyCargoToml(parsed));
    expect(reparsed.package?.name).toBe('demo');
    expect(reparsed.package?.version).toBe('0.1.0');
    expect((reparsed as Record<string, unknown>).dependencies).toBeDefined();
  });

  it('applies a version bump via round-trip', () => {
    const source = '[package]\nname = "x"\nversion = "1.2.3"\n';
    const parsed = parseCargoToml(source);
    if (parsed.package) {
      parsed.package.version = '1.2.4';
    }
    const out = stringifyCargoToml(parsed);
    const reparsed = parseCargoToml(out);
    expect(reparsed.package?.version).toBe('1.2.4');
    expect(out).not.toContain('1.2.3');
  });
});
