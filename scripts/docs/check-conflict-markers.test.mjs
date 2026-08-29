import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdir, mkdtemp, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import test from 'node:test';

const SCRIPT = resolve(import.meta.dirname, 'check-conflict-markers.mjs');

/** Build the three marker lines without ever writing a literal run of the
 *  marker characters into this file — otherwise the check would flag its own
 *  test source the moment anyone points it at a wider corpus. */
const OURS = '<'.repeat(7);
const BASE = '|'.repeat(7);
const THEIRS = '>'.repeat(7);
const SEPARATOR = '='.repeat(7);

async function makeRepo(files) {
  const root = await mkdtemp(join(tmpdir(), 'cm-'));
  for (const [rel, body] of Object.entries(files)) {
    const abs = join(root, rel);
    await mkdir(resolve(abs, '..'), { recursive: true });
    await writeFile(abs, body, 'utf8');
  }
  return root;
}

function run(root, extra = []) {
  return spawnSync(process.execPath, [SCRIPT, '--root', root, ...extra], { encoding: 'utf8' });
}

test('clean corpus passes', async () => {
  const root = await makeRepo({ 'docs/a.md': '# Title\n\nSome prose.\n' });
  try {
    const r = run(root);
    assert.equal(r.status, 0, r.stdout + r.stderr);
    assert.match(r.stdout, /\[conflict-markers\] ok/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('flags the diff3 base marker alone — the case that reached main', async () => {
  // The real defect: `<<<<<<<` and `>>>>>>>` had been cleaned up and only the
  // diff3 base marker survived, because in a Markdown table a run of pipes
  // reads as ordinary table syntax.
  const root = await makeRepo({
    'plans/index.aps.md': `| Module | Progress |\n| --- | --- |\n| CIB | 288/365 |\n${BASE} parent of abc1234 (some subject)\n| CIB | 288/357 |\n`,
  });
  try {
    const r = run(root);
    assert.equal(r.status, 1, r.stdout + r.stderr);
    assert.match(r.stderr, /plans\/index\.aps\.md:4/);
    assert.match(r.stderr, /conflict marker/i);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('flags ours and theirs markers', async () => {
  const root = await makeRepo({
    'docs/a.md': `# T\n\n${OURS} HEAD\nmine\n${THEIRS} feature-branch\n`,
  });
  try {
    const r = run(root);
    assert.equal(r.status, 1, r.stdout + r.stderr);
    assert.match(r.stderr, /docs\/a\.md:3/);
    assert.match(r.stderr, /docs\/a\.md:5/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('does NOT flag a setext H1 underline', async () => {
  // A run of `=` under a line is a valid Markdown H1. Git never emits a bare
  // separator without one of the other three markers, so flagging it would
  // only ever produce false positives.
  const root = await makeRepo({ 'docs/a.md': `Title\n${SEPARATOR}\n\nProse.\n` });
  try {
    const r = run(root);
    assert.equal(r.status, 0, r.stdout + r.stderr);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('does NOT flag longer runs or mid-line text', async () => {
  const root = await makeRepo({
    'docs/a.md': `# T\n\n${'<'.repeat(9)} decorative\ntext ${BASE} inline\n`,
  });
  try {
    const r = run(root);
    assert.equal(r.status, 0, r.stdout + r.stderr);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('flags markers inside fenced code blocks', async () => {
  // Deliberate: a conflict landing inside a doc's code fence is exactly as
  // broken as one in prose, so fences are not a blind spot.
  const root = await makeRepo({
    'docs/a.md': `# T\n\n\`\`\`text\n${OURS} HEAD\n\`\`\`\n`,
  });
  try {
    const r = run(root);
    assert.equal(r.status, 1, r.stdout + r.stderr);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('honours the documented per-file opt-out', async () => {
  const root = await makeRepo({
    'docs/resolving-conflicts.md': `<!-- docs-check: allow-conflict-markers -->\n\n# Resolving conflicts\n\n\`\`\`text\n${OURS} HEAD\n${SEPARATOR}\n${THEIRS} other\n\`\`\`\n`,
  });
  try {
    const r = run(root);
    assert.equal(r.status, 0, r.stdout + r.stderr);
    assert.match(r.stdout, /1 file\(s\) opted out/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('reports every occurrence, not just the first', async () => {
  const root = await makeRepo({
    'docs/a.md': `${OURS} HEAD\n`,
    'docs/b.md': `${THEIRS} other\n`,
  });
  try {
    const r = run(root);
    assert.equal(r.status, 1, r.stdout + r.stderr);
    assert.match(r.stderr, /docs\/a\.md:1/);
    assert.match(r.stderr, /docs\/b\.md:1/);
    assert.match(r.stderr, /2 conflict marker/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('an unreadable root is a tooling failure (exit 2), not a content failure', async () => {
  const r = run(join(tmpdir(), 'cm-does-not-exist-' + process.pid));
  assert.equal(r.status, 2, r.stdout + r.stderr);
});

test('scans non-Markdown text files too when asked', async () => {
  const root = await makeRepo({ 'src/a.rs': `fn main() {}\n${OURS} HEAD\n` });
  try {
    assert.equal(run(root).status, 0, 'markdown-only by default');
    const r = run(root, ['--all-text']);
    assert.equal(r.status, 1, r.stdout + r.stderr);
    assert.match(r.stderr, /src\/a\.rs:2/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('non-git fallback walks nested directories and reports forward-slash paths', async () => {
  // The fallback must not shell out to POSIX `find` (absent on Windows) and
  // must emit `/` separators so its paths match Git's, on every platform.
  const root = await makeRepo({
    'docs/deep/nested/a.md': `# T\n\n${OURS} HEAD\n`,
    'docs/keep.md': '# clean\n',
  });
  try {
    const r = run(root);
    assert.equal(r.status, 1, r.stdout + r.stderr);
    assert.match(r.stderr, /docs\/deep\/nested\/a\.md:3/);
    assert.ok(!r.stderr.includes('\\'), `path separators must be forward slashes: ${r.stderr}`);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('non-git fallback skips a .git directory', async () => {
  const root = await makeRepo({
    'docs/a.md': '# clean\n',
    '.git/HEAD.md': `${OURS} not a real doc\n`,
  });
  try {
    const r = run(root);
    assert.equal(r.status, 0, r.stdout + r.stderr);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('a bare mention of the opt-out phrase does not exempt a file', async () => {
  // Guards against self-exemption: this script and its tests both mention the
  // opt-out phrase, and matching it outside an HTML comment let them skip
  // themselves under --all-text.
  const root = await makeRepo({
    'docs/a.md': `# T\n\nSet "docs-check: allow-conflict-markers" to opt out.\n\n${OURS} HEAD\n`,
  });
  try {
    const r = run(root);
    assert.equal(r.status, 1, r.stdout + r.stderr);
    assert.match(r.stderr, /docs\/a\.md:5/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('explicit positional paths scan only those files', async () => {
  const root = await makeRepo({
    'docs/dirty.md': `${OURS} HEAD\n`,
    'docs/clean.md': '# clean\n',
  });
  try {
    const clean = run(root, ['docs/clean.md']);
    assert.equal(clean.status, 0, clean.stdout + clean.stderr);
    const dirty = run(root, ['docs/dirty.md']);
    assert.equal(dirty.status, 1, dirty.stdout + dirty.stderr);
    assert.match(dirty.stderr, /docs\/dirty\.md:1/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('explicit positional paths outside the repository are a tooling failure', async () => {
  const root = await makeRepo({ 'docs/clean.md': '# clean\n' });
  const outsideRoot = await makeRepo({
    'outside.md': `${OURS} HEAD\n`,
  });
  try {
    const outside = join(outsideRoot, 'outside.md');
    const r = run(root, [outside]);
    assert.equal(r.status, 2, r.stdout + r.stderr);
    assert.match(r.stderr, /outside the repository root/i);
    assert.ok(!r.stderr.includes(outside), 'must not echo the escaped absolute path');
  } finally {
    await rm(root, { recursive: true, force: true });
    await rm(outsideRoot, { recursive: true, force: true });
  }
});

test(
  'explicit positional file symlinks cannot escape the repository',
  { skip: process.platform === 'win32' },
  async () => {
    const root = await makeRepo({ 'docs/clean.md': '# clean\n' });
    const outsideRoot = await makeRepo({ 'outside.md': `${OURS} EXTERNAL-PROBE\n` });
    try {
      await symlink(join(outsideRoot, 'outside.md'), join(root, 'docs', 'linked.md'));
      const r = run(root, ['docs/linked.md']);
      assert.equal(r.status, 2, r.stdout + r.stderr);
      assert.match(r.stderr, /outside the repository root/i);
      assert.ok(!r.stderr.includes('EXTERNAL-PROBE'), 'must not read external content');
      assert.ok(!r.stderr.includes(outsideRoot), 'must not echo the escaped target');
    } finally {
      await rm(root, { recursive: true, force: true });
      await rm(outsideRoot, { recursive: true, force: true });
    }
  }
);

test(
  'explicit positional symlinked ancestors cannot escape the repository',
  { skip: process.platform === 'win32' },
  async () => {
    const root = await makeRepo({ 'placeholder.md': '# clean\n' });
    const outsideRoot = await makeRepo({ 'a.md': `${BASE} EXTERNAL-PROBE\n` });
    try {
      await symlink(outsideRoot, join(root, 'docs'), 'dir');
      const r = run(root, ['docs/a.md']);
      assert.equal(r.status, 2, r.stdout + r.stderr);
      assert.match(r.stderr, /outside the repository root/i);
      assert.ok(!r.stderr.includes('EXTERNAL-PROBE'), 'must not read external content');
      assert.ok(!r.stderr.includes(outsideRoot), 'must not echo the escaped target');
    } finally {
      await rm(root, { recursive: true, force: true });
      await rm(outsideRoot, { recursive: true, force: true });
    }
  }
);

test('explicit absolute paths report repo-relative forward-slash paths', async () => {
  const root = await makeRepo({ 'plans/index.aps.md': `| a |\n${BASE} parent of abc (s)\n` });
  try {
    const r = run(root, [join(root, 'plans', 'index.aps.md')]);
    assert.equal(r.status, 1, r.stdout + r.stderr);
    assert.match(r.stderr, /plans\/index\.aps\.md:2/);
    assert.ok(!r.stderr.includes(root), 'must not echo the absolute root');
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
