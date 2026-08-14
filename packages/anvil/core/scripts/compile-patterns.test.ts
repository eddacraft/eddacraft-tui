/**
 * CLI-level behaviour of `scripts/compile-patterns.ts`.
 *
 * `compile.test.ts` covers the compiler library. These tests cover the thing
 * CI actually runs: the `--check` parity gate. The distinction matters —
 * CIB-335 was a gate that could never pass because the CLI treated the
 * compiler's (by-design, load-bearing) legacy `AP` prefix warnings as drift
 * and exited before it ever read the registry on disk.
 *
 * The CLI entry point is exercised through `runCompilePatternsCli`, which
 * returns an exit code and writes to injected streams, so the assertions are
 * on real CLI semantics (argument parsing, gate ordering, exit codes,
 * operator-facing messages) rather than on internals.
 */

import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import * as os from 'node:os';
import * as path from 'node:path';

import { afterAll, beforeAll, describe, expect, it } from 'vitest';

import { runCompilePatternsCli, type CompilePatternsIo } from './compile-patterns.js';

const DEFINITION = [
  '---',
  'id: fam-x',
  'type: definition',
  'name: Family X',
  'category: escape-hatch',
  'targets: [source]',
  '---',
  '',
  '## What It Is',
  'what',
  '',
  "## Why It's Harmful",
  'harm',
  '',
  '## The Spectrum',
  'spectrum',
  '',
  '## The Right Response',
  'fix',
  '',
  '## Detection Signals',
  'signals',
  '',
  '## Example',
  'example',
  '',
].join('\n');

const RULE = [
  '---',
  'id: FX-001',
  'type: rule',
  'family: fam-x',
  'title: t',
  'version: 1',
  'severity: warning',
  'confidence: high',
  'spectrum_position: 2',
  'targets: [source]',
  "detection: { type: regex, pattern: 'x' }",
  '---',
  '',
  'nudge body',
].join('\n');

/**
 * A definition that declares a rule with no matching file. The compiler
 * reports this as a *warning* and still produces a complete registry — the
 * same shape as the nine legacy `AP` prefix-collision warnings the real
 * `patterns/` tree emits on every run.
 */
const DEFINITION_WITH_WARNING = DEFINITION.replace(
  'targets: [source]',
  'targets: [source]\nrules: [FX-001, FX-404]'
);

interface CapturedIo extends CompilePatternsIo {
  out: string;
  err: string;
}

function makeIo(): CapturedIo {
  const io: CapturedIo = {
    out: '',
    err: '',
    stdout: (chunk: string) => {
      io.out += chunk;
    },
    stderr: (chunk: string) => {
      io.err += chunk;
    },
  };
  return io;
}

describe('compile-patterns CLI — --check parity gate', () => {
  let tmp: string;

  beforeAll(async () => {
    tmp = await mkdtemp(path.join(os.tmpdir(), 'anvil-compile-cli-'));
  });

  afterAll(async () => {
    await rm(tmp, { recursive: true, force: true });
  });

  async function writeAnvil(rel: string, content: string): Promise<void> {
    const abs = path.join(tmp, rel);
    await mkdir(path.dirname(abs), { recursive: true });
    await writeFile(abs, content, 'utf8');
  }

  /** Build a patterns tree that compiles cleanly but emits warnings. */
  async function warningTree(name: string): Promise<{ input: string; output: string }> {
    await writeAnvil(`${name}/patterns/fam-x/definition.anvil`, DEFINITION_WITH_WARNING);
    await writeAnvil(`${name}/patterns/fam-x/FX-001.anvil`, RULE);
    return {
      input: path.join(tmp, name, 'patterns'),
      output: path.join(tmp, name, 'compiled', 'registry.json'),
    };
  }

  it('exits 0 when the registry is in sync, even though the compile warns', async () => {
    const { input, output } = await warningTree('warns-in-sync');

    const writeIo = makeIo();
    const writeCode = await runCompilePatternsCli(
      ['--input', input, '--output', output],
      writeIo,
      tmp
    );
    expect(writeCode, writeIo.err).toBe(0);

    const io = makeIo();
    const code = await runCompilePatternsCli(
      ['--input', input, '--output', output, '--check'],
      io,
      tmp
    );

    // The warnings must still be reported — they are load-bearing information.
    expect(io.err).toMatch(/warnings:/);
    expect(io.err).toMatch(/FX-404/);
    // ...but on their own they are advisory, not drift.
    expect(code, `stderr:\n${io.err}`).toBe(0);
    expect(io.out).toMatch(/in sync/);
  });

  it('leaves the registry on disk untouched in --check mode', async () => {
    const { input, output } = await warningTree('warns-in-sync');
    const before = await readFile(output, 'utf8');

    const io = makeIo();
    await runCompilePatternsCli(['--input', input, '--output', output, '--check'], io, tmp);

    expect(await readFile(output, 'utf8')).toBe(before);
  });

  it('exits non-zero and names the drift when the committed registry is stale', async () => {
    const { input, output } = await warningTree('stale');
    await runCompilePatternsCli(['--input', input, '--output', output], makeIo(), tmp);

    // Simulate a `.anvil` edit that was never recompiled: the committed
    // registry claims `severity: error` for a rule the source calls `warning`.
    const committed = JSON.parse(await readFile(output, 'utf8')) as {
      patterns: { id: string; severity: string }[];
    };
    committed.patterns[0]!.severity = 'error';
    await writeFile(output, JSON.stringify(committed, null, 2) + '\n', 'utf8');

    const io = makeIo();
    const code = await runCompilePatternsCli(
      ['--input', input, '--output', output, '--check'],
      io,
      tmp
    );

    expect(code).toBe(1);
    expect(io.err).toMatch(/stale/);
    // The message must say WHAT drifted, not merely that something did.
    expect(io.err).toMatch(/pattern "FX-001" differs \(first change: severity\)/);
    expect(io.err).toMatch(/patterns:compile/);
  });

  it('exits non-zero and names the rule when the committed registry lost a pattern', async () => {
    const { input, output } = await warningTree('dropped');
    await runCompilePatternsCli(['--input', input, '--output', output], makeIo(), tmp);

    const committed = JSON.parse(await readFile(output, 'utf8')) as { patterns: unknown[] };
    committed.patterns = [];
    await writeFile(output, JSON.stringify(committed, null, 2) + '\n', 'utf8');

    const io = makeIo();
    const code = await runCompilePatternsCli(
      ['--input', input, '--output', output, '--check'],
      io,
      tmp
    );

    expect(code).toBe(1);
    expect(io.err).toMatch(/pattern "FX-001" is missing from the committed registry/);
  });

  /**
   * The committed `patterns/compiled/registry.json` is oxfmt-normalised while
   * the compiler emits fully expanded `JSON.stringify(..., 2)`. A text-level
   * comparison reports drift on a byte-for-byte-correct registry, so the gate
   * has to compare parsed JSON. `compiled_at` differs on every run for the
   * same reason.
   */
  it('does not report drift for a semantically identical but reformatted registry', async () => {
    const { input, output } = await warningTree('reformatted');
    await runCompilePatternsCli(['--input', input, '--output', output], makeIo(), tmp);

    const committed = JSON.parse(await readFile(output, 'utf8')) as Record<string, unknown>;
    committed.compiled_at = '1999-01-01T00:00:00.000Z';
    // Single-line, no trailing newline — the extreme of the oxfmt reflow.
    await writeFile(output, JSON.stringify(committed), 'utf8');

    const io = makeIo();
    const code = await runCompilePatternsCli(
      ['--input', input, '--output', output, '--check'],
      io,
      tmp
    );

    expect(code, `stderr:\n${io.err}`).toBe(0);
    expect(io.out).toMatch(/in sync/);
  });

  /**
   * Flagged twice by CLAWPATCH audits (2026-05-20, 2026-05-31): the previous
   * comparison stringified parsed objects, so re-serialising the registry with
   * the same values in a different key order reported drift. Harmless while
   * the gate never ran; a false red the moment it does.
   */
  it('does not report drift when object keys are in a different order', async () => {
    const { input, output } = await warningTree('reordered-keys');
    await runCompilePatternsCli(['--input', input, '--output', output], makeIo(), tmp);

    const committed = JSON.parse(await readFile(output, 'utf8')) as Record<string, unknown>;
    const reversed = Object.fromEntries(Object.entries(committed).toReversed());
    reversed.patterns = (committed.patterns as Record<string, unknown>[]).map((pattern) =>
      Object.fromEntries(Object.entries(pattern).toReversed())
    );
    await writeFile(output, JSON.stringify(reversed, null, 2) + '\n', 'utf8');

    const io = makeIo();
    const code = await runCompilePatternsCli(
      ['--input', input, '--output', output, '--check'],
      io,
      tmp
    );

    expect(code, `stderr:\n${io.err}`).toBe(0);
  });

  it('exits non-zero on compile errors', async () => {
    // A rule whose family has no definition.anvil — a hard compile error.
    await writeAnvil('broken/patterns/fam-x/FX-001.anvil', RULE);
    const input = path.join(tmp, 'broken', 'patterns');
    const output = path.join(tmp, 'broken', 'compiled', 'registry.json');

    const io = makeIo();
    const code = await runCompilePatternsCli(
      ['--input', input, '--output', output, '--check'],
      io,
      tmp
    );

    expect(code).toBe(1);
    expect(io.err).toMatch(/errors:/);
    expect(io.err).toMatch(/no definition\.anvil/);
    expect(io.err).toMatch(/compile failed/);
  });

  it('exits non-zero when the registry has never been generated', async () => {
    const { input } = await warningTree('never-generated');
    const output = path.join(tmp, 'never-generated', 'compiled', 'absent.json');

    const io = makeIo();
    const code = await runCompilePatternsCli(
      ['--input', input, '--output', output, '--check'],
      io,
      tmp
    );

    expect(code).toBe(1);
    expect(io.err).toMatch(/no compiled registry/);
  });

  it('escalates warnings to a failure only under --strict', async () => {
    const { input, output } = await warningTree('strict');
    await runCompilePatternsCli(['--input', input, '--output', output], makeIo(), tmp);

    const relaxed = makeIo();
    expect(
      await runCompilePatternsCli(['--input', input, '--output', output, '--check'], relaxed, tmp)
    ).toBe(0);

    const strict = makeIo();
    const code = await runCompilePatternsCli(
      ['--input', input, '--output', output, '--check', '--strict'],
      strict,
      tmp
    );

    expect(code).toBe(1);
    expect(strict.err).toMatch(/--strict: compile produced 1 warning\(s\)/);
  });
});

describe('compile-patterns CLI — real patterns/ tree', () => {
  /**
   * The regression that CIB-335 is about: the real tree emits nine by-design
   * legacy `AP` prefix-collision warnings, and the gate treated them as drift,
   * so `--check` failed identically whether the registry was in sync or badly
   * stale. This asserts the shipped registry actually matches its sources.
   */
  it('reports the checked-in registry as in sync despite the legacy AP warnings', async () => {
    const io = makeIo();
    const code = await runCompilePatternsCli(['--check'], io);

    expect(io.err).toMatch(/prefix "AP" spans multiple families/);
    expect(code, `stderr:\n${io.err}`).toBe(0);
    expect(io.out).toMatch(/in sync/);
  });
});
