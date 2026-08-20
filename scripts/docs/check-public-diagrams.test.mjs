import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { chmod, cp, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import test from 'node:test';

import { annotateSvg, loadContract, validatePublicDiagrams } from './lib/public-diagrams.mjs';

const REPO_ROOT = resolve(import.meta.dirname, '../..');
const FIXTURE_ROOT = resolve(import.meta.dirname, 'fixtures/public-diagrams');
const CONTRACT = await loadContract(REPO_ROOT);
const RAW_SVG =
  '<svg xmlns="http://www.w3.org/2000/svg" width="400" height="120" content="&lt;mxfile&gt;&lt;diagram/&gt;&lt;/mxfile&gt;"><rect width="400" height="120"/></svg>';

async function fixture() {
  const root = await mkdtemp(join(tmpdir(), 'anvil-public-diagrams-'));
  const familyRoot = join(root, 'docs/public/anvil');
  const renderer = join(root, 'apps/anvil-docs-private/docusaurus.config.ts');
  await mkdir(familyRoot, { recursive: true });
  await mkdir(dirname(renderer), { recursive: true });
  await cp(join(FIXTURE_ROOT, 'sample-flow.drawio'), join(familyRoot, 'sample-flow.drawio'));
  await cp(join(FIXTURE_ROOT, 'sample-flow.md'), join(familyRoot, 'sample-flow.md'));
  await writeFile(renderer, "path: '../../docs/public/anvil', routeBasePath: '/',\n", 'utf8');
  const source = await readFile(join(familyRoot, 'sample-flow.drawio'), 'utf8');
  const svg = annotateSvg({
    svg: RAW_SVG,
    source,
    sourceName: 'sample-flow.drawio',
    contract: CONTRACT,
  });
  await writeFile(join(familyRoot, 'sample-flow.svg'), svg, 'utf8');
  const contract = {
    ...CONTRACT,
    families: [
      {
        name: 'anvil',
        root: 'docs/public/anvil',
        renderer: 'apps/anvil-docs-private/docusaurus.config.ts',
      },
    ],
  };
  return { root, familyRoot, contract };
}

async function findingsFor(mutate = async () => {}) {
  const state = await fixture();
  try {
    await mutate(state);
    return await validatePublicDiagrams(state.root, state.contract);
  } finally {
    await rm(state.root, { recursive: true, force: true });
  }
}

test('valid mounted, paired, referenced and provenanced diagram passes', async () => {
  assert.deepEqual(await findingsFor(), []);
});

test('stale source provenance fails', async () => {
  const findings = await findingsFor(async ({ familyRoot }) => {
    await writeFile(join(familyRoot, 'sample-flow.drawio'), '<mxfile/>\n', 'utf8');
  });
  assert.ok(findings.some(({ code }) => code === 'stale-source'));
});

test('multi-page Draw.io source fails the single-SVG contract', async () => {
  const findings = await findingsFor(async ({ familyRoot }) => {
    const path = join(familyRoot, 'sample-flow.drawio');
    await writeFile(
      path,
      (await readFile(path, 'utf8')).replace(
        '</mxfile>',
        '<diagram id="second" name="Page-2"><mxGraphModel/></diagram></mxfile>'
      ),
      'utf8'
    );
  });
  assert.ok(findings.some(({ code }) => code === 'multi-page-source'));
});

test('changed SVG content fails deterministic provenance', async () => {
  const findings = await findingsFor(async ({ familyRoot }) => {
    const path = join(familyRoot, 'sample-flow.svg');
    await writeFile(path, (await readFile(path, 'utf8')).replace('<rect', '<circle'), 'utf8');
  });
  assert.ok(findings.some(({ code }) => code === 'stale-export'));
});

test('missing embedded source fails', async () => {
  const findings = await findingsFor(async ({ familyRoot }) => {
    const path = join(familyRoot, 'sample-flow.svg');
    await writeFile(path, (await readFile(path, 'utf8')).replace(/ content="[^"]*"/, ''), 'utf8');
  });
  assert.ok(findings.some(({ code }) => code === 'source-not-embedded'));
});

test('unsafe active SVG content fails', async () => {
  const findings = await findingsFor(async ({ familyRoot }) => {
    const path = join(familyRoot, 'sample-flow.svg');
    await writeFile(
      path,
      (await readFile(path, 'utf8')).replace('</svg>', '<script>alert(1)</script></svg>'),
      'utf8'
    );
  });
  assert.ok(findings.some(({ code }) => code === 'unsafe-svg'));
});

test('missing accessible description fails', async () => {
  const findings = await findingsFor(async ({ familyRoot }) => {
    const path = join(familyRoot, 'sample-flow.svg');
    await writeFile(
      path,
      (await readFile(path, 'utf8')).replace(/<desc[^>]*>.*?<\/desc>/s, ''),
      'utf8'
    );
  });
  assert.ok(findings.some(({ code }) => code === 'inaccessible-svg'));
});

test('unreferenced SVG fails', async () => {
  const findings = await findingsFor(async ({ familyRoot }) => {
    await writeFile(join(familyRoot, 'sample-flow.md'), '# Sample flow\n', 'utf8');
  });
  assert.ok(findings.some(({ code }) => code === 'unreferenced-svg'));
});

test('adjacent equivalent prose permits an empty Markdown alt', async () => {
  const findings = await findingsFor(async ({ familyRoot }) => {
    await writeFile(
      join(familyRoot, 'sample-flow.md'),
      '# Sample flow\n\nThe diagram shows a request moving from input to output.\n\n![](sample-flow.svg)\n',
      'utf8'
    );
  });
  assert.deepEqual(findings, []);
});

test('non-lower-kebab diagram names fail', async () => {
  const findings = await findingsFor(async ({ familyRoot }) => {
    await cp(join(familyRoot, 'sample-flow.drawio'), join(familyRoot, 'Bad_Name.drawio'));
  });
  assert.ok(findings.some(({ code }) => code === 'invalid-name'));
});

test('raster sibling fails', async () => {
  const findings = await findingsFor(async ({ familyRoot }) => {
    await writeFile(join(familyRoot, 'sample-flow.png'), 'not a public export', 'utf8');
  });
  assert.ok(findings.some(({ code }) => code === 'raster-diagram'));
});

test('declared family must remain mounted by its production renderer', async () => {
  const findings = await findingsFor(async ({ root }) => {
    await writeFile(
      join(root, 'apps/anvil-docs-private/docusaurus.config.ts'),
      'export default {};\n',
      'utf8'
    );
  });
  assert.ok(findings.some(({ code }) => code === 'family-not-mounted'));
});

test('export CLI enforces the pinned Desktop flags and writes the sibling SVG', async () => {
  const { root, familyRoot } = await fixture();
  const fakeDrawio = join(root, 'fake-drawio.mjs');
  const callsPath = join(root, 'drawio-calls.json');
  await mkdir(join(root, 'scripts/docs'), { recursive: true });
  await writeFile(
    join(root, 'scripts/docs/public-diagrams.json'),
    JSON.stringify(CONTRACT),
    'utf8'
  );
  await writeFile(
    fakeDrawio,
    `#!/usr/bin/env node
import { appendFileSync, writeFileSync } from 'node:fs';
const args = process.argv.slice(2);
if (args[0] === '--version') {
  process.stdout.write('31.1.8\\n');
} else {
  appendFileSync(${JSON.stringify(callsPath)}, JSON.stringify(args) + '\\n');
  const output = args[args.indexOf('--output') + 1];
  writeFileSync(output, '<svg xmlns="http://www.w3.org/2000/svg" content="&lt;mxfile&gt;&lt;diagram/&gt;&lt;/mxfile&gt;"><rect/></svg>');
}
`,
    'utf8'
  );
  await chmod(fakeDrawio, 0o755);
  try {
    const result = spawnSync(
      process.execPath,
      [
        resolve(import.meta.dirname, 'export-public-diagram.mjs'),
        'docs/public/anvil/sample-flow.drawio',
        '--root',
        root,
        '--drawio-bin',
        fakeDrawio,
      ],
      { encoding: 'utf8' }
    );
    assert.equal(result.status, 0, result.stderr);
    const calls = JSON.parse(`[${(await readFile(callsPath, 'utf8')).trim()}]`);
    assert.deepEqual(calls[0].slice(0, CONTRACT.exportArgs.length), CONTRACT.exportArgs);
    assert.match(
      await readFile(join(familyRoot, 'sample-flow.svg'), 'utf8'),
      /data-drawio-version="31\.1\.8"/
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
