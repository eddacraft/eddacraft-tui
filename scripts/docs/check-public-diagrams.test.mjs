import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { chmod, cp, mkdir, mkdtemp, readFile, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, join, resolve } from 'node:path';
import test from 'node:test';

import {
  annotateSvg,
  canonicalDrawioSource,
  loadContract,
  sha256,
  validatePublicDiagrams,
} from './lib/public-diagrams.mjs';

const REPO_ROOT = resolve(import.meta.dirname, '../..');
const FIXTURE_ROOT = resolve(import.meta.dirname, 'fixtures/public-diagrams');
const CONTRACT = await loadContract(REPO_ROOT);
function xmlAttribute(value) {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('"', '&quot;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;');
}

function rawSvg(source) {
  return `<svg xmlns="http://www.w3.org/2000/svg" width="400" height="120" content="${xmlAttribute(source)}"><rect width="400" height="120"/></svg>`;
}

function withCurrentExportHash(svg) {
  const content = svg.replace(/\s*<metadata\s+id="anvil-drawio-provenance"[^>]*\/>\s*/, '');
  return svg.replace(
    /data-export-sha256="[a-f0-9]{64}"/,
    `data-export-sha256="${sha256(content)}"`
  );
}

async function fixture() {
  const root = await mkdtemp(join(tmpdir(), 'anvil-public-diagrams-'));
  const publicFamilyRoot = join(root, 'docs/public/anvil');
  const familyRoot = join(publicFamilyRoot, 'assets/diagrams');
  await mkdir(familyRoot, { recursive: true });
  await cp(join(FIXTURE_ROOT, 'sample-flow.drawio'), join(familyRoot, 'sample-flow.drawio'));
  await cp(join(FIXTURE_ROOT, 'sample-flow.md'), join(familyRoot, 'sample-flow.md'));
  const source = await readFile(join(familyRoot, 'sample-flow.drawio'), 'utf8');
  const svg = annotateSvg({
    svg: rawSvg(source),
    source,
    sourceName: 'sample-flow.drawio',
    contract: CONTRACT,
  });
  await writeFile(join(familyRoot, 'sample-flow.svg'), svg, 'utf8');
  const contract = {
    ...CONTRACT,
    diagramDirectories: ['docs/public/anvil/assets/diagrams'],
    families: [{ name: 'anvil', root: 'docs/public/anvil' }],
  };
  return { root, familyRoot, publicFamilyRoot, contract };
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

async function installFakeDrawio(root, versionOutput = '31.1.8') {
  const fakeDrawio = join(root, 'fake-drawio-safety.mjs');
  await mkdir(join(root, 'scripts/docs'), { recursive: true });
  await writeFile(
    join(root, 'scripts/docs/public-diagrams.json'),
    JSON.stringify(CONTRACT),
    'utf8'
  );
  await writeFile(
    fakeDrawio,
    `#!/usr/bin/env node
import { readFileSync, writeFileSync } from 'node:fs';
const args = process.argv.slice(2);
if (args[0] === '--version') {
  process.stdout.write(${JSON.stringify(versionOutput)} + '\\n');
} else {
  const output = args[args.indexOf('--output') + 1];
  const source = readFileSync(args.at(-1), 'utf8');
  const embedded = source.replaceAll('&', '&amp;').replaceAll('"', '&quot;').replaceAll('<', '&lt;').replaceAll('>', '&gt;');
  writeFileSync(output, '<svg xmlns="http://www.w3.org/2000/svg" width="400" height="120" content="' + embedded + '"><rect width="400" height="120"/></svg>');
}
`,
    'utf8'
  );
  await chmod(fakeDrawio, 0o755);
  return fakeDrawio;
}

function runExport(root, fakeDrawio) {
  return spawnSync(
    process.execPath,
    [
      resolve(import.meta.dirname, 'export-public-diagram.mjs'),
      'docs/public/anvil/assets/diagrams/sample-flow.drawio',
      '--root',
      root,
      '--drawio-bin',
      fakeDrawio,
    ],
    { encoding: 'utf8' }
  );
}

test('export usage documents the supported repository root option', () => {
  const result = spawnSync(
    process.execPath,
    [resolve(import.meta.dirname, 'export-public-diagram.mjs')],
    { encoding: 'utf8' }
  );
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /--root <path>/);
});

async function runChecker(root, contract) {
  await mkdir(join(root, 'scripts/docs'), { recursive: true });
  await writeFile(
    join(root, 'scripts/docs/public-diagrams.json'),
    JSON.stringify(contract),
    'utf8'
  );
  const result = spawnSync(
    process.execPath,
    [resolve(import.meta.dirname, 'check-public-diagrams.mjs'), '--root', root, '--json'],
    { encoding: 'utf8' }
  );
  return { result, output: JSON.parse(result.stdout) };
}

test('valid mounted, paired, referenced and provenanced diagram passes', async () => {
  assert.deepEqual(await findingsFor(), []);
});

test('checker validates committed provenance without invoking Draw.io', async () => {
  const state = await fixture();
  try {
    assert.deepEqual(await validatePublicDiagrams(state.root, state.contract), []);
  } finally {
    await rm(state.root, { recursive: true, force: true });
  }
});

test('contract scopes governed directories without renderer-analysis schema', () => {
  assert.equal('productionRenderers' in CONTRACT, false);
  assert.equal('excludedRoots' in CONTRACT, false);
  assert.equal('excludedRenderers' in CONTRACT, false);
  assert.ok(CONTRACT.families.every((family) => !('renderer' in family)));
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

test('Draw.io canonicalisation is structural and attribute-order independent', () => {
  const left =
    '<mxfile anvil-title="Example" anvil-description="A useful diagram."><diagram name="Page-1" id="one"><mxGraphModel><root><mxCell id="0"/></root></mxGraphModel></diagram></mxfile>';
  const right =
    '<mxfile anvil-description="A useful diagram." anvil-title="Example">\n  <diagram id="one" name="Page-1">\n    <mxGraphModel><root><mxCell id="0" /></root></mxGraphModel>\n  </diagram>\n</mxfile>';
  assert.equal(canonicalDrawioSource(left), canonicalDrawioSource(right));
});

test('Draw.io XML parser fails closed on malformed roots, namespaces, and invalid pages', () => {
  for (const source of [
    '<mxfile><diagram></mxfile>',
    '<mxfile><diagram id="one"><mxGraphModel><root/></mxGraphModel></diagram></mxfile><mxfile/>',
    '<evil:mxfile xmlns:evil="urn:evil"><evil:diagram/></evil:mxfile>',
    '<mxfile><diagram id="one"/></mxfile>',
    '<mxfile><diagram id="one"><mxGraphModel/></diagram></mxfile>',
    '<mxfile><diagram id="one"><mxGraphModel><root/></mxGraphModel></diagram><diagram id="two">encoded</diagram></mxfile>',
  ]) {
    assert.throws(() => canonicalDrawioSource(source), /Draw\.io|mxfile|diagram|XML/i);
  }
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

test('embedded Draw.io source must canonically match the sibling source', async () => {
  const findings = await findingsFor(async ({ familyRoot }) => {
    const path = join(familyRoot, 'sample-flow.svg');
    const mismatched = '<mxfile><diagram id="different"/></mxfile>';
    await writeFile(
      path,
      (await readFile(path, 'utf8')).replace(
        / content="[^"]*"/,
        ` content="${xmlAttribute(mismatched)}"`
      ),
      'utf8'
    );
  });
  assert.ok(findings.some(({ code }) => code === 'embedded-source-mismatch'));
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

test('namespace-aware SVG safety fails closed on active XML and URL tricks', async (t) => {
  const attacks = [
    ['doctype', '<!DOCTYPE svg [<!ENTITY xxe SYSTEM "file:///etc/passwd">]>'],
    ['processing instruction', '<?xml-stylesheet href="https://example.invalid/a.css"?>'],
    ['custom entity', '<text>&attack;</text>'],
    ['malformed XML', '<g>'],
    ['namespaced element', '<evil:script xmlns:evil="urn:evil">alert(1)</evil:script>'],
    ['namespaced event attribute', '<rect evil:onload="alert(1)" xmlns:evil="urn:evil"/>'],
    ['spoofed default namespace', '<g xmlns="http://www.w3.org/1999/xhtml"><a href="#x"/></g>'],
    ['CSS import', '<style>@import "https://example.invalid/a.css";</style>'],
    ['CSS external URL', '<rect style="fill:url(https://example.invalid/a.svg#x)"/>'],
    ['encoded dangerous scheme', '<a href="&#x6a;avascript:alert(1)"><text>x</text></a>'],
    ['non-fragment reference', '<use href="data:image/svg+xml;base64,PHN2Zy8+"/>'],
    ...[
      'fill',
      'stroke',
      'filter',
      'clip-path',
      'mask',
      'marker',
      'marker-start',
      'marker-mid',
      'marker-end',
      'cursor',
    ].map((attribute) => [
      `external ${attribute} URL`,
      `<rect ${attribute}="url(https://example.invalid/shape.svg#x)"/>`,
    ]),
    ['ping attribute', '<a ping="https://example.invalid/audit"><text>x</text></a>'],
    ['external shape-inside URL', '<rect shape-inside="url(https://example.invalid/a.svg#x)"/>'],
    [
      'external shape-subtract URL',
      '<rect shape-subtract="url(https://example.invalid/a.svg#x)"/>',
    ],
    [
      'external arbitrary attribute URL',
      '<rect data-shape="url(https://example.invalid/a.svg#x)"/>',
    ],
  ];
  for (const [name, attack] of attacks) {
    await t.test(name, async () => {
      const findings = await findingsFor(async ({ familyRoot }) => {
        const path = join(familyRoot, 'sample-flow.svg');
        const svg = (await readFile(path, 'utf8')).replace('</svg>', `${attack}</svg>`);
        await writeFile(path, withCurrentExportHash(svg), 'utf8');
      });
      assert.ok(findings.some(({ code }) => code === 'unsafe-svg'));
    });
  }
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

test('explicit adjacent description association permits an empty Markdown alt', async () => {
  const findings = await findingsFor(async ({ familyRoot }) => {
    await writeFile(
      join(familyRoot, 'sample-flow.md'),
      '# Sample flow\n\n<!-- diagram-description: sample-flow.svg -->\n\nThe diagram shows a request moving from input to output.\n\n![](sample-flow.svg)\n',
      'utf8'
    );
  });
  assert.deepEqual(findings, []);
});

test('arbitrary adjacent prose does not authorise an empty alt', async () => {
  const findings = await findingsFor(async ({ familyRoot }) => {
    await writeFile(
      join(familyRoot, 'sample-flow.md'),
      '# Sample flow\n\nThe diagram shows a request moving from input to output.\n\n![](sample-flow.svg)\n',
      'utf8'
    );
  });
  assert.ok(findings.some(({ code }) => code === 'unreferenced-svg'));
});

test('weak alt text is not meaningful', async () => {
  const findings = await findingsFor(async ({ familyRoot }) => {
    await writeFile(
      join(familyRoot, 'sample-flow.md'),
      '# Sample flow\n\n![diagram](sample-flow.svg)\n',
      'utf8'
    );
  });
  assert.ok(findings.some(({ code }) => code === 'unreferenced-svg'));
});

test('image-like references inside code and comments are ignored', async () => {
  const findings = await findingsFor(async ({ familyRoot }) => {
    await writeFile(
      join(familyRoot, 'sample-flow.md'),
      '# Sample flow\n\n```md\n![Misleading alt text](sample-flow.svg)\n```\n\n<!-- ![Another misleading alt](sample-flow.svg) -->\n',
      'utf8'
    );
  });
  assert.ok(findings.some(({ code }) => code === 'unreferenced-svg'));
});

test('visible raw HTML image with meaningful alt is a real reference', async () => {
  const findings = await findingsFor(async ({ familyRoot }) => {
    await writeFile(
      join(familyRoot, 'sample-flow.md'),
      '<div><img src="sample-flow.svg" alt="Sample flow from input to output" /></div>\n',
      'utf8'
    );
  });
  assert.deepEqual(findings, []);
});

test('hidden or attribute-text raw HTML images do not count as references', async (t) => {
  for (const [name, html] of [
    [
      'hidden ancestor',
      '<div hidden><img src="sample-flow.svg" alt="Sample flow from input to output" /></div>',
    ],
    [
      'attribute text',
      '<div data-example="&lt;img src=&quot;sample-flow.svg&quot; alt=&quot;Sample flow from input to output&quot;&gt;">No visible image</div>',
    ],
  ]) {
    await t.test(name, async () => {
      const findings = await findingsFor(async ({ familyRoot }) => {
        await writeFile(join(familyRoot, 'sample-flow.mdx'), html, 'utf8');
        await writeFile(join(familyRoot, 'sample-flow.md'), '# Sample flow\n', 'utf8');
      });
      assert.ok(findings.some(({ code }) => code === 'unreferenced-svg'));
    });
  }
});

test('description association must name the adjacent SVG target', async () => {
  const findings = await findingsFor(async ({ familyRoot }) => {
    await writeFile(
      join(familyRoot, 'sample-flow.md'),
      '<!-- diagram-description: different.svg -->\n\nThe diagram shows a request moving from input to output.\n\n![](sample-flow.svg)\n',
      'utf8'
    );
  });
  assert.ok(findings.some(({ code }) => code === 'unreferenced-svg'));
});

test('non-lower-kebab diagram names fail', async () => {
  const findings = await findingsFor(async ({ familyRoot }) => {
    await cp(join(familyRoot, 'sample-flow.drawio'), join(familyRoot, 'Bad_Name.drawio'));
  });
  assert.ok(findings.some(({ code }) => code === 'invalid-name'));
});

test('diagram-like files outside explicit diagram directories are not governed', async () => {
  const findings = await findingsFor(async ({ familyRoot, publicFamilyRoot }) => {
    const source = await readFile(join(familyRoot, 'sample-flow.drawio'), 'utf8');
    await writeFile(join(publicFamilyRoot, 'legacy.drawio'), source, 'utf8');
    await writeFile(
      join(publicFamilyRoot, 'legacy.svg'),
      annotateSvg({
        svg: rawSvg(source),
        source,
        sourceName: 'legacy.drawio',
        contract: CONTRACT,
      }),
      'utf8'
    );
  });
  assert.deepEqual(findings, []);
});

test('governed diagram extensions must be lower-case', async () => {
  const findings = await findingsFor(async ({ familyRoot }) => {
    await cp(join(familyRoot, 'sample-flow.svg'), join(familyRoot, 'uppercase.SVG'));
  });
  assert.ok(findings.some(({ code }) => code === 'invalid-extension'));
});

test('checker summary counts governed files with invalid extension casing', async () => {
  const state = await fixture();
  try {
    await cp(join(state.familyRoot, 'sample-flow.svg'), join(state.familyRoot, 'uppercase.SVG'));
    const { result, output } = await runChecker(state.root, state.contract);
    assert.notEqual(result.status, 0);
    assert.equal(output.summary.filesChecked, 3);
  } finally {
    await rm(state.root, { recursive: true, force: true });
  }
});

test('raster sibling fails', async () => {
  const findings = await findingsFor(async ({ familyRoot }) => {
    await writeFile(join(familyRoot, 'sample-flow.png'), 'not a public export', 'utf8');
  });
  assert.ok(findings.some(({ code }) => code === 'raster-diagram'));
});

test('ordinary public screenshot is outside the governed diagram candidate set', async () => {
  const findings = await findingsFor(async ({ publicFamilyRoot }) => {
    await writeFile(join(publicFamilyRoot, 'installation-screenshot.png'), 'screenshot', 'utf8');
  });
  assert.deepEqual(findings, []);
});

test('ADR-123 reviewed raster consumer exception is accepted', async () => {
  const findings = await findingsFor(async ({ familyRoot, contract }) => {
    await writeFile(join(familyRoot, 'email-client.png'), 'raster', 'utf8');
    await writeFile(
      join(familyRoot, 'raster.md'),
      '![Transactional email client flow](email-client.png)\n',
      'utf8'
    );
    contract.rasterExceptions = [
      {
        path: 'docs/public/anvil/assets/diagrams/email-client.png',
        consumer: 'Transactional email client cannot render SVG',
        reviewedAgainst: 'ADR-123',
      },
    ];
  });
  assert.deepEqual(findings, []);
});

test('reviewed raster exception still requires an accessible real reference', async () => {
  const findings = await findingsFor(async ({ familyRoot, contract }) => {
    await writeFile(join(familyRoot, 'email-client.png'), 'raster', 'utf8');
    contract.rasterExceptions = [
      {
        path: 'docs/public/anvil/assets/diagrams/email-client.png',
        consumer: 'Transactional email client cannot render SVG',
        reviewedAgainst: 'ADR-123',
      },
    ];
  });
  assert.ok(findings.some(({ code }) => code === 'invalid-raster-exception'));
});

test('unreviewed raster diagram exception fails closed', async () => {
  const findings = await findingsFor(async ({ familyRoot, contract }) => {
    await writeFile(join(familyRoot, 'email-client.png'), 'raster', 'utf8');
    contract.rasterExceptions = [
      {
        path: 'docs/public/anvil/assets/diagrams/email-client.png',
        consumer: '',
        reviewedAgainst: 'ADR-122',
      },
    ];
  });
  assert.ok(
    findings.some(({ code }) => code === 'invalid-raster-exception' || code === 'raster-diagram')
  );
});

test('missing or duplicate explicit diagram directories fail closed', async () => {
  const findings = await findingsFor(async ({ contract }) => {
    contract.diagramDirectories = [
      'docs/public/anvil/assets/diagrams',
      'docs/public/anvil/assets/diagrams',
    ];
  });
  assert.ok(findings.some(({ code }) => code === 'invalid-contract'));
});

test('checker rejects an absolute manifest family root before traversal', async () => {
  const findings = await findingsFor(async ({ root, contract }) => {
    const familyRoot = join(root, 'missing-absolute-family');
    contract.families = [{ name: 'anvil', root: familyRoot }];
    contract.diagramDirectories = [`${familyRoot}/assets/diagrams`];
  });
  assert.ok(findings.some(({ code }) => code === 'invalid-contract'));
});

test('checker rejects a parent-traversing manifest diagram root before traversal', async () => {
  const findings = await findingsFor(async ({ root, contract }) => {
    const diagramRoot = `docs/public/anvil/../../../${basename(root)}-escape`;
    const escapedRoot = resolve(root, diagramRoot);
    await mkdir(escapedRoot);
    await symlink(join(root, 'external.drawio'), join(escapedRoot, 'tripwire.drawio'));
    contract.diagramDirectories = [diagramRoot];
  });
  assert.ok(findings.some(({ code }) => code === 'invalid-contract'));
  assert.ok(findings.every(({ code }) => code !== 'symlink-path'));
});

test('checker summary does not traverse an invalid manifest diagram root', async () => {
  const state = await fixture();
  const escapedRoot = join(resolve(state.root, '..'), `${basename(state.root)}-summary-escape`);
  try {
    await mkdir(escapedRoot);
    await writeFile(join(escapedRoot, 'outside.svg'), '<svg/>', 'utf8');
    state.contract.diagramDirectories = [`../${basename(escapedRoot)}`];
    const { result, output } = await runChecker(state.root, state.contract);
    assert.notEqual(result.status, 0);
    assert.equal(output.summary.filesChecked, 0);
  } finally {
    await rm(state.root, { recursive: true, force: true });
    await rm(escapedRoot, { recursive: true, force: true });
  }
});

test('checker rejects symlinked diagram entries instead of traversing them', async () => {
  const findings = await findingsFor(async ({ root, familyRoot }) => {
    const external = join(root, 'external.drawio');
    await cp(join(familyRoot, 'sample-flow.drawio'), external);
    await rm(join(familyRoot, 'sample-flow.drawio'));
    await symlink(external, join(familyRoot, 'sample-flow.drawio'));
  });
  assert.ok(findings.some(({ code }) => code === 'symlink-path'));
});

test('checker rejects a symlink in any ancestor from the repository root', async () => {
  const findings = await findingsFor(async ({ root }) => {
    const externalDocs = join(root, 'external-docs');
    await cp(join(root, 'docs'), externalDocs, { recursive: true });
    await rm(join(root, 'docs'), { recursive: true, force: true });
    await symlink(externalDocs, join(root, 'docs'));
  });
  assert.ok(findings.some(({ code, path }) => code === 'symlink-path' && path === 'docs'));
});

test('checker skips an unrelated family symlink without reporting or following it', async () => {
  const findings = await findingsFor(async ({ root, publicFamilyRoot }) => {
    const external = join(root, 'external-reference');
    await mkdir(external);
    await writeFile(join(external, 'decoy.md'), '![decoy](sample-flow.svg)\n', 'utf8');
    await symlink(external, join(publicFamilyRoot, 'unrelated-link'));
  });
  assert.deepEqual(findings, []);
});

test('export refuses an external symlink source', async () => {
  const { root, familyRoot } = await fixture();
  try {
    const fakeDrawio = await installFakeDrawio(root);
    const external = join(root, 'external.drawio');
    await cp(join(familyRoot, 'sample-flow.drawio'), external);
    await rm(join(familyRoot, 'sample-flow.drawio'));
    await symlink(external, join(familyRoot, 'sample-flow.drawio'));
    const result = runExport(root, fakeDrawio);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /symlink/i);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('export refuses a source outside an explicit diagram directory', async () => {
  const { root, familyRoot, publicFamilyRoot } = await fixture();
  try {
    const fakeDrawio = await installFakeDrawio(root);
    await cp(join(familyRoot, 'sample-flow.drawio'), join(publicFamilyRoot, 'legacy.drawio'));
    const result = spawnSync(
      process.execPath,
      [
        resolve(import.meta.dirname, 'export-public-diagram.mjs'),
        'docs/public/anvil/legacy.drawio',
        '--root',
        root,
        '--drawio-bin',
        fakeDrawio,
      ],
      { encoding: 'utf8' }
    );
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /explicit governed diagram directories/i);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('export refuses a symlinked intermediate family directory', async () => {
  const { root, familyRoot } = await fixture();
  try {
    const fakeDrawio = await installFakeDrawio(root);
    const externalFamily = join(root, 'external-family');
    await cp(familyRoot, externalFamily, { recursive: true });
    await rm(familyRoot, { recursive: true, force: true });
    await symlink(externalFamily, familyRoot);
    const result = runExport(root, fakeDrawio);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /symlink/i);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('export refuses an existing output symlink without changing its target', async () => {
  const { root, familyRoot } = await fixture();
  try {
    const fakeDrawio = await installFakeDrawio(root);
    const externalOutput = join(root, 'external.svg');
    await writeFile(externalOutput, 'unchanged', 'utf8');
    await rm(join(familyRoot, 'sample-flow.svg'));
    await symlink(externalOutput, join(familyRoot, 'sample-flow.svg'));
    const result = runExport(root, fakeDrawio);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /symlink/i);
    assert.equal(await readFile(externalOutput, 'utf8'), 'unchanged');
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('export requires exact unambiguous Desktop version output', async (t) => {
  for (const versionOutput of ['31.1.80', 'Draw.io 31.1.8', '31.1.8 31.1.9']) {
    await t.test(versionOutput, async () => {
      const { root } = await fixture();
      try {
        const fakeDrawio = await installFakeDrawio(root, versionOutput);
        const result = runExport(root, fakeDrawio);
        assert.notEqual(result.status, 0);
        assert.match(result.stderr, /exact version output/i);
      } finally {
        await rm(root, { recursive: true, force: true });
      }
    });
  }
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
import { appendFileSync, readFileSync, writeFileSync } from 'node:fs';
const args = process.argv.slice(2);
if (args[0] === '--version') {
  process.stdout.write('31.1.8\\n');
} else {
  appendFileSync(${JSON.stringify(callsPath)}, JSON.stringify(args) + '\\n');
  const output = args[args.indexOf('--output') + 1];
  const source = readFileSync(args.at(-1), 'utf8');
  const embedded = source.replaceAll('&', '&amp;').replaceAll('"', '&quot;').replaceAll('<', '&lt;').replaceAll('>', '&gt;');
  writeFileSync(output, '<svg xmlns="http://www.w3.org/2000/svg" content="' + embedded + '"><rect/></svg>');
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
        'docs/public/anvil/assets/diagrams/sample-flow.drawio',
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
    assert.match(
      await readFile(join(familyRoot, 'sample-flow.svg'), 'utf8'),
      /data-drawio-version-output="31\.1\.8"/
    );
    assert.match(
      await readFile(join(familyRoot, 'sample-flow.svg'), 'utf8'),
      /data-embedded-source-sha256="[a-f0-9]{64}"/
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
