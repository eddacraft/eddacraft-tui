import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { promisify } from 'node:util';

import {
  checkDiagramImpact,
  classifyDiagramImpact,
  createMermaidRenderer,
  discoverDiagramDocuments,
  extractMermaidBlocks,
  matchesUpstream,
  runDiagramImpactCli,
  validateMermaidBlocks,
  verifyMermaidVersion,
} from './check-diagram-impact.mjs';

const execFileAsync = promisify(execFile);

const diagramDocument = {
  path: 'crates/example/ARCHITECTURE.md',
  upstreams: ['crates/example/src/**'],
  content: [
    '# Example architecture',
    '',
    '```mermaid',
    'flowchart LR',
    '  Input --> Output',
    '```',
    '',
  ].join('\n'),
};

test('extracts Mermaid fences with stable file-level locations', () => {
  assert.deepEqual(extractMermaidBlocks(diagramDocument.content, diagramDocument.path), [
    {
      path: diagramDocument.path,
      index: 1,
      line: 3,
      source: 'flowchart LR\n  Input --> Output',
    },
  ]);
});

test('matches declared files, directories and globs without escaping boundaries', () => {
  assert.equal(matchesUpstream('crates/example/src/lib.rs', 'crates/example/src/**'), true);
  assert.equal(matchesUpstream('crates/example/src/lib.rs', 'crates/example'), true);
  assert.equal(matchesUpstream('crates/example-other/src/lib.rs', 'crates/example'), false);
  assert.equal(matchesUpstream('../outside', 'crates/example/**'), false);
});

test('relevant declared-upstream changes fail when the owning diagram is untouched', () => {
  assert.deepEqual(
    classifyDiagramImpact({
      documents: [diagramDocument],
      changedPaths: ['crates/example/src/lib.rs'],
    }),
    [
      {
        code: 'diagram-review-owed',
        path: diagramDocument.path,
        upstream: 'crates/example/src/**',
      },
    ]
  );
});

test('updated owning diagram satisfies the relevant-change disposition', () => {
  assert.deepEqual(
    classifyDiagramImpact({
      documents: [diagramDocument],
      changedPaths: ['crates/example/src/lib.rs', diagramDocument.path],
    }),
    []
  );
});

test('irrelevant changes pass without a waiver or marker', () => {
  assert.deepEqual(
    classifyDiagramImpact({
      documents: [diagramDocument],
      changedPaths: ['crates/unrelated/src/lib.rs'],
    }),
    []
  );
});

test('validates Mermaid fences through the injected renderer with stable diagnostics', async () => {
  const rendered = [];
  assert.deepEqual(
    await validateMermaidBlocks({
      blocks: extractMermaidBlocks(diagramDocument.content, diagramDocument.path),
      render: async (block) => rendered.push(block.source),
    }),
    []
  );
  assert.deepEqual(rendered, ['flowchart LR\n  Input --> Output']);

  assert.deepEqual(
    await validateMermaidBlocks({
      blocks: extractMermaidBlocks(diagramDocument.content, diagramDocument.path),
      render: async () => {
        throw new Error('Parse error on line 2');
      },
    }),
    [
      {
        code: 'mermaid-render-failed',
        path: diagramDocument.path,
        index: 1,
        line: 3,
        message: 'Parse error on line 2',
      },
    ]
  );
});

test('discovers live governed Mermaid documents with declared upstreams', async () => {
  const root = await mkdtemp(join(tmpdir(), 'diagram-impact-'));
  const live = `# Live diagram

| Type     | Authority | Owner  | Status | Freshness                                |
| -------- | --------- | ------ | ------ | ---------------------------------------- |
| As-built | Derived   | DOCRB  | Live   | Last reviewed 2026-08-21 against \`abc1234\` |

| Upstream                   | Downstream |
| -------------------------- | ---------- |
| \`crates/example/src/**\` | none       |

\`\`\`mermaid
flowchart LR
  Input --> Output
\`\`\`
`;
  const archived = live.replace('| Live   |', '| Archived |');
  const component = live
    .replace('# Live diagram', '# Component architecture')
    .replace('| As-built | Derived', '| Architecture | Derived')
    .replace('crates/example/src/**', 'crates/component/src/**');
  const planSpec = live
    .replace('# Live diagram', '# Live plan specification')
    .replace('| As-built | Derived', '| Spec | Authoritative')
    .replace('| Live   |', '| Accepted |')
    .replace('crates/example/src/**', 'docs/guides/documentation-governance.md');
  const ignoredPlan = planSpec.replace('# Live plan specification', '# Non-spec plan');

  try {
    await mkdir(join(root, 'docs', 'architecture'), { recursive: true });
    await mkdir(join(root, 'crates', 'component'), { recursive: true });
    await mkdir(join(root, 'plans', 'specs'), { recursive: true });
    await mkdir(join(root, 'plans', 'modules'), { recursive: true });
    await writeFile(join(root, 'docs', 'architecture', 'live.md'), live);
    await writeFile(join(root, 'docs', 'architecture', 'archived.md'), archived);
    await writeFile(join(root, 'crates', 'component', 'ARCHITECTURE.md'), component);
    await writeFile(join(root, 'plans', 'specs', 'live.md'), planSpec);
    await writeFile(join(root, 'plans', 'modules', 'ignored.md'), ignoredPlan);

    assert.deepEqual(await discoverDiagramDocuments({ root }), [
      {
        path: 'crates/component/ARCHITECTURE.md',
        upstreams: ['crates/component/src/**'],
        content: component,
      },
      {
        path: 'docs/architecture/live.md',
        upstreams: ['crates/example/src/**'],
        content: live,
      },
      {
        path: 'plans/specs/live.md',
        upstreams: ['docs/guides/documentation-governance.md'],
        content: planSpec,
      },
    ]);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('checks only affected owners and requires updated Mermaid to render', async () => {
  const rendered = [];
  assert.deepEqual(
    await checkDiagramImpact({
      documents: [diagramDocument],
      changedPaths: ['crates/example/src/lib.rs', diagramDocument.path],
      render: async (block) => rendered.push(block.path),
    }),
    []
  );
  assert.deepEqual(rendered, [diagramDocument.path]);

  assert.deepEqual(
    await checkDiagramImpact({
      documents: [diagramDocument],
      changedPaths: [diagramDocument.path],
      render: async () => {
        throw new Error('malformed Mermaid');
      },
    }),
    [
      {
        code: 'mermaid-render-failed',
        path: diagramDocument.path,
        index: 1,
        line: 3,
        message: 'malformed Mermaid',
      },
    ]
  );

  rendered.length = 0;
  assert.deepEqual(
    await checkDiagramImpact({
      documents: [diagramDocument],
      changedPaths: ['crates/unrelated/src/lib.rs'],
      render: async (block) => rendered.push(block.path),
    }),
    []
  );
  assert.deepEqual(rendered, []);
});

test('runs the repository-local Mermaid CLI for one isolated fence', async () => {
  const calls = [];
  const render = createMermaidRenderer({
    root: '/repo',
    execute: async (command, args) => {
      const inputIndex = args.indexOf('--input');
      calls.push({ command, args, source: await readFile(args[inputIndex + 1], 'utf8') });
    },
  });

  await render({
    path: 'docs/architecture/example.md',
    index: 2,
    line: 17,
    source: 'flowchart LR\n  A --> B',
  });

  assert.equal(calls.length, 2);
  assert.match(calls[0].source, /SandboxProbeStart --> SandboxProbeEnd/u);
  assert.equal(calls[1].source, 'flowchart LR\n  A --> B\n');
  assert.equal(calls[1].command, '/repo/node_modules/.bin/mmdc');
  assert.deepEqual(calls[1].args.slice(0, 2), ['--input', calls[1].args[1]]);
  assert.deepEqual(calls[1].args.slice(2), ['--output', calls[1].args[3], '--quiet']);
});

test('verifies the exact repository-local Mermaid CLI version', async () => {
  const calls = [];
  const version = await verifyMermaidVersion({
    root: '/repo',
    execute: async (command, args) => {
      calls.push({ command, args });
      return { stdout: '11.16.0\n', stderr: '' };
    },
  });

  assert.equal(version, '11.16.0');
  assert.deepEqual(calls, [{ command: '/repo/node_modules/.bin/mmdc', args: ['--version'] }]);
});

test('fails closed when the Mermaid CLI version cannot be proved exactly', async () => {
  const invalidResults = [
    { stdout: '11.15.0\n', stderr: '' },
    { stdout: '11.16.0\n11.16.0\n', stderr: '' },
    { stdout: '11.16.0\n', stderr: 'warning\n' },
  ];

  for (const result of invalidResults) {
    await assert.rejects(
      verifyMermaidVersion({ root: '/repo', execute: async () => result }),
      /expected stdout-only 11\.16\.0/u
    );
  }

  await assert.rejects(
    verifyMermaidVersion({
      root: '/repo',
      execute: async () => {
        throw Object.assign(new Error('version command failed'), { stderr: 'launch failed\n' });
      },
    }),
    /Mermaid CLI version check failed: launch failed/u
  );
});

test('proves the renderer version before publishing the CLI summary', async () => {
  const root = await mkdtemp(join(tmpdir(), 'diagram-impact-cli-'));
  const calls = [];
  const originalWrite = process.stdout.write;
  let output = '';
  process.stdout.write = (chunk) => {
    output += String(chunk);
    return true;
  };

  try {
    const code = await runDiagramImpactCli(['--root', root, '--json'], {
      executeVersion: async (command, args) => {
        calls.push({ command, args });
        return { stdout: '11.16.0\n', stderr: '' };
      },
    });
    assert.equal(code, 0);
    assert.equal(JSON.parse(output).summary.rendererVersion, '11.16.0');
    assert.equal(JSON.parse(output).summary.rendererMode, 'not-probed');
    assert.deepEqual(calls, [
      { command: join(root, 'node_modules/.bin/mmdc'), args: ['--version'] },
    ]);
  } finally {
    process.stdout.write = originalWrite;
    await rm(root, { recursive: true, force: true });
  }
});

test('the real --since collector retains a deleted exact declared upstream', async () => {
  const root = await mkdtemp(join(tmpdir(), 'diagram-impact-deletion-'));
  const upstream = join(root, 'crates', 'example', 'src', 'lib.rs');
  const owner = join(root, 'docs', 'architecture', 'owner.md');
  const content = `# Deletion owner

| Type  | Authority     | Owner | Status | Freshness                                                        |
| ----- | ------------- | ----- | ------ | ---------------------------------------------------------------- |
| Guide | Authoritative | DOCRB | Live   | Last reviewed 2026-08-22 against \`crates/example/src/lib.rs\` |

| Upstream                        | Downstream |
| ------------------------------- | ---------- |
| \`crates/example/src/lib.rs\` | none       |

\`\`\`mermaid
flowchart LR
  Source --> Owner
\`\`\`
`;
  const originalWrite = process.stdout.write;
  let output = '';
  let rendered = 0;

  try {
    await mkdir(join(root, 'crates', 'example', 'src'), { recursive: true });
    await mkdir(join(root, 'docs', 'architecture'), { recursive: true });
    await writeFile(upstream, 'export const value = 1;\n');
    await writeFile(owner, content);
    await execFileAsync('git', ['init', '--quiet'], { cwd: root });
    await execFileAsync('git', ['add', '.'], { cwd: root });
    await execFileAsync(
      'git',
      [
        '-c',
        'user.name=DOCRB test',
        '-c',
        'user.email=docrb@example.invalid',
        'commit',
        '--quiet',
        '-m',
        'base',
      ],
      { cwd: root }
    );
    const base = (await execFileAsync('git', ['rev-parse', 'HEAD'], { cwd: root })).stdout.trim();
    await rm(upstream);
    await execFileAsync('git', ['add', '-A'], { cwd: root });
    await execFileAsync(
      'git',
      [
        '-c',
        'user.name=DOCRB test',
        '-c',
        'user.email=docrb@example.invalid',
        'commit',
        '--quiet',
        '-m',
        'delete upstream',
      ],
      { cwd: root }
    );

    process.stdout.write = (chunk) => {
      output += String(chunk);
      return true;
    };
    const code = await runDiagramImpactCli(['--root', root, '--since', base, '--json'], {
      executeVersion: async () => ({ stdout: '11.16.0\n', stderr: '' }),
      executeRenderer: async (_command, args) => {
        const inputIndex = args.indexOf('--input');
        const source = await readFile(args[inputIndex + 1], 'utf8');
        if (!source.includes('SandboxProbeStart')) rendered += 1;
      },
    });

    assert.equal(code, 1);
    assert.deepEqual(JSON.parse(output).findings, [
      {
        code: 'diagram-review-owed',
        path: 'docs/architecture/owner.md',
        upstream: 'crates/example/src/lib.rs',
      },
    ]);
    assert.equal(rendered, 1);
  } finally {
    process.stdout.write = originalWrite;
    await rm(root, { recursive: true, force: true });
  }
});

test('candidate text cannot authorise the no-sandbox fallback', async () => {
  const candidateErrors = [
    'prefix No usable sandbox!',
    'No usable sandbox! suffix',
    'Parse error near user label: No usable sandbox!',
  ];

  for (const candidateError of candidateErrors) {
    let candidateAttempts = 0;
    let fallbackAttempts = 0;
    const render = createMermaidRenderer({
      root: '/repo',
      execute: async (_command, args) => {
        const inputIndex = args.indexOf('--input');
        const source = await readFile(args[inputIndex + 1], 'utf8');
        if (source.includes('SandboxProbeStart')) return;

        candidateAttempts += 1;
        if (args.includes('--puppeteerConfigFile')) {
          fallbackAttempts += 1;
          return;
        }
        throw Object.assign(new Error('candidate parse failed'), { stderr: candidateError });
      },
    });

    await assert.rejects(
      render({
        path: 'docs/architecture/untrusted.md',
        index: 1,
        line: 3,
        source: 'flowchart LR\n  A[No usable sandbox!] --> B',
      }),
      new RegExp(candidateError.replace(/[.*+?^${}()|[\]\\]/gu, '\\$&'), 'u')
    );
    assert.equal(candidateAttempts, 1);
    assert.equal(fallbackAttempts, 0);
  }
});

test('trusted probe fails closed on near-match sandbox text', async () => {
  for (const probeError of ['prefix No usable sandbox!', 'No usable sandbox! suffix']) {
    let attempts = 0;
    const render = createMermaidRenderer({
      root: '/repo',
      execute: async () => {
        attempts += 1;
        throw Object.assign(new Error('probe failed'), { stderr: probeError });
      },
    });

    await assert.rejects(
      render({ path: 'example.md', index: 1, line: 1, source: 'flowchart LR' }),
      new RegExp(probeError, 'u')
    );
    assert.equal(attempts, 1);
  }
});

test('retries only the nested-sandbox launch failure with an isolated fallback config', async () => {
  let attempts = 0;
  let detectedMode;
  let fallbackConfig;
  const inputs = [];
  const render = createMermaidRenderer({
    root: '/repo',
    onMode: (mode) => {
      detectedMode = mode;
    },
    execute: async (_command, args) => {
      attempts += 1;
      const inputIndex = args.indexOf('--input');
      const input = args[inputIndex + 1];
      inputs.push(input);
      if (attempts === 1) {
        throw Object.assign(new Error('browser launch failed'), {
          stderr:
            '[0822/233202.459272:FATAL:content/browser/zygote_host/zygote_host_impl_linux.cc:129] No usable sandbox! If you are running on Ubuntu 23.10+ or another Linux distro that has disabled unprivileged user namespaces with AppArmor, see https://chromium.googlesource.com/chromium/src/+/main/docs/security/apparmor-userns-restrictions.md. Otherwise see https://chromium.googlesource.com/chromium/src/+/main/docs/linux/suid_sandbox_development.md for more information on developing with the (older) SUID sandbox. If you want to live dangerously and need an immediate workaround, you can try using --no-sandbox.',
        });
      }
      assert.doesNotMatch(await readFile(input, 'utf8'), /SandboxProbeStart/u);
      const configIndex = args.indexOf('--puppeteerConfigFile');
      assert.notEqual(configIndex, -1);
      fallbackConfig = JSON.parse(await readFile(args[configIndex + 1], 'utf8'));
    },
  });

  await render({ path: 'example.md', index: 1, line: 1, source: 'flowchart LR' });
  assert.equal(attempts, 2);
  assert.deepEqual(fallbackConfig, {
    args: ['--no-sandbox', '--disable-setuid-sandbox'],
  });
  assert.equal(detectedMode, 'conditional-no-sandbox-fallback');
  for (const input of inputs) {
    await assert.rejects(readFile(input, 'utf8'), { code: 'ENOENT' });
  }
});
